#![allow(missing_docs)]

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Method, StatusCode};
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{Error, SubscriptionClient, SubscriptionSettings};

fn response(body: impl Into<Vec<u8>>, content_type: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_bytes(body)
        .insert_header("content-type", content_type)
        .insert_header(
            "subscription-userinfo",
            "upload=100; download=200; total=1000; expire=1700000000",
        )
        .insert_header("profile-title", "base64:TXkgUHJvZmlsZQ==")
        .insert_header("profile-update-interval", "10")
        .insert_header("profile-web-page-url", "https://sub.example/sub/secret/id")
        .insert_header("support-url", "https://support.example")
        .insert_header("announce", "base64:SGVsbG8=")
        .insert_header("routing-enable", "true")
        .insert_header("routing", "private-routing-rules")
        .insert_header("hide-settings", "1")
}

async fn mount(
    server: &MockServer,
    method: &str,
    path: &str,
    query: Option<(&str, &str)>,
    template: ResponseTemplate,
) {
    let mut mock = Mock::given(matchers::method(method)).and(matchers::path(path));
    if let Some((key, value)) = query {
        mock = mock.and(matchers::query_param(key, value));
    }
    mock.respond_with(template).expect(1).mount(server).await;
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn all_six_v360_subscription_routes_are_typed_and_secret_safe() {
    let server = MockServer::start().await;
    let private_link = "vless://private-client-credential@example.com:443\n";
    let encoded = STANDARD.encode(private_link);
    mount(
        &server,
        "GET",
        "/sub/secret%2Fid",
        None,
        response(encoded, "text/plain; charset=utf-8"),
    )
    .await;
    mount(
        &server,
        "HEAD",
        "/sub/secret%2Fid",
        None,
        response(Vec::new(), "text/plain; charset=utf-8"),
    )
    .await;
    mount(
        &server,
        "GET",
        "/json/secret%2Fid",
        Some(("view", "raw")),
        response(
            json!({
                "outbounds": [{
                    "protocol": "vless",
                    "settings": {"vnext": [{"users": [{"id": "private-json-id"}]}]}
                }]
            })
            .to_string(),
            "application/json; charset=utf-8",
        )
        .insert_header(
            "content-disposition",
            "attachment; filename=\"subscription.json\"",
        ),
    )
    .await;
    mount(
        &server,
        "HEAD",
        "/json/secret%2Fid",
        None,
        response(Vec::new(), "text/plain; charset=utf-8"),
    )
    .await;
    mount(
        &server,
        "GET",
        "/clash/secret%2Fid",
        Some(("view", "raw")),
        response(
            "proxies:\n  - name: edge\n    password: private-clash-password\n",
            "application/yaml; charset=utf-8",
        )
        .insert_header(
            "content-disposition",
            "attachment; filename=\"subscription.yaml\"",
        ),
    )
    .await;
    mount(
        &server,
        "HEAD",
        "/clash/secret%2Fid",
        None,
        response(Vec::new(), "application/yaml; charset=utf-8"),
    )
    .await;

    let client = SubscriptionClient::new(server.uri()).unwrap();
    let raw = client.raw("secret/id").await.unwrap();
    let decoded = raw.content.decode_base64().unwrap();
    assert_eq!(decoded.lines().collect::<Vec<_>>(), [private_link.trim()]);
    assert_eq!(raw.metadata.traffic.unwrap().download, 200);
    assert_eq!(raw.metadata.profile_title.as_deref(), Some("My Profile"));
    assert_eq!(raw.metadata.announcement.as_deref(), Some("Hello"));
    assert_eq!(raw.metadata.update_interval_minutes, Some(10));
    assert!(raw.metadata.routing_enabled);
    assert!(raw.metadata.hide_settings);
    assert_eq!(
        raw.metadata.profile_web_page_url(),
        Some("https://sub.example/sub/secret/id")
    );
    assert_eq!(raw.metadata.routing_rules(), Some("private-routing-rules"));
    let raw_debug = format!("{raw:?}");
    assert!(!raw_debug.contains("private-client-credential"));
    assert!(!raw_debug.contains("secret/id"));
    assert!(!raw_debug.contains("private-routing-rules"));

    let json = client.json("secret/id").await.unwrap();
    assert_eq!(json.content.as_value()["outbounds"][0]["protocol"], "vless");
    assert!(!format!("{json:?}").contains("private-json-id"));
    assert_eq!(
        json.metadata.content_disposition.as_deref(),
        Some("attachment; filename=\"subscription.json\"")
    );

    let clash = client.clash("secret/id").await.unwrap();
    assert!(clash.content.as_str().contains("proxies:"));
    assert!(!format!("{clash:?}").contains("private-clash-password"));

    assert_eq!(
        client
            .raw_metadata("secret/id")
            .await
            .unwrap()
            .traffic
            .unwrap()
            .upload,
        100
    );
    assert_eq!(
        client
            .json_metadata("secret/id")
            .await
            .unwrap()
            .profile_title
            .as_deref(),
        Some("My Profile")
    );
    assert!(
        client
            .clash_metadata("secret/id")
            .await
            .unwrap()
            .routing_enabled
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 6);
    assert!(requests.iter().all(|request| {
        request.headers.get("authorization").is_none() && request.headers.get("cookie").is_none()
    }));
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == Method::HEAD)
            .count(),
        3
    );
}

#[tokio::test]
async fn info_html_and_custom_paths_cover_content_negotiation() {
    let server = MockServer::start().await;
    mount(
        &server,
        "GET",
        "/custom/raw/private%2Finfo",
        Some(("format", "info")),
        ResponseTemplate::new(200).set_body_json(json!({
            "sId": "private/info",
            "enabled": true,
            "isOnline": false,
            "download": "200 B",
            "upload": "100 B",
            "total": "1000 B",
            "used": "300 B",
            "remained": "700 B",
            "expire": 1_700_000_000,
            "lastOnline": 1_700_000_000_000_i64,
            "downloadByte": 200,
            "uploadByte": 100,
            "totalByte": 1000,
            "subUrl": "https://sub.example/custom/raw/private/info",
            "subJsonUrl": "https://sub.example/custom/json/private/info",
            "subClashUrl": "https://sub.example/custom/clash/private/info",
            "subTitle": "Private",
            "subSupportUrl": "https://support.example",
            "emails": ["private@example.com"],
            "datepicker": "gregorian",
            "announce": "hello"
        })),
    )
    .await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/custom/raw/private%2Fhtml"))
        .and(matchers::header("accept", "text/html"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html; charset=utf-8")
                .set_body_string("<html>private-subscription-page</html>"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = SubscriptionClient::builder(server.uri())
        .unwrap()
        .raw_path("custom/raw/")
        .json_path("custom/json/")
        .clash_path("custom/clash/")
        .build()
        .unwrap();
    let info = client.info("private/info").await.unwrap();
    assert_eq!(info.download_byte, 200);
    let debug = format!("{info:?}");
    assert!(!debug.contains("private/info"));
    assert!(!debug.contains("private@example.com"));

    let html = client.html("private/html").await.unwrap();
    assert!(html.content.as_str().starts_with("<html>"));
    assert!(!format!("{html:?}").contains("private-subscription-page"));
}

#[tokio::test]
async fn subscription_errors_and_client_debug_redact_secret_paths() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/sub/private%2Fsubscription-id"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let client = SubscriptionClient::new(server.uri()).unwrap();
    let error = client.raw("private/subscription-id").await.unwrap_err();
    assert!(matches!(
        &error,
        Error::HttpStatus { method, url, status }
            if *method == Method::GET
                && *status == StatusCode::NOT_FOUND
                && !url.as_str().contains("private")
                && !url.as_str().contains("subscription-id")
    ));
    assert!(!error.to_string().contains("private/subscription-id"));
    assert!(!format!("{client:?}").contains("/sub/"));
    assert!(client.raw("").await.is_err());
}

#[test]
fn settings_constructor_uses_public_uris_without_debug_leaks() {
    let settings = SubscriptionSettings {
        sub_uri: "https://sub.example/private-raw/".into(),
        sub_json_uri: "https://sub.example/private-json/".into(),
        sub_clash_uri: "https://sub.example/private-clash/".into(),
        ..SubscriptionSettings::default()
    };
    let client = SubscriptionClient::from_settings(&settings).unwrap();
    let debug = format!("{client:?}");
    assert!(debug.contains("https://sub.example"));
    assert!(!debug.contains("private-raw"));
    assert!(!debug.contains("private-json"));
    assert!(!debug.contains("private-clash"));

    let fallback_paths = SubscriptionSettings {
        sub_uri: "https://sub.example/private-raw/".into(),
        sub_json_path: "/custom-json/".into(),
        sub_clash_path: "/custom-clash/".into(),
        ..SubscriptionSettings::default()
    };
    let fallback_client = SubscriptionClient::from_settings(&fallback_paths).unwrap();
    let fallback_debug = format!("{fallback_client:?}");
    assert!(fallback_debug.contains("https://sub.example"));
    assert!(!fallback_debug.contains("custom-json"));
    assert!(!fallback_debug.contains("custom-clash"));
}
