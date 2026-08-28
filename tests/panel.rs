#![allow(missing_docs)]

use reqwest::Method;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::Client;

#[tokio::test]
async fn panel_misc_routes_handle_direct_json_and_empty_success() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/openapi.json"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "openapi": "3.0.3",
            "info": {"title": "3x-ui API"},
            "paths": {
                "/panel/api/server/status": {
                    "get": {"operationId": "get_status"}
                },
                "→ type: status": {
                    "ws": {"operationId": "ws_type_status"}
                }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/backuptotgbot"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let document = client.panel().openapi().await.unwrap();
    assert_eq!(document.version(), Some("3.0.3"));
    assert_eq!(document.http_operation_count(), 1);
    assert!(!format!("{document:?}").contains("get_status"));
    client.panel().backup_to_telegram().await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let backup = requests
        .iter()
        .find(|request| request.method == Method::POST)
        .unwrap();
    assert!(backup.body.is_empty());
}
