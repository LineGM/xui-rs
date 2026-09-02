#![allow(missing_docs)]

use reqwest::Method;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    BulkAdjustRequest, BulkFlowAdjustment, Client, ClientConfig, ClientCreateRequest,
    ClientExternalLinkInput, ClientExternalLinkKind, ClientPageRequest, ClientSort,
    ClientStatusFilter, Error, InboundProtocol, SortOrder,
};

fn record_json(email: &str) -> Value {
    json!({
        "id": 1,
        "email": email,
        "subId": "subscription-secret",
        "uuid": "protocol-secret",
        "password": "password-secret",
        "auth": "auth-secret",
        "flow": "xtls-rprx-vision",
        "security": "auto",
        "reverse": null,
        "privateKey": "private-secret",
        "publicKey": "public-key",
        "allowedIPs": "10.0.0.2/32",
        "preSharedKey": "psk-secret",
        "keepAlive": 25,
        "secret": "mtproto-secret",
        "adTag": "ad-secret",
        "limitIp": 2,
        "totalGB": 1000,
        "expiryTime": 2000,
        "enable": true,
        "tgId": 123,
        "group": "tier-a",
        "comment": "managed",
        "reset": 30,
        "createdAt": 100,
        "updatedAt": 200
    })
}

fn details_json(email: &str) -> Value {
    json!({
        "client": record_json(email),
        "inboundIds": [7],
        "externalLinks": [],
        "usedTraffic": 30
    })
}

fn traffic_json(email: &str) -> Value {
    json!({
        "id": 1,
        "inboundId": 7,
        "enable": true,
        "email": email,
        "uuid": "protocol-secret",
        "subId": "subscription-secret",
        "up": 10,
        "down": 20,
        "expiryTime": 2000,
        "total": 1000,
        "reset": 30,
        "lastOnline": 300
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
#[allow(clippy::too_many_lines)]
async fn every_v370_client_and_group_route_is_wired() {
    let server = MockServer::start().await;
    let page = json!({
        "items": [],
        "total": 1,
        "filtered": 1,
        "page": 1,
        "pageSize": 25,
        "summary": {"total": 1, "active": 1},
        "groups": ["tier-a"]
    });
    let portable = json!([{
        "client": {"email": "alice", "security": "auto", "enable": true},
        "inboundIds": [7]
    }]);
    let routes = [
        (
            Method::GET,
            "/panel/api/clients/list",
            Some(json!([{
                "id": 1,
                "email": "alice",
                "inboundIds": [7],
                "traffic": traffic_json("alice")
            }])),
        ),
        (Method::GET, "/panel/api/clients/list/paged", Some(page)),
        (
            Method::GET,
            "/panel/api/clients/get/alice",
            Some(details_json("alice")),
        ),
        (
            Method::GET,
            "/panel/api/clients/get/tgId/123",
            Some(json!([details_json("alice")])),
        ),
        (
            Method::GET,
            "/panel/api/clients/traffic/alice",
            Some(traffic_json("alice")),
        ),
        (
            Method::GET,
            "/panel/api/clients/subLinks/sub",
            Some(json!(["vless://subscription-link"])),
        ),
        (
            Method::GET,
            "/panel/api/clients/links/alice",
            Some(json!(["vless://client-link"])),
        ),
        (
            Method::POST,
            "/panel/api/clients/add",
            Some(json!({"nodePending": true})),
        ),
        (
            Method::POST,
            "/panel/api/clients/update/alice",
            Some(json!({"nodePending": false})),
        ),
        (Method::POST, "/panel/api/clients/del/alice", None),
        (
            Method::POST,
            "/panel/api/clients/alice/attach",
            Some(json!({"nodePending": true})),
        ),
        (
            Method::POST,
            "/panel/api/clients/alice/detach",
            Some(json!({"nodePending": false})),
        ),
        (Method::POST, "/panel/api/clients/alice/externalLinks", None),
        (Method::GET, "/panel/api/clients/export", Some(portable)),
        (
            Method::POST,
            "/panel/api/clients/import",
            Some(json!({"created": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/delOrphans",
            Some(json!({"deleted": 0})),
        ),
        (Method::POST, "/panel/api/clients/resetAllTraffics", None),
        (
            Method::POST,
            "/panel/api/clients/delDepleted",
            Some(json!({"deleted": 0})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkAdjust",
            Some(json!({"adjusted": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkEnable",
            Some(json!({"changed": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkDisable",
            Some(json!({"changed": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkDel",
            Some(json!({"deleted": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkCreate",
            Some(json!({"created": 1, "skipped": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkAttach",
            Some(json!({"attached": ["alice"], "skipped": [], "errors": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkDetach",
            Some(json!({"detached": ["alice"], "skipped": [], "errors": []})),
        ),
        (
            Method::POST,
            "/panel/api/clients/bulkResetTraffic",
            Some(json!({"affected": 1})),
        ),
        (Method::POST, "/panel/api/clients/resetTraffic/alice", None),
        (Method::POST, "/panel/api/clients/updateTraffic/alice", None),
        (
            Method::POST,
            "/panel/api/clients/ips/alice",
            Some(json!([{
                "ip": "203.0.113.10",
                "time": "2026-08-27 12:00:00",
                "node": "edge"
            }])),
        ),
        (Method::POST, "/panel/api/clients/clearIps/alice", None),
        (
            Method::POST,
            "/panel/api/clients/hwids/alice",
            Some(json!([{
                "id": 9,
                "firstSeen": 100,
                "lastSeen": 200,
                "userAgent": "Hiddify/2",
                "deviceOs": "Android",
                "osVersion": "15",
                "deviceModel": "Pixel"
            }])),
        ),
        (Method::DELETE, "/panel/api/clients/hwids/alice", None),
        (Method::DELETE, "/panel/api/clients/hwids/alice/9", None),
        (
            Method::POST,
            "/panel/api/clients/onlines",
            Some(json!(["alice"])),
        ),
        (
            Method::POST,
            "/panel/api/clients/onlinesByGuid",
            Some(json!({"panel-guid": ["alice"]})),
        ),
        (
            Method::POST,
            "/panel/api/clients/clientIpsByGuid",
            Some(json!({
                "panel-guid": {
                    "alice": [{"ip": "203.0.113.10", "timestamp": 100}]
                }
            })),
        ),
        (
            Method::POST,
            "/panel/api/clients/activeInbounds",
            Some(json!({"panel-guid": ["in-443-tcp"]})),
        ),
        (
            Method::POST,
            "/panel/api/clients/lastOnline",
            Some(json!({"alice": 300})),
        ),
        (
            Method::GET,
            "/panel/api/clients/groups",
            Some(json!([{
                "name": "tier-a",
                "clientCount": 1,
                "trafficUsed": 30,
                "up": 10,
                "down": 20
            }])),
        ),
        (
            Method::GET,
            "/panel/api/clients/groups/tier/emails",
            Some(json!(["alice"])),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/create",
            Some(json!({"name": "tier"})),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/rename",
            Some(json!({"affected": 1})),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/delete",
            Some(json!({"affected": 1})),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/resetTraffic",
            Some(json!({"name": "tier"})),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/bulkAdd",
            Some(json!({"affected": 1})),
        ),
        (
            Method::POST,
            "/panel/api/clients/groups/bulkRemove",
            Some(json!({"affected": 1})),
        ),
    ];
    assert_eq!(routes.len(), 46);
    for (method, path, object) in routes {
        mount_endpoint(&server, method, path, object).await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let emails = vec!["alice".to_owned()];
    let email_refs = ["alice"];
    let config = ClientConfig::new("alice");
    let create = ClientCreateRequest::new(config.clone(), vec![7]);

    assert_eq!(
        client.clients().list().await.unwrap()[0].client.email,
        "alice"
    );
    assert_eq!(
        client
            .clients()
            .list_paged(&ClientPageRequest::default())
            .await
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        client.clients().get("alice").await.unwrap().used_traffic,
        30
    );
    assert_eq!(
        client.clients().get_by_telegram_id(123).await.unwrap()[0]
            .client
            .email,
        "alice"
    );
    assert_eq!(client.clients().traffic("alice").await.unwrap().up, 10);
    assert_eq!(
        client.clients().subscription_links("sub").await.unwrap(),
        ["vless://subscription-link"]
    );
    assert_eq!(
        client.clients().links("alice").await.unwrap(),
        ["vless://client-link"]
    );
    assert!(client.clients().create(&create).await.unwrap().node_pending);
    assert!(
        !client
            .clients()
            .update_on_inbounds("alice", &config, &[7])
            .await
            .unwrap()
            .node_pending
    );
    client.clients().delete("alice", true).await.unwrap();
    assert!(
        client
            .clients()
            .attach("alice", &[7])
            .await
            .unwrap()
            .node_pending
    );
    assert!(
        !client
            .clients()
            .detach("alice", &[7])
            .await
            .unwrap()
            .node_pending
    );
    client
        .clients()
        .set_external_links(
            "alice",
            &[ClientExternalLinkInput {
                kind: ClientExternalLinkKind::Link,
                value: "vless://secret-link".to_owned(),
                remark: "edge".to_owned(),
            }],
        )
        .await
        .unwrap();
    assert_eq!(client.clients().export().await.unwrap()[0].inbound_ids, [7]);
    assert_eq!(
        client
            .clients()
            .import(std::slice::from_ref(&create))
            .await
            .unwrap()
            .created,
        1
    );
    assert_eq!(client.clients().delete_orphans().await.unwrap().deleted, 0);
    client.clients().reset_all_traffic().await.unwrap();
    assert_eq!(client.clients().delete_depleted().await.unwrap().deleted, 0);
    assert_eq!(
        client
            .clients()
            .bulk_adjust(&BulkAdjustRequest {
                emails: emails.clone(),
                add_days: 30,
                add_bytes: 1024,
                flow: BulkFlowAdjustment::Vision,
            })
            .await
            .unwrap()
            .adjusted,
        1
    );
    assert_eq!(
        client
            .clients()
            .bulk_enable(&email_refs)
            .await
            .unwrap()
            .changed,
        1
    );
    assert_eq!(
        client
            .clients()
            .bulk_disable(&email_refs)
            .await
            .unwrap()
            .changed,
        1
    );
    assert_eq!(
        client
            .clients()
            .bulk_delete(&email_refs, false)
            .await
            .unwrap()
            .deleted,
        1
    );
    assert_eq!(
        client
            .clients()
            .bulk_create(std::slice::from_ref(&create))
            .await
            .unwrap()
            .created,
        1
    );
    assert_eq!(
        client
            .clients()
            .bulk_attach(&email_refs, &[7])
            .await
            .unwrap()
            .attached,
        emails
    );
    assert_eq!(
        client
            .clients()
            .bulk_detach(&email_refs, &[7])
            .await
            .unwrap()
            .detached,
        emails
    );
    assert_eq!(
        client
            .clients()
            .bulk_reset_traffic(&email_refs)
            .await
            .unwrap()
            .affected,
        1
    );
    client.clients().reset_traffic("alice").await.unwrap();
    client
        .clients()
        .update_traffic("alice", 10, 20)
        .await
        .unwrap();
    assert_eq!(client.clients().ips("alice").await.unwrap()[0].node, "edge");
    client.clients().clear_ips("alice").await.unwrap();
    assert_eq!(
        client.clients().hwid_devices("alice").await.unwrap()[0].device_os,
        "Android"
    );
    client.clients().clear_hwid_devices("alice").await.unwrap();
    client
        .clients()
        .delete_hwid_device("alice", 9)
        .await
        .unwrap();
    assert_eq!(client.clients().onlines().await.unwrap(), emails);
    assert_eq!(
        client.clients().onlines_by_guid().await.unwrap()["panel-guid"],
        emails
    );
    assert_eq!(
        client.clients().client_ips_by_guid().await.unwrap()["panel-guid"]["alice"][0].ip,
        "203.0.113.10"
    );
    assert_eq!(
        client.clients().active_inbounds_by_guid().await.unwrap()["panel-guid"],
        ["in-443-tcp"]
    );
    assert_eq!(client.clients().last_online().await.unwrap()["alice"], 300);
    assert_eq!(client.clients().groups().await.unwrap()[0].traffic_used, 30);
    assert_eq!(client.clients().group_emails("tier").await.unwrap(), emails);
    assert_eq!(
        client.clients().create_group("tier").await.unwrap().name,
        "tier"
    );
    assert_eq!(
        client
            .clients()
            .rename_group("tier", "new-tier")
            .await
            .unwrap()
            .affected,
        1
    );
    assert_eq!(
        client
            .clients()
            .delete_group("tier")
            .await
            .unwrap()
            .affected,
        1
    );
    assert_eq!(
        client
            .clients()
            .reset_group_traffic("tier")
            .await
            .unwrap()
            .name,
        "tier"
    );
    assert_eq!(
        client
            .clients()
            .add_to_group(&email_refs, "tier")
            .await
            .unwrap()
            .affected,
        1
    );
    assert_eq!(
        client
            .clients()
            .remove_from_group(&email_refs)
            .await
            .unwrap()
            .affected,
        1
    );

    let requests = server.received_requests().await.unwrap();
    let scoped_update = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/clients/update/alice")
        .unwrap();
    assert!(
        scoped_update
            .url
            .query_pairs()
            .any(|(name, value)| { name == "inboundIds" && value == "7" })
    );
}

#[tokio::test]
async fn paged_update_delete_and_import_use_exact_wire_formats() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/clients/list/paged"))
        .and(matchers::query_param("filter", "online,expiring"))
        .and(matchers::query_param("protocol", "vless"))
        .and(matchers::query_param("inbound", "7,9"))
        .and(matchers::query_param("sort", "lastOnline"))
        .and(matchers::query_param("order", "descend"))
        .and(matchers::query_param("autoRenew", "on"))
        .and(matchers::query_param("hasTgId", "no"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "obj": {"items": [], "summary": {}}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/clients/update/alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/clients/del/alice"))
        .and(matchers::query_param("keepTraffic", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/clients/import"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "obj": {"created": 1}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let page = ClientPageRequest {
        statuses: vec![ClientStatusFilter::Online, ClientStatusFilter::Expiring],
        protocols: vec![InboundProtocol::Vless],
        inbound_ids: vec![7, 9],
        sort: Some(ClientSort::LastOnline),
        order: SortOrder::Descending,
        auto_renew: Some(true),
        has_telegram_id: Some(false),
        ..ClientPageRequest::default()
    };
    client.clients().list_paged(&page).await.unwrap();
    let mut config = ClientConfig::new("alice");
    config.total_gb = 4096;
    client.clients().update("alice", &config).await.unwrap();
    client.clients().delete("alice", true).await.unwrap();
    client
        .clients()
        .import(&[ClientCreateRequest::new(config, vec![7])])
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let update = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/clients/update/alice")
        .unwrap();
    let update_body: Value = serde_json::from_slice(&update.body).unwrap();
    assert_eq!(update_body["email"], "alice");
    assert_eq!(update_body["enable"], true);
    assert_eq!(update_body["totalGB"], 4096);

    let import = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/clients/import")
        .unwrap();
    let import_body: Value = serde_json::from_slice(&import.body).unwrap();
    let nested: Value = serde_json::from_str(import_body["data"].as_str().unwrap()).unwrap();
    assert_eq!(nested[0]["client"]["email"], "alice");
    assert_eq!(nested[0]["client"]["totalGB"], 4096);
    assert_eq!(nested[0]["inboundIds"], json!([7]));
}

#[tokio::test]
async fn path_values_are_encoded_as_single_segments() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path(
            "/panel/api/clients/get/alice%2Bops%40example%2Ecom%2Fadmin",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "obj": details_json("alice+ops@example.com/admin")
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();

    let details = client
        .clients()
        .get("alice+ops@example.com/admin")
        .await
        .unwrap();

    assert_eq!(details.client.email, "alice+ops@example.com/admin");
}

#[tokio::test]
async fn client_mutation_failures_keep_operation_context() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/clients/add"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "email already in use"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let request = ClientCreateRequest::new(ClientConfig::new("alice"), vec![7]);

    let error = client.clients().create(&request).await.unwrap_err();

    assert!(matches!(
        error,
        Error::Api { method, url, message }
            if method == Method::POST
                && url.path() == "/panel/api/clients/add"
                && message == "email already in use"
    ));
}
