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

#[tokio::test]
async fn test_get_client_ips() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock get client IPs endpoint
    let client_ips_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/clientIps/user@example.com/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": [
                {"ip": "192.168.1.1", "timestamp": 1661234567000_u64},
                {"ip": "10.0.0.1", "timestamp": 1661234568000_u64}
            ]
        }));
    });

    // Create client, login, and get client IPs
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let client_ips = client.get_client_ips("user@example.com").await;

    // Verify response
    assert!(client_ips.is_ok());
    let client_ips_data = client_ips.unwrap();
    assert!(client_ips_data["success"].as_bool().unwrap());
    assert_eq!(client_ips_data["obj"].as_array().unwrap().len(), 2);

    // Verify mocks were called
    login_mock.assert();
    client_ips_mock.assert();
}

#[tokio::test]
async fn test_add_inbound() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Define test inbound config
    let inbound_config = json!({
        "up": 0,
        "down": 0,
        "total": 0,
        "remark": "Test Inbound",
        "enable": true,
        "expiryTime": 0,
        "listen": "",
        "port": 10000,
        "protocol": "vmess",
        "settings": "{\"clients\":[{\"id\":\"b831381d-6324-4d53-ad4f-8cda48b30811\",\"alterId\":0,\"email\":\"test@example.com\",\"limitIp\":0,\"totalGB\":0,\"expiryTime\":0,\"enable\":true,\"tgId\":\"\",\"subId\":\"\"}],\"disableInsecureEncryption\":false}",
        "streamSettings": "{\"network\":\"tcp\",\"security\":\"none\",\"tcpSettings\":{\"acceptProxyProtocol\":false,\"header\":{\"type\":\"none\"}}}",
        "sniffing": "{\"enabled\":true,\"destOverride\":[\"http\",\"tls\"]}"
    });

    // Mock add inbound endpoint
    let add_inbound_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/add/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/")
            .json_body_partial(inbound_config.clone().to_string());
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Create Successfully",
            "obj": {"id": 3}
        }));
    });

    // Create client, login, and add inbound
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let add_result = client.add_inbound(inbound_config).await;

    // Verify response
    assert!(add_result.is_ok());
    let add_result_data = add_result.unwrap();
    assert!(add_result_data["success"].as_bool().unwrap());
    assert_eq!(add_result_data["obj"]["id"].as_i64().unwrap(), 3);

    // Verify mocks were called
    login_mock.assert();
    add_inbound_mock.assert();
}

#[tokio::test]
async fn test_add_client() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Define test client config
    let client_config = json!({
        "id": "bbfad557-28f2-47e5-9f3d-e3c7f532fbda",
        "flow": "",
        "email": "new_client@example.com",
        "limitIp": 0,
        "totalGB": 0,
        "expiryTime": 0,
        "enable": true,
        "tgId": "",
        "subId": "sub_id_here",
        "reset": 0
    });

    // Expected request structure when wrapped in the settings format
    let expected_request = json!({
        "id": 5,
        "settings": json!({
            "clients": [client_config.clone()]
        }).to_string()
    });

    // Mock add client endpoint
    let add_client_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/addClient/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/")
            .json_body_partial(expected_request.to_string());
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Client(s) added Successfully"
        }));
    });

    // Create client, login, and add client
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let add_result = client.add_client(5_u64, client_config).await;

    // Verify response
    assert!(add_result.is_ok());
    let add_result_data = add_result.unwrap();
    assert!(add_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    add_client_mock.assert();
}

#[tokio::test]
async fn test_update_inbound() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Define test inbound config update
    let updated_inbound_config = json!({
        "id": 4,
        "up": 0,
        "down": 0,
        "total": 0,
        "remark": "Updated Inbound",
        "enable": true,
        "expiryTime": 0,
        "listen": "",
        "port": 44360,
        "protocol": "vless",
        "settings": "{\"clients\":[{\"id\":\"b831381d-6324-4d53-ad4f-8cda48b30811\",\"flow\":\"\",\"email\":\"test@example.com\",\"limitIp\":0,\"totalGB\":0,\"expiryTime\":0,\"enable\":true}],\"decryption\":\"none\",\"fallbacks\":[]}",
        "streamSettings": "{\"network\":\"tcp\",\"security\":\"none\",\"tcpSettings\":{\"acceptProxyProtocol\":false,\"header\":{\"type\":\"none\"}}}",
        "sniffing": "{\"enabled\":true,\"destOverride\":[\"http\",\"tls\"]}"
    });

    // Mock update inbound endpoint
    let update_inbound_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/update/4/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/")
            .json_body_partial(updated_inbound_config.clone().to_string());
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Update Successfully"
        }));
    });

    // Create client, login, and update inbound
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let update_result = client.update_inbound(4_u64, updated_inbound_config).await;

    // Verify response
    assert!(update_result.is_ok());
    let update_result_data = update_result.unwrap();
    assert!(update_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    update_inbound_mock.assert();
}

#[tokio::test]
async fn test_update_client() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Define test client update
    let client_uuid = "95e4e7bb-7796-47e7-e8a7-f4055194f776";
    let updated_client = json!({
        "id": client_uuid,
        "flow": "",
        "email": "updated_client@example.com",
        "limitIp": 2,
        "totalGB": 42949672960_u64,
        "expiryTime": 1682864675944_u64,
        "enable": true,
        "tgId": "",
        "subId": "sub_id_here",
        "reset": 0
    });

    // Expected request structure when wrapped in the settings format
    let expected_request = json!({
        "id": 3,
        "settings": json!({
            "clients": [updated_client.clone()]
        }).to_string()
    });

    // Mock update client endpoint
    let update_client_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/panel/api/inbounds/updateClient/{}/", client_uuid))
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/")
            .json_body_partial(expected_request.to_string());
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Client updated Successfully"
        }));
    });

    // Create client, login, and update client
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let update_result = client
        .update_client(client_uuid, 3_u64, updated_client)
        .await;

    // Verify response
    assert!(update_result.is_ok());
    let update_result_data = update_result.unwrap();
    assert!(update_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    update_client_mock.assert();
}

#[tokio::test]
async fn test_clear_client_ips() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock clear client IPs endpoint
    let clear_ips_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/clearClientIps/user@example.com/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Log Cleared Successfully"
        }));
    });

    // Create client, login, and clear client IPs
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let clear_result = client.clear_client_ips("user@example.com").await;

    // Verify response
    assert!(clear_result.is_ok());
    let clear_result_data = clear_result.unwrap();
    assert!(clear_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    clear_ips_mock.assert();
}

#[tokio::test]
async fn test_reset_all_traffics() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock reset all traffics endpoint
    let reset_all_traffics_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/resetAllTraffics/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "all traffic has been reset Successfully"
        }));
    });

    // Create client, login, and reset all traffics
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let reset_result = client.reset_all_traffics().await;

    // Verify response
    assert!(reset_result.is_ok());
    let reset_result_data = reset_result.unwrap();
    assert!(reset_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    reset_all_traffics_mock.assert();
}

#[tokio::test]
async fn test_reset_all_client_traffics() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock reset all client traffics endpoint
    let reset_all_client_traffics_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/resetAllClientTraffics/3/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "All traffic from the client has been reset. Successfully"
        }));
    });

    // Create client, login, and reset all client traffics
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let reset_result = client.reset_all_client_traffics(3_u64).await;

    // Verify response
    assert!(reset_result.is_ok());
    let reset_result_data = reset_result.unwrap();
    assert!(reset_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    reset_all_client_traffics_mock.assert();
}

#[tokio::test]
async fn test_reset_client_traffic() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock reset client traffic endpoint
    let reset_client_traffic_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/3/resetClientTraffic/user@example.com/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Traffic has been reset Successfully"
        }));
    });

    // Create client, login, and reset client traffic
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let reset_result = client.reset_client_traffic(3_u64, "user@example.com").await;

    // Verify response
    assert!(reset_result.is_ok());
    let reset_result_data = reset_result.unwrap();
    assert!(reset_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    reset_client_traffic_mock.assert();
}

#[tokio::test]
async fn test_delete_client() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock delete client endpoint
    let client_uuid = "bf036995-a81d-41b3-8e06-8e233418c96a";
    let delete_client_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/panel/api/inbounds/3/delClient/{}/", client_uuid))
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Client deleted Successfully"
        }));
    });

    // Create client, login, and delete client
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let delete_result = client.delete_client(3_u64, client_uuid).await;

    // Verify response
    assert!(delete_result.is_ok());
    let delete_result_data = delete_result.unwrap();
    assert!(delete_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    delete_client_mock.assert();
}

#[tokio::test]
async fn test_delete_inbound() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock delete inbound endpoint
    let delete_inbound_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/del/3/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "Delete Successfully"
        }));
    });

    // Create client, login, and delete inbound
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let delete_result = client.delete_inbound(3_u64).await;

    // Verify response
    assert!(delete_result.is_ok());
    let delete_result_data = delete_result.unwrap();
    assert!(delete_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    delete_inbound_mock.assert();
}

#[tokio::test]
async fn test_delete_depleted_clients_specific_inbound() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock delete depleted clients from specific inbound endpoint
    let delete_depleted_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/delDepletedClients/4/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "All depleted clients are deleted Successfully"
        }));
    });

    // Create client, login, and delete depleted clients
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let delete_result = client.delete_depleted_clients(Some(4_u64)).await;

    // Verify response
    assert!(delete_result.is_ok());
    let delete_result_data = delete_result.unwrap();
    assert!(delete_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    delete_depleted_mock.assert();
}

#[tokio::test]
async fn test_delete_depleted_clients_all_inbounds() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock delete depleted clients from all inbounds endpoint
    let delete_depleted_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/delDepletedClients/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "msg": "All depleted clients are deleted Successfully"
        }));
    });

    // Create client, login, and delete depleted clients
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let delete_result = client.delete_depleted_clients(None::<u64>).await;

    // Verify response
    assert!(delete_result.is_ok());
    let delete_result_data = delete_result.unwrap();
    assert!(delete_result_data["success"].as_bool().unwrap());

    // Verify mocks were called
    login_mock.assert();
    delete_depleted_mock.assert();
}

#[tokio::test]
async fn test_get_online_clients() {
    let server = setup_mock_server();

    // Mock login endpoint
    let login_mock = server.mock(|when, then| {
        when.method(POST).path("/login/");
        then.status(200)
            .header("set-cookie", "session=test-cookie; Max-Age=3600; Path=/");
    });

    // Mock get online clients endpoint
    let online_clients_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/panel/api/inbounds/onlines/")
            .header("cookie", "session=test-cookie; Max-Age=3600; Path=/");
        then.status(200).json_body(json!({
            "success": true,
            "obj": [
                {
                    "email": "user1@example.com",
                    "inboundId": 1,
                    "ip": "192.168.1.1",
                    "up": 1024,
                    "down": 2048
                },
                {
                    "email": "user2@example.com",
                    "inboundId": 2,
                    "ip": "192.168.1.2",
                    "up": 3072,
                    "down": 4096
                }
            ]
        }));
    });

    // Create client, login, and get online clients
    let mut client = XUiClient::new(server.url("/")).unwrap();
    let _ = client.login("user", "pass").await;
    let online_clients = client.get_online_clients().await;

    // Verify response
    assert!(online_clients.is_ok());
    let online_clients_data = online_clients.unwrap();
    assert!(online_clients_data["success"].as_bool().unwrap());
    assert_eq!(online_clients_data["obj"].as_array().unwrap().len(), 2);

    // Verify mocks were called
    login_mock.assert();
    online_clients_mock.assert();
}
