#![allow(missing_docs)]

use reqwest::Method;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, ClientTrafficUsage, Error, FallbackInput, InboundConfig, InboundProtocol,
    TrafficPushRequest,
};

fn inbound_json(id: i64) -> Value {
    json!({
        "id": id,
        "up": 10,
        "down": 20,
        "total": 0,
        "remark": "primary",
        "subSortIndex": 1,
        "enable": true,
        "expiryTime": 0,
        "trafficReset": "never",
        "trafficResetDay": 1,
        "lastTrafficResetTime": 0,
        "clientStats": [],
        "listen": "",
        "port": 443,
        "protocol": "vless",
        "settings": {"clients": []},
        "streamSettings": {"network": "tcp"},
        "tag": "in-443-tcp",
        "sniffing": {"enabled": true},
        "shareAddrStrategy": "node",
        "shareAddr": ""
    })
}

async fn mount_endpoint(server: &MockServer, method: Method, path: &str, object: Option<Value>) {
    let mut body = json!({"success": true, "msg": "success"});
    if let Some(object) = object {
        body["obj"] = object;
    }
    Mock::given(matchers::method(method.as_str()))
        .and(matchers::path(path))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn empty_all_links_normalizes_the_upstream_null_slice() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/inbounds/allLinks"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "msg": "success",
            "obj": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    assert!(client.inbounds().all_links().await.unwrap().is_empty());
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn every_v370_inbound_route_is_wired() {
    let server = MockServer::start().await;
    let routes = [
        (
            Method::GET,
            "/panel/api/inbounds/list",
            Some(json!([inbound_json(7)])),
        ),
        (
            Method::GET,
            "/panel/api/inbounds/list/slim",
            Some(json!([inbound_json(7)])),
        ),
        (
            Method::GET,
            "/panel/api/inbounds/options",
            Some(json!([{
                "id": 7,
                "remark": "primary",
                "tag": "in-443-tcp",
                "protocol": "vless",
                "port": 443,
                "enable": true,
                "tlsFlowCapable": true,
                "ssMethod": ""
            }])),
        ),
        (
            Method::GET,
            "/panel/api/inbounds/allLinks",
            Some(json!(["vless://link"])),
        ),
        (
            Method::GET,
            "/panel/api/inbounds/get/7",
            Some(inbound_json(7)),
        ),
        (
            Method::GET,
            "/panel/api/inbounds/7/fallbacks",
            Some(json!([{
                "id": 1,
                "masterId": 7,
                "childId": 8,
                "name": "",
                "alpn": "h2",
                "path": "/fallback",
                "dest": "",
                "xver": 2,
                "sortOrder": 0
            }])),
        ),
        (
            Method::POST,
            "/panel/api/inbounds/add",
            Some(inbound_json(7)),
        ),
        (Method::POST, "/panel/api/inbounds/del/7", Some(json!(7))),
        (
            Method::POST,
            "/panel/api/inbounds/bulkDel",
            Some(json!({"deleted": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/inbounds/update/7",
            Some(inbound_json(7)),
        ),
        (Method::POST, "/panel/api/inbounds/setEnable/7", None),
        (Method::POST, "/panel/api/inbounds/7/subSortIndex", None),
        (Method::POST, "/panel/api/inbounds/7/resetTraffic", None),
        (
            Method::POST,
            "/panel/api/inbounds/7/delAllClients",
            Some(json!({"deleted": 2, "skipped": []})),
        ),
        (Method::POST, "/panel/api/inbounds/resetAllTraffics", None),
        (
            Method::POST,
            "/panel/api/inbounds/import",
            Some(inbound_json(8)),
        ),
        (Method::POST, "/panel/api/inbounds/7/fallbacks", None),
        (Method::POST, "/panel/api/inbounds/pushClientTraffics", None),
    ];
    for (method, path, object) in routes {
        mount_endpoint(&server, method, path, object).await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let mut config = InboundConfig::new(InboundProtocol::Vless, 443);
    config.remark = "primary".to_owned();
    config.settings = json!({"clients": []});

    assert_eq!(client.inbounds().list().await.unwrap()[0].id, 7);
    assert_eq!(client.inbounds().list_slim().await.unwrap()[0].id, 7);
    assert!(client.inbounds().options().await.unwrap()[0].tls_flow_capable);
    assert_eq!(
        client.inbounds().all_links().await.unwrap(),
        ["vless://link"]
    );
    assert_eq!(client.inbounds().get(7).await.unwrap().id, 7);
    assert_eq!(client.inbounds().fallbacks(7).await.unwrap()[0].child_id, 8);
    assert_eq!(client.inbounds().create(&config).await.unwrap().id, 7);
    assert_eq!(client.inbounds().delete(7).await.unwrap(), 7);
    assert_eq!(
        client.inbounds().delete_many(&[7]).await.unwrap().deleted,
        1
    );
    assert_eq!(client.inbounds().update(7, &config).await.unwrap().id, 7);
    client.inbounds().set_enabled(7, false).await.unwrap();
    client
        .inbounds()
        .set_subscription_sort_index(7, 2)
        .await
        .unwrap();
    client.inbounds().reset_traffic(7).await.unwrap();
    assert_eq!(
        client
            .inbounds()
            .delete_all_clients(7)
            .await
            .unwrap()
            .deleted,
        2
    );
    client.inbounds().reset_all_traffic().await.unwrap();
    assert_eq!(client.inbounds().import(&config).await.unwrap().id, 8);
    client
        .inbounds()
        .set_fallbacks(
            7,
            &[FallbackInput {
                child_id: 8,
                path: "/fallback".to_owned(),
                xver: 2,
                ..FallbackInput::default()
            }],
        )
        .await
        .unwrap();
    client
        .inbounds()
        .push_client_traffic(&TrafficPushRequest::new(
            "master-guid",
            vec![ClientTrafficUsage {
                email: "client@example.com".to_owned(),
                up: 10,
                down: 20,
            }],
        ))
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let import = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/inbounds/import")
        .unwrap();
    assert_eq!(
        import.headers["content-type"].to_str().unwrap(),
        "application/x-www-form-urlencoded"
    );
    assert!(String::from_utf8_lossy(&import.body).starts_with("data=%7B"));
}

#[tokio::test]
async fn mutating_inbound_requests_send_expected_wire_payloads() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/inbounds/setEnable/9"))
        .and(matchers::body_json(json!({"enable": false})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/inbounds/bulkDel"))
        .and(matchers::body_json(json!({"ids": [9, 10]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "obj": {"deleted": 2}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/inbounds/pushClientTraffics"))
        .and(matchers::body_json(json!({
            "masterGuid": "master-guid",
            "traffics": [{"email": "alice", "up": 100, "down": 200}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    client.inbounds().set_enabled(9, false).await.unwrap();
    assert_eq!(
        client
            .inbounds()
            .delete_many(&[9, 10])
            .await
            .unwrap()
            .deleted,
        2
    );
    client
        .inbounds()
        .push_client_traffic(&TrafficPushRequest::new(
            "master-guid",
            vec![ClientTrafficUsage {
                email: "alice".to_owned(),
                up: 100,
                down: 200,
            }],
        ))
        .await
        .unwrap();
}

#[tokio::test]
async fn inbound_api_failures_keep_operation_context() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/inbounds/del/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "inbound is attached to a node"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();

    let error = client.inbounds().delete(42).await.unwrap_err();

    assert!(matches!(
        error,
        Error::Api { method, url, message }
            if method == Method::POST
                && url.path() == "/panel/api/inbounds/del/42"
                && message == "inbound is attached to a node"
    ));
}
