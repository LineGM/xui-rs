#![allow(missing_docs, clippy::result_large_err)]

use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
        http::StatusCode,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};
use xui_rs::{
    Client, Error, EventMessageType, LoginRequest, NotificationLevel, PanelEventKind, ProcessState,
};

const PREAUTH_COOKIE: &str = "3x-ui=preauth-cookie";
const SESSION_COOKIE: &str = "3x-ui=authenticated-cookie";

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn session_cookie_authenticates_every_typed_event_and_reconnect() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handshakes = Arc::new(Mutex::new(Vec::new()));
    let server_handshakes = Arc::clone(&handshakes);

    let server = tokio::spawn(async move {
        let csrf = read_http_request(&listener).await;
        assert!(csrf.starts_with("GET /secret/csrf-token HTTP/1.1"));
        write_json_response(
            csrf.stream,
            json!({"success": true, "obj": "csrf-value"}),
            Some(PREAUTH_COOKIE),
        )
        .await;

        let login = read_http_request(&listener).await;
        assert!(login.text.starts_with("POST /secret/login HTTP/1.1"));
        assert!(login.text.contains("x-csrf-token: csrf-value"));
        assert!(login.text.contains(&format!("cookie: {PREAUTH_COOKIE}")));
        assert!(login.text.contains("\"password\":\"password-secret\""));
        write_json_response(
            login.stream,
            json!({"success": true, "msg": "ok"}),
            Some(SESSION_COOKIE),
        )
        .await;

        let (stream, _) = listener.accept().await.unwrap();
        let captured = Arc::clone(&server_handshakes);
        let mut websocket =
            accept_hdr_async(stream, move |request: &Request, response: Response| {
                validate_handshake(request, &captured);
                Ok(response)
            })
            .await
            .unwrap();

        let messages = source_messages();
        for message in messages {
            websocket
                .send(Message::Text(message.to_string().into()))
                .await
                .unwrap();
        }
        websocket
            .send(Message::Ping(Vec::from("keepalive").into()))
            .await
            .unwrap();
        websocket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "server rotation".into(),
            })))
            .await
            .unwrap();
        drop(websocket);

        let (stream, _) = listener.accept().await.unwrap();
        let captured = Arc::clone(&server_handshakes);
        let mut websocket =
            accept_hdr_async(stream, move |request: &Request, response: Response| {
                validate_handshake(request, &captured);
                Ok(response)
            })
            .await
            .unwrap();
        let close = websocket.next().await.unwrap().unwrap();
        match close {
            Message::Close(Some(frame)) => {
                assert_eq!(frame.code, CloseCode::Normal);
                assert_eq!(frame.reason, "xui-rs disconnect");
            }
            other => panic!("expected close frame, got {other:?}"),
        }
    });

    let client = Client::builder(format!("http://{address}/secret"))
        .unwrap()
        .bearer_token("api-secret-must-not-reach-websocket")
        .user_agent("xui-rs-event-test")
        .build()
        .unwrap();
    client
        .auth()
        .login(LoginRequest::new("admin", "password-secret"))
        .await
        .unwrap();

    let mut events = client.events().connect().await.unwrap();
    assert!(!events.is_closed());
    assert!(events.reconnect().await.is_err());

    let malformed = events.next_event().await.unwrap_err();
    assert!(matches!(
        malformed,
        Error::EventDecode {
            message_type: Some(message_type),
            ..
        } if message_type == "status"
    ));

    let status = events.next_event().await.unwrap().unwrap();
    assert_eq!(status.timestamp_ms, 2);
    match status.kind {
        PanelEventKind::Status(status) => {
            assert!((status.cpu - 12.5).abs() < f64::EPSILON);
            assert_eq!(status.xray.state, ProcessState::Running);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let traffic = events.next_event().await.unwrap().unwrap();
    match &traffic.kind {
        PanelEventKind::Traffic(update) => {
            assert_eq!(update.traffics.as_ref().unwrap()[0].tag, "inbound-1");
            assert_eq!(update.client_traffics.as_ref().unwrap()[0].up, 5);
            assert_eq!(update.online_clients, ["private@example.com"]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    let traffic_debug = format!("{traffic:?}");
    assert!(!traffic_debug.contains("private-client-uuid"));
    assert!(!traffic_debug.contains("private-sub-id"));

    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::Inbounds(rows) if rows.is_empty()
    ));
    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::Outbounds(rows) if rows[0].tag == "proxy"
    ));
    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::Nodes(rows) if rows.is_empty()
    ));

    let notification = events.next_event().await.unwrap().unwrap();
    match &notification.kind {
        PanelEventKind::Notification(value) => {
            assert_eq!(value.level, NotificationLevel::Warning);
            assert_eq!(value.title, "Private operation");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(!format!("{notification:?}").contains("operator-only details"));

    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::XrayState(change)
            if change.state == ProcessState::Error && change.error_msg == "private xray error"
    ));
    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::ClientStats(update)
            if update.snapshot && update.clients[0].email == "private@example.com"
    ));

    let clients = events.next_event().await.unwrap().unwrap();
    assert!(matches!(clients.kind, PanelEventKind::Clients(_)));
    assert!(!format!("{clients:?}").contains("reserved-private-secret"));

    assert!(matches!(
        events.next_event().await.unwrap().unwrap().kind,
        PanelEventKind::Invalidate(value) if value.target == EventMessageType::Clients
    ));

    let future = events.next_event().await.unwrap().unwrap();
    match &future.kind {
        PanelEventKind::Unknown {
            message_type,
            payload,
        } => {
            assert_eq!(message_type, "future_event");
            assert_eq!(payload["password"], "future-private-secret");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(!format!("{future:?}").contains("future-private-secret"));

    assert!(events.next_event().await.unwrap().is_none());
    assert!(events.is_closed());
    assert_eq!(events.close_info().unwrap().code, 1000);
    assert_eq!(events.close_info().unwrap().reason, "server rotation");

    events.reconnect().await.unwrap();
    events.close().await.unwrap();
    assert!(events.is_closed());
    assert_eq!(events.close_info().unwrap().reason, "xui-rs disconnect");
    assert!(!format!("{events:?}").contains("/secret/ws"));

    server.await.unwrap();
    let captured = handshakes.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert!(
        captured
            .iter()
            .all(|request| request.contains(SESSION_COOKIE))
    );
    assert!(
        captured
            .iter()
            .all(|request| !request.contains("api-secret"))
    );
}

#[tokio::test]
async fn unauthorized_handshake_maps_to_standard_authentication_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let error = accept_hdr_async(stream, |request: &Request, _: Response| {
            assert_eq!(request.uri().path(), "/secret/ws");
            assert!(request.headers().get("cookie").is_none());
            Err(tokio_tungstenite::tungstenite::http::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Some("login required".to_owned()))
                .unwrap())
        })
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            tokio_tungstenite::tungstenite::Error::Http(response)
                if response.status() == StatusCode::UNAUTHORIZED
        ));
    });

    let client = Client::new(format!("http://{address}/secret")).unwrap();
    let error = client.events().connect().await.unwrap_err();
    assert!(matches!(
        error,
        Error::Unauthorized { method, url }
            if method == reqwest::Method::GET && url.path() == "/secret/ws"
    ));
    server.await.unwrap();
}

fn validate_handshake(request: &Request, captured: &Arc<Mutex<Vec<String>>>) {
    assert_eq!(request.uri().path(), "/secret/ws");
    assert_eq!(request.headers().get("cookie").unwrap(), SESSION_COOKIE);
    assert_eq!(
        request.headers().get("user-agent").unwrap(),
        "xui-rs-event-test"
    );
    assert!(request.headers().get("authorization").is_none());
    captured.lock().unwrap().push(format!(
        "{} {}",
        request.uri(),
        request.headers().get("cookie").unwrap().to_str().unwrap()
    ));
}

fn source_messages() -> Vec<Value> {
    vec![
        json!({"type": "status", "payload": {"cpu": "invalid"}, "time": 1}),
        json!({
            "type": "status",
            "payload": {"cpu": 12.5, "xray": {"state": "running"}},
            "time": 2
        }),
        json!({
            "type": "traffic",
            "payload": {
                "traffics": [{"IsInbound": true, "IsOutbound": false, "Tag": "inbound-1", "Up": 50, "Down": 70}],
                "clientTraffics": [{
                    "id": 1, "inboundId": 7, "enable": true,
                    "email": "private@example.com", "uuid": "private-client-uuid",
                    "subId": "private-sub-id", "up": 5, "down": 6,
                    "expiryTime": 0, "total": 0, "reset": 0, "lastOnline": 123
                }],
                "onlineClients": ["private@example.com"],
                "onlineByGuid": {"panel-guid": ["private@example.com"]},
                "activeInbounds": {"panel-guid": ["inbound-1"]},
                "lastOnlineMap": {"private@example.com": 123}
            },
            "time": 3
        }),
        json!({"type": "inbounds", "payload": [], "time": 4}),
        json!({"type": "outbounds", "payload": [{"id": 1, "tag": "proxy", "up": 10, "down": 20, "total": 30}], "time": 5}),
        json!({"type": "nodes", "payload": [], "time": 6}),
        json!({"type": "notification", "payload": {"title": "Private operation", "message": "operator-only details", "level": "warning"}, "time": 7}),
        json!({"type": "xray_state", "payload": {"state": "error", "errorMsg": "private xray error"}, "time": 8}),
        json!({
            "type": "client_stats",
            "payload": {
                "snapshot": true,
                "clients": [{
                    "id": 1, "inboundId": 7, "enable": true,
                    "email": "private@example.com", "uuid": "private-client-uuid",
                    "subId": "private-sub-id", "up": 100, "down": 200,
                    "expiryTime": 0, "total": 1000, "reset": 0, "lastOnline": 123
                }],
                "inbounds": [{"id": 7, "up": 100, "down": 200, "total": 1000, "enable": true}]
            },
            "time": 9
        }),
        json!({"type": "clients", "payload": {"token": "reserved-private-secret"}, "time": 10}),
        json!({"type": "invalidate", "payload": {"type": "clients"}, "time": 11}),
        json!({"type": "future_event", "payload": {"password": "future-private-secret"}, "time": 12}),
    ]
}

struct HttpRequest {
    stream: TcpStream,
    text: String,
}

impl HttpRequest {
    fn starts_with(&self, prefix: &str) -> bool {
        self.text.starts_with(prefix)
    }
}

async fn read_http_request(listener: &TcpListener) -> HttpRequest {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = stream.read(&mut buffer).await.unwrap();
        assert!(read > 0, "connection closed before HTTP request completed");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = find_header_end(&bytes) {
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
        assert!(bytes.len() < 64 * 1024, "test HTTP request too large");
    }
    HttpRequest {
        stream,
        text: String::from_utf8(bytes).unwrap(),
    }
}

async fn write_json_response(mut stream: TcpStream, body: Value, cookie: Option<&str>) {
    let body = body.to_string();
    let cookie = cookie.map_or_else(String::new, |value| {
        format!("Set-Cookie: {value}; Path=/secret/; HttpOnly; SameSite=Lax\r\n")
    });
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{cookie}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap();
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}
