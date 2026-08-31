#![allow(missing_docs, clippy::result_large_err, clippy::too_many_lines)]

use std::{net::Ipv4Addr, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use futures_util::SinkExt;
use reqwest::StatusCode;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
    },
};
use xui_rs::{
    Client, Error, ErrorKind, LoginRequest, PanelEventKind, ProxyConfig, ProxyError, ProxyScheme,
    SubscriptionClient,
};

const PROXY_USER: &str = "proxy-user";
const PROXY_PASSWORD: &str = "proxy-password-secret";

#[test]
fn proxy_configuration_is_typed_validated_and_redacted() {
    let cases = [
        ("http://proxy.example", ProxyScheme::Http),
        ("https://proxy.example:8443/", ProxyScheme::Https),
        ("socks5://127.0.0.1:1080", ProxyScheme::Socks5),
        ("socks5h://proxy.example", ProxyScheme::Socks5h),
    ];
    for (url, expected) in cases {
        let proxy = ProxyConfig::new(url).unwrap();
        assert_eq!(proxy.scheme(), expected);
        assert_eq!(proxy.scheme().to_string(), proxy.url().scheme());
        assert!(!proxy.has_basic_auth());
    }

    for invalid in [
        "ftp://proxy.example",
        "http://user:password@proxy.example",
        "http://proxy.example/private",
        "http://proxy.example?secret=value",
        "http://proxy.example#fragment",
    ] {
        assert!(ProxyConfig::new(invalid).is_err(), "accepted {invalid}");
    }

    let proxy = ProxyConfig::new("socks5h://private-proxy.example:1080")
        .unwrap()
        .with_basic_auth(PROXY_USER, PROXY_PASSWORD)
        .unwrap();
    let debug = format!("{proxy:?}");
    assert!(proxy.has_basic_auth());
    assert!(!debug.contains("private-proxy"));
    assert!(!debug.contains(PROXY_USER));
    assert!(!debug.contains(PROXY_PASSWORD));
    assert!(debug.contains("Socks5h"));

    let oversized = "x".repeat(256);
    assert!(
        ProxyConfig::new("socks5://proxy.example")
            .unwrap()
            .with_basic_auth(PROXY_USER, oversized)
            .is_err()
    );
}

#[tokio::test]
async fn one_http_proxy_routes_panel_subscription_and_websocket_transports() {
    let websocket_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let websocket_address = websocket_listener.local_addr().unwrap();
    let websocket_server = tokio::spawn(async move {
        let (stream, _) = websocket_listener.accept().await.unwrap();
        let mut socket = accept_hdr_async(stream, |request: &Request, response: Response| {
            assert_eq!(request.uri().path(), "/secret/ws");
            assert_eq!(
                request.headers().get("cookie").unwrap(),
                "3x-ui=authenticated-cookie"
            );
            Ok(response)
        })
        .await
        .unwrap();
        socket
            .send(Message::Text(
                json!({"type": "future_proxy_event", "payload": {}, "time": 42})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });

    let proxy_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let proxy_server = tokio::spawn(async move {
        let csrf = read_http_request(&proxy_listener).await;
        assert!(csrf.text.starts_with("GET http://panel.invalid:"));
        assert!(csrf.text.contains("/secret/csrf-token HTTP/1.1"));
        assert_proxy_authentication(&csrf.text);
        write_http_response(
            csrf.stream,
            "200 OK",
            br#"{"success":true,"obj":"csrf-value"}"#,
            Some("3x-ui=preauth-cookie"),
        )
        .await;

        let login = read_http_request(&proxy_listener).await;
        assert!(login.text.starts_with("POST http://panel.invalid:"));
        assert!(login.text.contains("/secret/login HTTP/1.1"));
        assert!(login.text.contains("x-csrf-token: csrf-value"));
        assert!(login.text.contains("cookie: 3x-ui=preauth-cookie"));
        assert!(login.text.contains("password-secret"));
        assert_proxy_authentication(&login.text);
        write_http_response(
            login.stream,
            "200 OK",
            br#"{"success":true,"msg":"ok"}"#,
            Some("3x-ui=authenticated-cookie"),
        )
        .await;

        let subscription = read_http_request(&proxy_listener).await;
        assert!(
            subscription
                .text
                .starts_with("GET http://subscriptions.invalid:")
        );
        assert!(subscription.text.contains("/sub/private-id HTTP/1.1"));
        assert_proxy_authentication(&subscription.text);
        write_http_response(subscription.stream, "200 OK", b"dmxlc3M6Ly9wcm94aWVk", None).await;

        let connect = read_http_request(&proxy_listener).await;
        assert!(connect.text.starts_with(&format!(
            "CONNECT panel.invalid:{} HTTP/1.1",
            websocket_address.port()
        )));
        assert_proxy_authentication(&connect.text);
        let mut downstream = connect.stream;
        let mut upstream = TcpStream::connect(websocket_address).await.unwrap();
        downstream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .unwrap();
        copy_bidirectional(&mut downstream, &mut upstream)
            .await
            .unwrap();
    });

    let proxy = ProxyConfig::new(format!("http://{proxy_address}"))
        .unwrap()
        .with_basic_auth(PROXY_USER, PROXY_PASSWORD)
        .unwrap();
    let client = Client::builder(format!(
        "http://panel.invalid:{}/secret",
        websocket_address.port()
    ))
    .unwrap()
    .proxy(proxy.clone())
    .build()
    .unwrap();
    client
        .auth()
        .login(LoginRequest::new("admin", "password-secret"))
        .await
        .unwrap();

    let subscription = SubscriptionClient::builder("http://subscriptions.invalid:2096")
        .unwrap()
        .proxy(proxy)
        .build()
        .unwrap()
        .raw("private-id")
        .await
        .unwrap();
    assert_eq!(subscription.content.as_str(), "dmxlc3M6Ly9wcm94aWVk");

    let mut events = client.events().connect().await.unwrap();
    let event = events.next_event().await.unwrap().unwrap();
    assert_eq!(event.timestamp_ms, 42);
    assert!(matches!(event.kind, PanelEventKind::Unknown { .. }));
    assert!(events.next_event().await.unwrap().is_none());

    proxy_server.await.unwrap();
    websocket_server.await.unwrap();
}

#[tokio::test]
async fn socks5h_authentication_and_remote_dns_are_used_for_websocket() {
    let websocket_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let websocket_address = websocket_listener.local_addr().unwrap();
    let websocket_server = tokio::spawn(async move {
        let (stream, _) = websocket_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"type": "socks_event", "payload": {}, "time": 73})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });

    let socks_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let socks_address = socks_listener.local_addr().unwrap();
    let socks_server = tokio::spawn(async move {
        let (mut downstream, _) = socks_listener.accept().await.unwrap();
        let mut greeting = [0_u8; 2];
        downstream.read_exact(&mut greeting).await.unwrap();
        assert_eq!(greeting[0], 5);
        let mut methods = vec![0_u8; usize::from(greeting[1])];
        downstream.read_exact(&mut methods).await.unwrap();
        assert!(methods.contains(&2));
        downstream.write_all(&[5, 2]).await.unwrap();

        let mut auth = [0_u8; 2];
        downstream.read_exact(&mut auth).await.unwrap();
        assert_eq!(auth[0], 1);
        let mut username = vec![0_u8; usize::from(auth[1])];
        downstream.read_exact(&mut username).await.unwrap();
        let password_length = downstream.read_u8().await.unwrap();
        let mut password = vec![0_u8; usize::from(password_length)];
        downstream.read_exact(&mut password).await.unwrap();
        assert_eq!(username, PROXY_USER.as_bytes());
        assert_eq!(password, PROXY_PASSWORD.as_bytes());
        downstream.write_all(&[1, 0]).await.unwrap();

        let mut command = [0_u8; 4];
        downstream.read_exact(&mut command).await.unwrap();
        assert_eq!(command, [5, 1, 0, 3], "SOCKS5h must send a domain");
        let domain_length = downstream.read_u8().await.unwrap();
        let mut domain = vec![0_u8; usize::from(domain_length)];
        downstream.read_exact(&mut domain).await.unwrap();
        let target_port = downstream.read_u16().await.unwrap();
        assert_eq!(domain, b"panel.internal.invalid");
        assert_eq!(target_port, websocket_address.port());

        let mut upstream = TcpStream::connect(websocket_address).await.unwrap();
        let [a, b, c, d] = Ipv4Addr::LOCALHOST.octets();
        let [port_high, port_low] = websocket_address.port().to_be_bytes();
        downstream
            .write_all(&[5, 0, 0, 1, a, b, c, d, port_high, port_low])
            .await
            .unwrap();
        copy_bidirectional(&mut downstream, &mut upstream)
            .await
            .unwrap();
    });

    let proxy = ProxyConfig::new(format!("socks5h://{socks_address}"))
        .unwrap()
        .with_basic_auth(PROXY_USER, PROXY_PASSWORD)
        .unwrap();
    let client = Client::builder(format!(
        "http://panel.internal.invalid:{}/secret",
        websocket_address.port()
    ))
    .unwrap()
    .proxy(proxy)
    .build()
    .unwrap();
    let mut events = client.events().connect().await.unwrap();
    assert_eq!(events.next_event().await.unwrap().unwrap().timestamp_ms, 73);
    assert!(events.next_event().await.unwrap().is_none());

    socks_server.await.unwrap();
    websocket_server.await.unwrap();
}

#[tokio::test]
async fn socks5_resolves_target_locally_and_sends_an_ip() {
    let websocket_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let websocket_address = websocket_listener.local_addr().unwrap();
    let websocket_server = tokio::spawn(async move {
        let (stream, _) = websocket_listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"type": "local_dns_event", "payload": {}, "time": 91})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });

    let socks_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let socks_address = socks_listener.local_addr().unwrap();
    let socks_server = tokio::spawn(async move {
        let (mut downstream, _) = socks_listener.accept().await.unwrap();
        let mut greeting = [0_u8; 2];
        downstream.read_exact(&mut greeting).await.unwrap();
        assert_eq!(greeting[0], 5);
        let mut methods = vec![0_u8; usize::from(greeting[1])];
        downstream.read_exact(&mut methods).await.unwrap();
        assert!(methods.contains(&0));
        downstream.write_all(&[5, 0]).await.unwrap();

        let mut command = [0_u8; 4];
        downstream.read_exact(&mut command).await.unwrap();
        assert_eq!(&command[..3], &[5, 1, 0]);
        match command[3] {
            1 => {
                let mut address = [0_u8; 4];
                downstream.read_exact(&mut address).await.unwrap();
                assert!(Ipv4Addr::from(address).is_loopback());
            }
            4 => {
                let mut address = [0_u8; 16];
                downstream.read_exact(&mut address).await.unwrap();
                assert!(std::net::Ipv6Addr::from(address).is_loopback());
            }
            other => panic!("SOCKS5 local DNS sent unexpected ATYP {other}"),
        }
        assert_eq!(
            downstream.read_u16().await.unwrap(),
            websocket_address.port()
        );

        let mut upstream = TcpStream::connect(websocket_address).await.unwrap();
        let [a, b, c, d] = Ipv4Addr::LOCALHOST.octets();
        let [port_high, port_low] = websocket_address.port().to_be_bytes();
        downstream
            .write_all(&[5, 0, 0, 1, a, b, c, d, port_high, port_low])
            .await
            .unwrap();
        copy_bidirectional(&mut downstream, &mut upstream)
            .await
            .unwrap();
    });

    let proxy = ProxyConfig::new(format!("socks5://{socks_address}")).unwrap();
    let client = Client::builder(format!(
        "http://localhost:{}/secret",
        websocket_address.port()
    ))
    .unwrap()
    .proxy(proxy)
    .build()
    .unwrap();
    let mut events = client.events().connect().await.unwrap();
    assert_eq!(events.next_event().await.unwrap().unwrap().timestamp_ms, 91);
    assert!(events.next_event().await.unwrap().is_none());

    socks_server.await.unwrap();
    websocket_server.await.unwrap();
}

#[tokio::test]
async fn proxy_rejection_has_safe_stable_introspection() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let request = read_http_request(&listener).await;
        assert_proxy_authentication(&request.text);
        write_http_response(
            request.stream,
            "407 Proxy Authentication Required",
            b"proxy details must not escape",
            None,
        )
        .await;
    });

    let proxy = ProxyConfig::new(format!("http://{address}"))
        .unwrap()
        .with_basic_auth(PROXY_USER, PROXY_PASSWORD)
        .unwrap();
    let client = Client::builder("http://panel.invalid:54321/secret")
        .unwrap()
        .proxy(proxy)
        .build()
        .unwrap();
    let error = client.events().connect().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Proxy);
    assert_eq!(error.kind().as_str(), "proxy");
    assert_eq!(error.status(), None);
    assert_eq!(error.method(), None);
    assert_eq!(error.url().unwrap().host_str(), Some("panel.invalid"));
    assert!(matches!(
        error,
        Error::Proxy {
            scheme: ProxyScheme::Http,
            ref source,
            ..
        } if matches!(source.as_ref(), ProxyError::HttpStatus(StatusCode::PROXY_AUTHENTICATION_REQUIRED))
    ));
    let debug = format!("{error:?}");
    assert!(!debug.contains(PROXY_USER));
    assert!(!debug.contains(PROXY_PASSWORD));
    assert!(!debug.contains(&address.to_string()));

    server.await.unwrap();
}

struct CapturedRequest {
    stream: TcpStream,
    text: String,
}

async fn read_http_request(listener: &TcpListener) -> CapturedRequest {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).await.unwrap();
        assert_ne!(read, 0, "request ended before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(position) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or_default();
    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buffer).await.unwrap();
        assert_ne!(read, 0, "request ended before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    CapturedRequest {
        stream,
        text: String::from_utf8(bytes).unwrap(),
    }
}

fn assert_proxy_authentication(request: &str) {
    let expected = BASE64_STANDARD.encode(format!("{PROXY_USER}:{PROXY_PASSWORD}"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("proxy-authorization: basic {expected}").to_ascii_lowercase())
    );
}

async fn write_http_response(
    mut stream: TcpStream,
    status: &str,
    body: &[u8],
    cookie: Option<&str>,
) {
    let cookie = cookie.map_or_else(String::new, |value| {
        format!("Set-Cookie: {value}; Path=/; HttpOnly\r\n")
    });
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{cookie}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
}

#[test]
fn proxy_config_is_cheap_to_share_across_builders() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProxyConfig>();
    assert_send_sync::<ProxyError>();

    let proxy = Arc::new(ProxyConfig::new("http://proxy.example").unwrap());
    let _panel = Client::builder("https://panel.example")
        .unwrap()
        .proxy(Arc::unwrap_or_clone(Arc::clone(&proxy)))
        .no_proxy()
        .proxy_url("socks5h://proxy.example:1080")
        .unwrap();
    let _subscription = SubscriptionClient::builder("https://sub.example")
        .unwrap()
        .proxy(Arc::unwrap_or_clone(proxy))
        .no_proxy()
        .proxy_url("https://proxy.example:8443")
        .unwrap();
}
