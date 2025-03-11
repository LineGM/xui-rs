use httpmock::prelude::*;
use serde_json::json;
use xui_rs::api::XUiClient;
use xui_rs::errors::MyError;

// Helper function to set up a mock server
fn setup_mock_server() -> MockServer {
    MockServer::start()
}

#[tokio::test]
async fn test_client_creation() {
    // Test valid URL
    let client = XUiClient::new("https://valid-panel.com/");
    assert!(client.is_ok());

    // Test with trailing slash
    let client = XUiClient::new("https://valid-panel.com/path/");
    assert!(client.is_ok());

    // Test without trailing slash (should still work but will trim the last segment)
    let client = XUiClient::new("https://valid-panel.com/path");
    assert!(client.is_ok());

    // Test invalid URL (this should be handled by reqwest)
    let client = XUiClient::new("not-a-valid-url");
    assert!(client.is_err());
}

#[tokio::test]
async fn test_login_success() {
    let server = setup_mock_server();

    // Mock successful login response
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/").json_body(json!({
            "username": "test_user",
            "password": "test_pass"
        }));
        then.status(200)
            .header("content-type", "application/json")
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/")
            .json_body(json!({ "success": true }));
    });

    // Create client with mock server URL
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let result = client.login("test_user", "test_pass").await;

    // Assert login was successful
    assert!(result.is_ok());

    // Verify the mock was called
    login_mock.assert();

    // Check if cookie was properly stored in client (using internal method)
    // assert!(client.is_cookie_valid());
}

#[tokio::test]
async fn test_login_failure() {
    let server = setup_mock_server();

    // Mock failed login response
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(401)
            .header("content-type", "application/json")
            .json_body(json!({ "error": "Invalid credentials" }));
    });

    // Create client with mock server URL
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let result = client.login("wrong_user", "wrong_pass").await;

    // Assert login failed
    assert!(result.is_err());

    // Verify the mock was called
    login_mock.assert();
}

#[tokio::test]
async fn test_get_inbounds() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock inbounds list endpoint
    let inbounds_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/panel/api/inbounds/list/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": [
                {"id": 1, "protocol": "vmess", "remark": "Test Inbound"},
                {"id": 2, "protocol": "vless", "remark": "Another Inbound"}
            ]
        }));
    });

    // Create client, login, and get inbounds
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let inbounds = client.get_inbounds().await;

    // Verify response
    assert!(inbounds.is_ok());
    let inbounds_data = inbounds.unwrap();
    assert!(inbounds_data["success"].as_bool().unwrap());
    assert_eq!(inbounds_data["obj"].as_array().unwrap().len(), 2);

    // Verify mocks were called
    login_mock.assert();
    inbounds_mock.assert();
}

#[tokio::test]
async fn test_get_single_inbound() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock get single inbound endpoint
    let inbound_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/panel/api/inbounds/get/1/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": {"id": 1, "protocol": "vmess", "remark": "Test Inbound"}
        }));
    });

    // Create client, login, and get specific inbound
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let inbound = client.get_inbound(1_u64).await;

    // Verify response
    assert!(inbound.is_ok());
    let inbound_data = inbound.unwrap();
    assert!(inbound_data["success"].as_bool().unwrap());
    assert_eq!(inbound_data["obj"]["id"].as_i64().unwrap(), 1);

    // Verify mocks were called
    login_mock.assert();
    inbound_mock.assert();
}

#[tokio::test]
async fn test_get_client_traffic_by_email() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock get traffic by email endpoint
    let traffic_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/panel/api/inbounds/getClientTraffics/user@example.com/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": {
                "email": "user@example.com",
                "up": 1024,
                "down": 2048
            }
        }));
    });

    // Create client, login, and get traffic by email
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let traffic = client.get_client_traffic_by_email("user@example.com").await;

    // Verify response
    assert!(traffic.is_ok());
    let traffic_data = traffic.unwrap();
    assert!(traffic_data["success"].as_bool().unwrap());
    assert_eq!(
        traffic_data["obj"]["email"].as_str().unwrap(),
        "user@example.com"
    );

    // Verify mocks were called
    login_mock.assert();
    traffic_mock.assert();
}

#[tokio::test]
async fn test_get_client_traffic_by_uuid() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock get traffic by UUID endpoint
    let uuid = "d7c06399-a3e3-4007-9109-19012597dd01";
    let traffic_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!(
                "/panel/api/inbounds/getClientTrafficsById/{}/",
                uuid
            ))
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": {
                "uuid": uuid,
                "up": 1024,
                "down": 2048
            }
        }));
    });

    // Create client, login, and get traffic by UUID
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let traffic = client.get_client_traffic_by_uuid(uuid).await;

    // Verify response
    assert!(traffic.is_ok());
    let traffic_data = traffic.unwrap();
    assert!(traffic_data["success"].as_bool().unwrap());
    assert_eq!(traffic_data["obj"]["uuid"].as_str().unwrap(), uuid);

    // Verify mocks were called
    login_mock.assert();
    traffic_mock.assert();
}

#[tokio::test]
async fn test_get_backup() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock backup endpoint
    let backup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/panel/api/inbounds/createbackup/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200);
    });

    // Create client, login, and create backup
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let backup = client.get_backup().await;

    // Verify response
    assert!(backup.is_ok());
    assert_eq!(backup.unwrap(), 200);

    // Verify mocks were called
    login_mock.assert();
    backup_mock.assert();
}

#[tokio::test]
async fn test_auto_relogin_on_expired_cookie() {
    let server = setup_mock_server();

    // Mock for the initial login
    let mut login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/").json_body(json!({
            "username": "test_user",
            "password": "test_pass"
        }));
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=0; Path=/");
    });

    // Create client and do initial login
    let mut client = XUiClient::new(server.url("/")).unwrap();
    client.login("test_user", "test_pass").await.unwrap();

    // Verify initial login was called
    login_mock.assert();
    login_mock.delete(); // Delete the mock to avoid conflicts with relogin

    // Set up a mock for the re-login
    let relogin_mock = server.mock(|when, then| {
        when.method(POST).path("/login/").json_body(json!({
            "username": "test_user",
            "password": "test_pass"
        }));
        then.status(200).header(
            "set-cookie",
            "session=new-test-cookie; Max-Age=3600; Path=/",
        );
    });

    // Mock inbounds endpoint that will be called after re-login
    let inbounds_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/panel/api/inbounds/list/")
            .header("cookie", "session=new-test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({ "success": true }));
    });

    // Call an endpoint that should trigger re-login due to expired cookie
    // The cookie expires immediately because we set Max-Age=0
    let inbounds = client.get_inbounds().await;
    assert!(inbounds.is_ok());

    // Verify the re-login and subsequent API call mocks were called
    relogin_mock.assert();
    inbounds_mock.assert();
}

#[tokio::test]
async fn test_ensure_authenticated_no_credentials() {
    let server = setup_mock_server();

    // Create client without login
    let mut client = XUiClient::new(server.url("/")).unwrap();

    // Try to get inbounds without having logged in first
    let result = client.get_inbounds().await;

    // Should fail with authentication error
    assert!(result.is_err());
    match result {
        Err(MyError::CustomError(msg)) => {
            assert!(msg.contains("no credentials available"));
        }
        _ => panic!("Expected CustomError with credentials message"),
    }
}
