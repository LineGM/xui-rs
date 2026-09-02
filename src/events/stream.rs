use std::{
    borrow::Cow,
    fmt,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use futures_util::{SinkExt, Stream, StreamExt, stream::FusedStream};
use reqwest::{Method, StatusCode};
use rustls::{
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    time::timeout,
};
use tokio_rustls::TlsConnector;
use tokio_socks::{TargetAddr, tcp::Socks5Stream};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, client_async_tls_with_config,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        protocol::{WebSocketConfig, frame::CloseFrame, frame::coding::CloseCode},
    },
};
use url::{Host, Url};

use super::PanelEvent;
use crate::{Client, Error, ProxyConfig, ProxyError, ProxyScheme, Result};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;
const CLIENT_CLOSE_REASON: &str = "xui-rs disconnect";
const MAX_CONNECT_RESPONSE_SIZE: usize = 16 * 1024;

trait AsyncIo: AsyncRead + AsyncWrite {}

impl<T> AsyncIo for T where T: AsyncRead + AsyncWrite + ?Sized {}

type BoxedIo = Box<dyn AsyncIo + Send + Unpin>;
type Socket = WebSocketStream<MaybeTlsStream<BoxedIo>>;

/// Accessor for the panel's authenticated WebSocket endpoint.
#[derive(Clone, Copy, Debug)]
pub struct EventsApi<'client> {
    client: &'client Client,
}

impl<'client> EventsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Opens one authenticated real-time stream using the client's current
    /// cookie session.
    ///
    /// 3x-ui v3.7.0 deliberately ignores API bearer tokens on `/ws`. Call
    /// [`crate::AuthApi::login`] first; the same standards-compliant cookie jar
    /// is shared by HTTP requests and this handshake.
    ///
    /// # Errors
    ///
    /// Returns an authentication, timeout, TLS, transport, or WebSocket
    /// handshake error. HTTP 401/403 handshake responses map to the SDK's
    /// standard [`Error::Unauthorized`] and [`Error::Forbidden`] variants.
    pub async fn connect(self) -> Result<EventStream> {
        EventStream::connect(self.client.clone()).await
    }
}

/// Close information supplied by the peer or by an explicit local close.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebSocketClose {
    /// RFC 6455 close status code.
    pub code: u16,
    /// Human-readable close reason.
    pub reason: String,
}

/// One authenticated WebSocket connection yielding typed [`PanelEvent`]s.
///
/// The type implements [`Stream`]. Control frames are handled internally and
/// never surface as events. 3x-ui does not accept application commands from
/// clients, so the SDK intentionally exposes no arbitrary `send` method.
pub struct EventStream {
    client: Client,
    endpoint: Url,
    socket: Option<Socket>,
    close: Option<WebSocketClose>,
    terminated: bool,
}

impl EventStream {
    async fn connect(client: Client) -> Result<Self> {
        let (endpoint, socket) = connect_socket(&client).await?;
        Ok(Self {
            client,
            endpoint,
            socket: Some(socket),
            close: None,
            terminated: false,
        })
    }

    /// Receives the next typed event.
    ///
    /// Returns `Ok(None)` after a normal close handshake. An abrupt transport
    /// loss is returned as an error because events may have been missed.
    ///
    /// # Errors
    ///
    /// Returns an error for transport/protocol failures, binary data frames,
    /// oversized messages, or JSON that does not match the event's v3.7.0
    /// payload contract.
    pub async fn next_event(&mut self) -> Result<Option<PanelEvent>> {
        self.next().await.transpose()
    }

    /// Returns the latest close frame, when one was observed or sent.
    pub const fn close_info(&self) -> Option<&WebSocketClose> {
        self.close.as_ref()
    }

    /// Returns whether the current connection can yield more events.
    pub const fn is_closed(&self) -> bool {
        self.terminated
    }

    /// Reopens the stream after a close or error using the client's current
    /// cookie session.
    ///
    /// Events are not replayed by 3x-ui. Refresh relevant HTTP snapshots before
    /// reconnecting whenever the old connection ended unexpectedly.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] if the current connection is still
    /// active, or the same errors as [`EventsApi::connect`] for the new
    /// handshake.
    pub async fn reconnect(&mut self) -> Result<()> {
        if !self.terminated {
            return Err(Error::Configuration(
                "cannot reconnect an active WebSocket event stream".to_owned(),
            ));
        }
        let (endpoint, socket) = connect_socket(&self.client).await?;
        self.endpoint = endpoint;
        self.socket = Some(socket);
        self.close = None;
        self.terminated = false;
        Ok(())
    }

    /// Sends a normal close frame and terminates this connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the close frame cannot be written. Calling this on
    /// an already closed stream succeeds.
    pub async fn close(&mut self) -> Result<()> {
        let Some(mut socket) = self.socket.take() else {
            self.terminated = true;
            return Ok(());
        };
        let close = WebSocketClose {
            code: 1000,
            reason: CLIENT_CLOSE_REASON.to_owned(),
        };
        let frame = CloseFrame {
            code: CloseCode::Normal,
            reason: CLIENT_CLOSE_REASON.into(),
        };
        let result = socket.send(Message::Close(Some(frame))).await;
        self.close = Some(close);
        self.terminated = true;
        match result {
            Ok(())
            | Err(
                tokio_tungstenite::tungstenite::Error::ConnectionClosed
                | tokio_tungstenite::tungstenite::Error::AlreadyClosed,
            ) => Ok(()),
            Err(source) => Err(socket_error(self.endpoint.clone(), source)),
        }
    }
}

impl fmt::Debug for EventStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EventStream")
            .field("client", &"[REDACTED]")
            .field("panel_origin", &origin(&self.endpoint))
            .field("endpoint_path", &"[REDACTED]")
            .field(
                "socket",
                &self.socket.as_ref().map(|_| "[CONNECTED; REDACTED]"),
            )
            .field("closed", &self.terminated)
            .field("close", &self.close)
            .finish()
    }
}

impl Stream for EventStream {
    type Item = Result<PanelEvent>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if self.terminated {
                return Poll::Ready(None);
            }
            let poll = if let Some(socket) = self.socket.as_mut() {
                Pin::new(socket).poll_next(context)
            } else {
                self.terminated = true;
                return Poll::Ready(None);
            };
            match poll {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    return Poll::Ready(Some(PanelEvent::decode(text.as_str())));
                }
                Poll::Ready(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                Poll::Ready(Some(Ok(Message::Close(frame)))) => {
                    self.close = Some(frame.map_or(
                        WebSocketClose {
                            code: 1005,
                            reason: String::new(),
                        },
                        |frame| WebSocketClose {
                            code: frame.code.into(),
                            reason: frame.reason.to_string(),
                        },
                    ));
                    self.socket = None;
                    self.terminated = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Ok(Message::Binary(_)))) => {
                    self.socket = None;
                    self.terminated = true;
                    return Poll::Ready(Some(Err(Error::UnexpectedWebSocketFrame {
                        url: Box::new(self.endpoint.clone()),
                        kind: "binary data",
                    })));
                }
                Poll::Ready(Some(Ok(Message::Frame(_)))) => {
                    self.socket = None;
                    self.terminated = true;
                    return Poll::Ready(Some(Err(Error::UnexpectedWebSocketFrame {
                        url: Box::new(self.endpoint.clone()),
                        kind: "raw",
                    })));
                }
                Poll::Ready(
                    Some(Err(
                        tokio_tungstenite::tungstenite::Error::ConnectionClosed
                        | tokio_tungstenite::tungstenite::Error::AlreadyClosed,
                    ))
                    | None,
                ) => {
                    self.socket = None;
                    self.terminated = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(source))) => {
                    self.socket = None;
                    self.terminated = true;
                    return Poll::Ready(Some(Err(socket_error(self.endpoint.clone(), source))));
                }
            }
        }
    }
}

impl FusedStream for EventStream {
    fn is_terminated(&self) -> bool {
        self.terminated
    }
}

async fn connect_socket(client: &Client) -> Result<(Url, Socket)> {
    let http_url = client.endpoint("ws")?;
    let mut endpoint = http_url.clone();
    endpoint
        .set_scheme(match http_url.scheme() {
            "http" => "ws",
            "https" => "wss",
            _ => {
                return Err(Error::Configuration(
                    "panel URL must use HTTP or HTTPS".to_owned(),
                ));
            }
        })
        .map_err(|()| Error::Configuration("could not construct WebSocket URL".to_owned()))?;

    let mut request = endpoint
        .as_str()
        .into_client_request()
        .map_err(|source| socket_error(endpoint.clone(), source))?;
    let user_agent = client.user_agent().parse().map_err(|error| {
        Error::Configuration(format!("invalid WebSocket user agent header: {error}"))
    })?;
    request.headers_mut().insert(
        tokio_tungstenite::tungstenite::http::header::USER_AGENT,
        user_agent,
    );
    if let Some(cookie) = client.cookie_header(&http_url) {
        request
            .headers_mut()
            .insert(tokio_tungstenite::tungstenite::http::header::COOKIE, cookie);
    }

    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_SIZE))
        .max_frame_size(Some(MAX_MESSAGE_SIZE));
    let duration = client.connect_timeout();
    let connect = async {
        let transport = open_transport(client, &endpoint).await?;
        let connector = client
            .accepts_invalid_certs()
            .then(insecure_rustls_connector);
        client_async_tls_with_config(request, transport, Some(config), connector)
            .await
            .map_err(|source| handshake_error(endpoint.clone(), source))
    };
    let (socket, _) =
        timeout(duration, connect)
            .await
            .map_err(|_| Error::WebSocketConnectTimeout {
                url: Box::new(endpoint.clone()),
                timeout: duration,
            })??;
    Ok((endpoint, socket))
}

async fn open_transport(client: &Client, endpoint: &Url) -> Result<BoxedIo> {
    let Some(proxy) = client.proxy() else {
        return TcpStream::connect(target_address(endpoint)?)
            .await
            .map(|stream| Box::new(stream) as BoxedIo)
            .map_err(|source| {
                socket_error(
                    endpoint.clone(),
                    tokio_tungstenite::tungstenite::Error::Io(source),
                )
            });
    };

    let result = match proxy.scheme() {
        ProxyScheme::Http | ProxyScheme::Https => open_http_tunnel(proxy, endpoint, client).await,
        ProxyScheme::Socks5 | ProxyScheme::Socks5h => open_socks_tunnel(proxy, endpoint).await,
    };
    result.map_err(|source| Error::Proxy {
        scheme: proxy.scheme(),
        url: Box::new(endpoint.clone()),
        source: Box::new(source),
    })
}

async fn open_http_tunnel(
    proxy: &ProxyConfig,
    endpoint: &Url,
    client: &Client,
) -> std::result::Result<BoxedIo, ProxyError> {
    let tcp = TcpStream::connect((proxy.host(), proxy.port()))
        .await
        .map_err(ProxyError::Io)?;
    let mut transport: BoxedIo = if proxy.scheme() == ProxyScheme::Https {
        let config = proxy_tls_config(client.accepts_invalid_certs())?;
        let server_name = ServerName::try_from(proxy.host().to_owned()).map_err(|_| {
            ProxyError::TlsConfiguration("proxy host is not a valid TLS server name".to_owned())
        })?;
        let tls = TlsConnector::from(config)
            .connect(server_name, tcp)
            .await
            .map_err(ProxyError::Io)?;
        Box::new(tls)
    } else {
        Box::new(tcp)
    };

    let authority = target_authority(endpoint)?;
    let authentication = proxy
        .credentials()
        .map_or_else(String::new, |(username, password)| {
            let encoded = BASE64_STANDARD.encode(format!("{username}:{password}"));
            format!("Proxy-Authorization: Basic {encoded}\r\n")
        });
    let request = format!(
        "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nUser-Agent: {}\r\n{authentication}\r\n",
        client.user_agent()
    );
    transport
        .write_all(request.as_bytes())
        .await
        .map_err(ProxyError::Io)?;
    transport.flush().await.map_err(ProxyError::Io)?;
    read_connect_response(&mut transport).await?;
    Ok(transport)
}

async fn read_connect_response(transport: &mut BoxedIo) -> std::result::Result<(), ProxyError> {
    let mut response = Vec::with_capacity(1024);
    while !response.windows(4).any(|window| window == b"\r\n\r\n") {
        if response.len() == MAX_CONNECT_RESPONSE_SIZE {
            return Err(ProxyError::HttpResponseTooLarge {
                limit: MAX_CONNECT_RESPONSE_SIZE,
            });
        }
        let remaining = MAX_CONNECT_RESPONSE_SIZE - response.len();
        let mut buffer = [0_u8; 1024];
        let capacity = remaining.min(buffer.len());
        let read = transport
            .read(&mut buffer[..capacity])
            .await
            .map_err(ProxyError::Io)?;
        if read == 0 {
            return Err(ProxyError::InvalidHttpResponse);
        }
        response.extend_from_slice(&buffer[..read]);
    }

    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut parsed = httparse::Response::new(&mut headers);
    match parsed.parse(&response) {
        Ok(httparse::Status::Complete(_)) => {}
        Ok(httparse::Status::Partial) | Err(_) => return Err(ProxyError::InvalidHttpResponse),
    }
    let status = parsed
        .code
        .and_then(|code| StatusCode::from_u16(code).ok())
        .ok_or(ProxyError::InvalidHttpResponse)?;
    if !status.is_success() {
        return Err(ProxyError::HttpStatus(status));
    }
    Ok(())
}

async fn open_socks_tunnel(
    proxy: &ProxyConfig,
    endpoint: &Url,
) -> std::result::Result<BoxedIo, ProxyError> {
    let host = endpoint.host_str().ok_or(ProxyError::InvalidHttpResponse)?;
    let port = endpoint
        .port_or_known_default()
        .ok_or(ProxyError::InvalidHttpResponse)?;
    if proxy.scheme().resolves_remotely() {
        let target = TargetAddr::Domain(Cow::Owned(host.to_owned()), port);
        return connect_socks(proxy, target)
            .await
            .map(|stream| Box::new(stream) as BoxedIo);
    }

    let addresses = lookup_host((host, port)).await.map_err(ProxyError::Io)?;
    let mut last_error = None;
    for address in addresses {
        match connect_socks(proxy, TargetAddr::Ip(address)).await {
            Ok(stream) => return Ok(Box::new(stream)),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        ProxyError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "target hostname did not resolve",
        ))
    }))
}

async fn connect_socks(
    proxy: &ProxyConfig,
    target: TargetAddr<'_>,
) -> std::result::Result<Socks5Stream<TcpStream>, ProxyError> {
    let stream = match proxy.credentials() {
        Some((username, password)) => {
            Socks5Stream::connect_with_password(
                (proxy.host(), proxy.port()),
                target,
                username,
                password,
            )
            .await
        }
        None => Socks5Stream::connect((proxy.host(), proxy.port()), target).await,
    }
    .map_err(|source| ProxyError::Socks5(Box::new(source)))?;
    Ok(stream)
}

fn target_address(endpoint: &Url) -> Result<(&str, u16)> {
    let host = endpoint
        .host_str()
        .ok_or_else(|| Error::Configuration("WebSocket URL must contain a host".to_owned()))?;
    let port = endpoint.port_or_known_default().ok_or_else(|| {
        Error::Configuration("WebSocket URL must contain or imply a port".to_owned())
    })?;
    Ok((host, port))
}

fn target_authority(endpoint: &Url) -> std::result::Result<String, ProxyError> {
    let host = endpoint.host().ok_or(ProxyError::InvalidHttpResponse)?;
    let port = endpoint
        .port_or_known_default()
        .ok_or(ProxyError::InvalidHttpResponse)?;
    Ok(match host {
        Host::Ipv6(address) => format!("[{address}]:{port}"),
        Host::Ipv4(address) => format!("{address}:{port}"),
        Host::Domain(domain) => format!("{domain}:{port}"),
    })
}

fn proxy_tls_config(
    accept_invalid_certs: bool,
) -> std::result::Result<Arc<ClientConfig>, ProxyError> {
    if accept_invalid_certs {
        return Ok(match insecure_rustls_connector() {
            Connector::Rustls(config) => config,
            _ => unreachable!("insecure connector is always rustls"),
        });
    }

    let loaded = rustls_native_certs::load_native_certs();
    if loaded.certs.is_empty() {
        return Err(ProxyError::TlsConfiguration(
            "no native root CA certificates were found".to_owned(),
        ));
    }
    let mut roots = RootCertStore::empty();
    roots.add_parsable_certificates(loaded.certs);
    Ok(Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
}

fn handshake_error(endpoint: Url, source: tokio_tungstenite::tungstenite::Error) -> Error {
    if let tokio_tungstenite::tungstenite::Error::Http(response) = &source {
        return match response.status() {
            StatusCode::UNAUTHORIZED => Error::Unauthorized {
                method: Method::GET,
                url: Box::new(endpoint),
            },
            StatusCode::FORBIDDEN => Error::Forbidden {
                method: Method::GET,
                url: Box::new(endpoint),
            },
            status => Error::HttpStatus {
                method: Method::GET,
                url: Box::new(endpoint),
                status,
            },
        };
    }
    socket_error(endpoint, source)
}

fn socket_error(endpoint: Url, source: tokio_tungstenite::tungstenite::Error) -> Error {
    Error::WebSocket {
        url: Box::new(endpoint),
        source: Box::new(source),
    }
}

fn insecure_rustls_connector() -> Connector {
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
        .with_no_client_auth();
    Connector::Rustls(Arc::new(config))
}

#[derive(Debug)]
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

fn origin(url: &Url) -> String {
    let mut value = url.clone();
    value.set_path("");
    value.set_query(None);
    value.set_fragment(None);
    value.to_string().trim_end_matches('/').to_owned()
}
