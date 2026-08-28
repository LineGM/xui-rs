use std::{
    fmt,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::{SinkExt, Stream, StreamExt, stream::FusedStream};
use reqwest::{Method, StatusCode};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        protocol::{WebSocketConfig, frame::CloseFrame, frame::coding::CloseCode},
    },
};
use url::Url;

use super::PanelEvent;
use crate::{Client, Error, Result};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;
const CLIENT_CLOSE_REASON: &str = "xui-rs disconnect";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
    /// 3x-ui v3.6.0 deliberately ignores API bearer tokens on `/ws`. Call
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
    /// oversized messages, or JSON that does not match the event's v3.6.0
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
    let connector = client
        .accepts_invalid_certs()
        .then(insecure_rustls_connector);
    let connect = connect_async_tls_with_config(request, Some(config), false, connector);
    let duration = client.connect_timeout();
    let (socket, _) = timeout(duration, connect)
        .await
        .map_err(|_| Error::WebSocketConnectTimeout {
            url: Box::new(endpoint.clone()),
            timeout: duration,
        })?
        .map_err(|source| handshake_error(endpoint.clone(), source))?;
    Ok((endpoint, socket))
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
