#![allow(missing_docs, clippy::result_large_err)]

use std::{
    io::Write,
    net::Ipv4Addr,
    sync::{Arc, Mutex, OnceLock},
};

use reqwest::Method;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tracing_subscriber::{
    Layer as _, filter::filter_fn, fmt::MakeWriter, layer::SubscriberExt as _,
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, DEFAULT_API_RESPONSE_BODY_LIMIT, DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT,
    DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT, ErrorKind, SubscriptionClient,
};

#[tokio::test]
async fn declared_body_limit_is_typed_and_exact_boundary_is_accepted() {
    let server = MockServer::start().await;
    let body = br#"{"openapi":"3.0.0","paths":{}}"#;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/openapi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(2)
        .mount(&server)
        .await;

    let exact = Client::builder(server.uri())
        .unwrap()
        .response_body_limit(body.len())
        .build()
        .unwrap();
    assert_eq!(
        exact.panel().openapi().await.unwrap().version(),
        Some("3.0.0")
    );

    let limited = Client::builder(server.uri())
        .unwrap()
        .response_body_limit(body.len() - 1)
        .build()
        .unwrap();
    let error = limited.panel().openapi().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ResponseTooLarge);
    assert!(error.is_response_too_large());
    assert_eq!(error.response_body_limit(), Some(body.len() - 1));
    assert_eq!(error.advertised_content_length(), Some(body.len() as u64));
    assert_eq!(error.method(), Some(&Method::GET));
    assert_eq!(error.status(), None);
    assert_eq!(error.url().unwrap().path(), "/panel/api/openapi.json");
}

#[tokio::test]
async fn chunked_body_is_bounded_without_content_length() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = br#"{"openapi":"3.0.0","padding":"body-without-a-declared-size"}"#;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        read_request_headers(&mut stream).await;
        let headers = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        stream.write_all(headers).await.unwrap();
        stream
            .write_all(format!("{:x}\r\n", body.len()).as_bytes())
            .await
            .unwrap();
        stream.write_all(body).await.unwrap();
        stream.write_all(b"\r\n0\r\n\r\n").await.unwrap();
    });

    let client = Client::builder(format!("http://{address}"))
        .unwrap()
        .response_body_limit(16)
        .build()
        .unwrap();
    let error = client.panel().openapi().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ResponseTooLarge);
    assert_eq!(error.response_body_limit(), Some(16));
    assert_eq!(error.advertised_content_length(), None);
    server.await.unwrap();
}

#[tokio::test]
async fn head_metadata_does_not_treat_resource_length_as_a_received_body() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        read_request_headers(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
    });

    let client = SubscriptionClient::builder(format!("http://{address}"))
        .unwrap()
        .response_body_limit(1)
        .build()
        .unwrap();
    let metadata = client.raw_metadata("subscription-id").await.unwrap();
    assert_eq!(metadata.content_type.as_deref(), Some("text/plain"));
    server.await.unwrap();
}

#[tokio::test]
async fn database_download_uses_its_independent_limit() {
    let bytes = vec![0x5a; 128];
    let accepted_server = MockServer::start().await;
    mount_database(&accepted_server, bytes.clone()).await;
    let accepted = Client::builder(accepted_server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .response_body_limit(1)
        .download_body_limit(bytes.len())
        .build()
        .unwrap();
    assert_eq!(
        accepted.server().download_database().await.unwrap().bytes,
        bytes
    );

    let rejected_server = MockServer::start().await;
    mount_database(&rejected_server, bytes.clone()).await;
    let rejected = Client::builder(rejected_server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .response_body_limit(bytes.len() * 2)
        .download_body_limit(bytes.len() - 1)
        .build()
        .unwrap();
    let error = rejected.server().download_database().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ResponseTooLarge);
    assert_eq!(error.response_body_limit(), Some(bytes.len() - 1));
}

#[tokio::test]
async fn subscription_limit_error_keeps_identifier_redacted() {
    let server = MockServer::start().await;
    let body = b"private-subscription-document";
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/sub/private%2Fsubscription-id"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(1)
        .mount(&server)
        .await;
    let client = SubscriptionClient::builder(server.uri())
        .unwrap()
        .response_body_limit(body.len() - 1)
        .build()
        .unwrap();
    let error = client.raw("private/subscription-id").await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ResponseTooLarge);
    assert!(!error.to_string().contains("private/subscription-id"));
    assert!(!error.to_string().contains("private%2Fsubscription-id"));
    assert_eq!(error.url().unwrap().path(), "/sub/[REDACTED]");
}

#[tokio::test]
async fn tracing_is_correlated_but_never_contains_request_secrets_or_urls() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path(
            "/private-panel-path/panel/api/clients/subLinks/private%2Dsubscription%2Did",
        ))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"success": true, "obj": []})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let output = transport_trace_output();
    let client = Client::builder(format!("{}/private-panel-path", server.uri()))
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    client
        .clients()
        .subscription_links("private-subscription-id")
        .await
        .unwrap();

    let output = output.contents();
    assert!(output.contains("sending 3x-ui HTTP request"), "{output}");
    assert!(
        output.contains("received 3x-ui HTTP response headers"),
        "{output}"
    );
    assert!(output.contains("request_id="));
    assert!(output.contains("method=GET"));
    assert!(output.contains("status=200"));
    assert!(!output.contains("private-panel-path"));
    assert!(!output.contains("private-subscription-id"));
    assert!(!output.contains("private%2Dsubscription%2Did"));
    assert!(!output.contains("api-secret"));
    assert!(!output.contains(&server.uri()));
}

#[test]
fn safe_limits_are_the_builder_defaults_and_publicly_inspectable() {
    let panel = Client::new("https://panel.example.com").unwrap();
    assert_eq!(panel.response_body_limit(), DEFAULT_API_RESPONSE_BODY_LIMIT);
    assert_eq!(
        panel.download_body_limit(),
        DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT
    );
    let subscription = SubscriptionClient::new("https://sub.example.com").unwrap();
    assert_eq!(
        subscription.response_body_limit(),
        DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT
    );
}

async fn mount_database(server: &MockServer, bytes: Vec<u8>) {
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/server/getDb"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/octet-stream")
                .set_body_bytes(bytes),
        )
        .expect(1)
        .mount(server)
        .await;
}

async fn read_request_headers(stream: &mut tokio::net::TcpStream) {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).await.unwrap();
        assert_ne!(read, 0, "request ended before headers");
        bytes.extend_from_slice(&buffer[..read]);
    }
}

#[derive(Clone, Default)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl SharedWriter {
    fn contents(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

struct SharedWriterGuard(SharedWriter);

impl Write for SharedWriterGuard {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.0.lock().unwrap().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for SharedWriter {
    type Writer = SharedWriterGuard;

    fn make_writer(&'writer self) -> Self::Writer {
        SharedWriterGuard(self.clone())
    }
}

fn transport_trace_output() -> SharedWriter {
    static OUTPUT: OnceLock<SharedWriter> = OnceLock::new();
    OUTPUT
        .get_or_init(|| {
            let output = SharedWriter::default();
            let subscriber = tracing_subscriber::registry().with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_ansi(false)
                    .with_writer(output.clone())
                    .with_filter(filter_fn(|metadata| {
                        metadata.target() == "xui_rs::transport"
                    })),
            );
            tracing::subscriber::set_global_default(subscriber)
                .expect("transport test installs the only global subscriber");
            output
        })
        .clone()
}
