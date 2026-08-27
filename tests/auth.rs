#![allow(missing_docs)]

use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{Client, Error, LoginRequest};

fn json_response(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

async fn mount_csrf(server: &MockServer, expected_calls: u64) {
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/secret/csrf-token"))
        .respond_with(
            json_response(serde_json::json!({
                "success": true,
                "obj": "csrf-value"
            }))
            .insert_header(
                "set-cookie",
                "3x-ui=preauth-cookie; Path=/secret/; HttpOnly; SameSite=Lax",
            ),
        )
        .expect(expected_calls)
        .mount(server)
        .await;
}

#[tokio::test]
async fn login_fetches_csrf_and_lets_cookie_jar_manage_the_session() {
    let server = MockServer::start().await;
    mount_csrf(&server, 1).await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/secret/login"))
        .and(matchers::header("x-csrf-token", "csrf-value"))
        .and(matchers::header("cookie", "3x-ui=preauth-cookie"))
        .and(matchers::body_json(serde_json::json!({
            "username": "admin",
            "password": "password",
            "twoFactorCode": "123456"
        })))
        .respond_with(
            json_response(serde_json::json!({
                "success": true,
                "msg": "Logged in successfully"
            }))
            .insert_header(
                "set-cookie",
                "3x-ui=authenticated-cookie; Path=/secret/; HttpOnly; SameSite=Lax",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/secret/logout"))
        .and(matchers::header("x-csrf-token", "csrf-value"))
        .and(matchers::header("cookie", "3x-ui=authenticated-cookie"))
        .respond_with(json_response(serde_json::json!({ "success": true })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(format!("{}/secret", server.uri())).unwrap();
    client
        .auth()
        .login(LoginRequest::new("admin", "password").with_two_factor_code("123456"))
        .await
        .unwrap();
    client.auth().logout().await.unwrap();
}

#[tokio::test]
async fn csrf_initialization_is_shared_between_concurrent_clones() {
    let server = MockServer::start().await;
    mount_csrf(&server, 1).await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/secret/getTwoFactorEnable"))
        .and(matchers::header("x-csrf-token", "csrf-value"))
        .and(matchers::header("cookie", "3x-ui=preauth-cookie"))
        .respond_with(json_response(serde_json::json!({
            "success": true,
            "obj": false
        })))
        .expect(2)
        .mount(&server)
        .await;

    let client = Client::new(format!("{}/secret/", server.uri())).unwrap();
    let clone = client.clone();
    let (first, second) = tokio::join!(
        client.auth().is_two_factor_enabled(),
        clone.auth().is_two_factor_enabled()
    );

    assert!(!first.unwrap());
    assert!(!second.unwrap());
}

#[tokio::test]
async fn panel_level_failure_is_a_typed_error() {
    let server = MockServer::start().await;
    mount_csrf(&server, 1).await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/secret/login"))
        .respond_with(json_response(serde_json::json!({
            "success": false,
            "msg": "Wrong username or password"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(format!("{}/secret", server.uri())).unwrap();
    let error = client
        .auth()
        .login(LoginRequest::new("admin", "wrong"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::Api { message, .. } if message == "Wrong username or password"
    ));
}

#[tokio::test]
async fn csrf_tokens_are_redacted_in_debug_output() {
    let server = MockServer::start().await;
    mount_csrf(&server, 1).await;
    let client = Client::new(format!("{}/secret", server.uri())).unwrap();

    let token = client.auth().csrf_token().await.unwrap();

    assert_eq!(token.expose(), "csrf-value");
    assert!(!format!("{token:?}").contains("csrf-value"));
}

#[tokio::test]
async fn forbidden_session_request_refreshes_csrf_and_retries_once() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let server = MockServer::start().await;
    mount_csrf(&server, 2).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let responder_calls = Arc::clone(&calls);
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/secret/getTwoFactorEnable"))
        .and(matchers::header("x-csrf-token", "csrf-value"))
        .respond_with(move |_: &wiremock::Request| {
            if responder_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                ResponseTemplate::new(403)
            } else {
                json_response(serde_json::json!({ "success": true, "obj": true }))
            }
        })
        .expect(2)
        .mount(&server)
        .await;

    let client = Client::new(format!("{}/secret", server.uri())).unwrap();

    assert!(client.auth().is_two_factor_enabled().await.unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
