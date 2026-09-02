#![allow(missing_docs)]

use std::collections::BTreeSet;

use reqwest::Method;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, Error, HistoryBucket, NodeInboundSyncMode, NodeMetric, NodeRequest, NodeScheme,
    NodeStatus, NodeTlsVerifyMode, NodeUpdateChannel, NodeView, RemoteInboundProtocol,
};

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

fn node_json() -> Value {
    json!({
        "id": 7,
        "name": "edge-de",
        "remark": "Frankfurt edge",
        "scheme": "https",
        "address": "node.example.com",
        "port": 2053,
        "basePath": "/admin/",
        "hasApiToken": true,
        "enable": true,
        "allowPrivateAddress": false,
        "tlsVerifyMode": "pin",
        "pinnedCertSha256": "sha256-base64-pin",
        "inboundSyncMode": "selected",
        "inboundTags": null,
        "outboundTag": "direct",
        "guid": "node-guid",
        "status": "online",
        "lastHeartbeat": 1_700_000_000,
        "latencyMs": 42,
        "xrayVersion": "25.10.31",
        "panelVersion": "v3.7.0",
        "cpuPct": 12.5,
        "memPct": 45.2,
        "uptimeSecs": 86_400,
        "netUp": 1_024,
        "netDown": 2_048,
        "lastError": "",
        "xrayState": "running",
        "xrayError": "",
        "configDirty": false,
        "configDirtyAt": 0,
        "inboundCount": 3,
        "clientCount": 25,
        "onlineCount": 5,
        "activeCount": 20,
        "disabledCount": 2,
        "depletedCount": 1,
        "parentGuid": "master-guid",
        "transitive": false,
        "createdAt": 1_700_000_000_000_i64,
        "updatedAt": 1_700_003_600_000_i64
    })
}

fn probe_json(status: &str) -> Value {
    json!({
        "status": status,
        "latencyMs": 42,
        "xrayVersion": "25.10.31",
        "panelVersion": "v3.7.0",
        "cpuPct": 12.5,
        "memPct": 45.2,
        "uptimeSecs": 86_400,
        "error": if status == "offline" { "connection refused" } else { "" },
        "xrayState": "running",
        "xrayError": ""
    })
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn every_v370_node_route_is_wired_with_exact_payloads() {
    let server = MockServer::start().await;
    let routes = [
        (
            Method::GET,
            "/panel/api/nodes/list",
            Some(json!([node_json()])),
        ),
        (Method::GET, "/panel/api/nodes/get/7", Some(node_json())),
        (
            Method::GET,
            "/panel/api/nodes/webCert/7",
            Some(json!({
                "webCertFile": "/etc/node/fullchain.pem",
                "webKeyFile": "/etc/node/private.key"
            })),
        ),
        (Method::POST, "/panel/api/nodes/add", Some(node_json())),
        (Method::POST, "/panel/api/nodes/update/7", None),
        (Method::POST, "/panel/api/nodes/del/7", None),
        (Method::POST, "/panel/api/nodes/setEnable/7", None),
        (
            Method::POST,
            "/panel/api/nodes/test",
            Some(probe_json("offline")),
        ),
        (
            Method::POST,
            "/panel/api/nodes/certFingerprint",
            Some(json!("sha256-base64-pin")),
        ),
        (
            Method::POST,
            "/panel/api/nodes/inbounds",
            Some(json!([{
                "tag": "inbound-443",
                "remark": "VLESS",
                "protocol": "future-protocol",
                "port": 443
            }])),
        ),
        (
            Method::POST,
            "/panel/api/nodes/probe/7",
            Some(probe_json("online")),
        ),
        (
            Method::POST,
            "/panel/api/nodes/updatePanel",
            Some(json!([
                {"id": 7, "name": "edge-de", "ok": true},
                {"id": 8, "name": "edge-fr", "ok": false, "error": "node is offline"}
            ])),
        ),
        (
            Method::GET,
            "/panel/api/nodes/history/7/netUp/30",
            Some(json!([{"t": 1_700_000_000, "v": 1024.5}])),
        ),
        (
            Method::POST,
            "/panel/api/nodes/mtls/ca",
            Some(json!({"caCert": "-----BEGIN CERTIFICATE-----\npublic-ca\n"})),
        ),
        (Method::POST, "/panel/api/nodes/mtls/trustCA", None),
        (Method::POST, "/panel/api/nodes/mtls/reloadClient", None),
    ];
    for (method, path, object) in routes {
        mount_endpoint(&server, method, path, object).await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let nodes = client.nodes();

    let listed = nodes.list().await.unwrap();
    assert_eq!(listed[0].name, "edge-de");
    assert!(listed[0].has_api_token);
    assert!(listed[0].inbound_tags.is_empty());
    assert_eq!(listed[0].status, NodeStatus::Online);
    assert_eq!(nodes.get(7).await.unwrap().guid, "node-guid");
    assert_eq!(
        nodes.web_certificate_files(7).await.unwrap().web_key_file,
        "/etc/node/private.key"
    );

    let mut request = NodeRequest::new("edge-de", "node.example.com", 2053)
        .with_api_token("write-only-node-token");
    request.remark = "Frankfurt edge".into();
    request.base_path = "/admin/".into();
    request.tls_verify_mode = NodeTlsVerifyMode::Pin;
    request.pinned_cert_sha256 = "sha256-base64-pin".into();
    request.inbound_sync_mode = NodeInboundSyncMode::Selected;
    request.inbound_tags = vec!["inbound-443".into()];
    request.outbound_tag = "direct".into();

    assert_eq!(nodes.create(&request).await.unwrap().id, 7);
    let retained = listed[0].to_request();
    nodes.update(7, &retained).await.unwrap();
    nodes.delete(7).await.unwrap();
    nodes.set_enabled(7, false).await.unwrap();

    let tested = nodes.test_connection(&request).await.unwrap();
    assert_eq!(tested.status, NodeStatus::Offline);
    assert_eq!(tested.error, "connection refused");
    assert_eq!(
        nodes.certificate_fingerprint(&request).await.unwrap(),
        "sha256-base64-pin"
    );
    let remote = nodes.remote_inbounds(&request).await.unwrap();
    assert_eq!(
        remote[0].protocol,
        RemoteInboundProtocol::Other("future-protocol".into())
    );
    assert_eq!(nodes.probe(7).await.unwrap().status, NodeStatus::Online);
    let updates = nodes
        .update_panels(&[7, 8], NodeUpdateChannel::Development)
        .await
        .unwrap();
    assert!(updates[0].ok);
    assert_eq!(updates[1].error, "node is offline");
    let history = nodes
        .history(7, NodeMetric::NetworkUp, HistoryBucket::Minutes30)
        .await
        .unwrap();
    assert!((history[0].value - 1024.5).abs() < f64::EPSILON);
    assert!(nodes.mtls_ca().await.unwrap().ca_cert.contains("public-ca"));
    nodes
        .set_mtls_trust_ca("-----BEGIN CERTIFICATE-----\ntrusted-ca\n")
        .await
        .unwrap();
    nodes.reload_mtls_client().await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let add = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/add")
        .unwrap();
    let body: Value = serde_json::from_slice(&add.body).unwrap();
    assert_eq!(body["apiToken"], "write-only-node-token");
    assert_eq!(body["tlsVerifyMode"], "pin");
    assert_eq!(body["inboundSyncMode"], "selected");
    assert_eq!(body["inboundTags"], json!(["inbound-443"]));
    assert!(body.get("clearApiToken").is_none());

    let update = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/update/7")
        .unwrap();
    let body: Value = serde_json::from_slice(&update.body).unwrap();
    assert!(body.get("apiToken").is_none());
    assert!(body.get("clearApiToken").is_none());

    let panel_update = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/updatePanel")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&panel_update.body).unwrap(),
        json!({"ids": [7, 8], "dev": true})
    );
    let trust = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/mtls/trustCA")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&trust.body).unwrap(),
        json!({"caCert": "-----BEGIN CERTIFICATE-----\ntrusted-ca\n"})
    );
    let probe = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/probe/7")
        .unwrap();
    assert!(probe.body.is_empty());
    let ca = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/nodes/mtls/ca")
        .unwrap();
    assert!(ca.body.is_empty());
}

#[tokio::test]
async fn node_failures_keep_method_path_and_message() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/nodes/add"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "node address is not allowed"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();

    let error = client
        .nodes()
        .create(&NodeRequest::new("private", "127.0.0.1", 2053).with_api_token("token"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::Api { method, url, message }
            if method == Method::POST
                && url.path() == "/panel/api/nodes/add"
                && message == "node address is not allowed"
    ));
}

#[test]
fn node_credentials_are_explicit_mutually_exclusive_and_redacted() {
    let mut request =
        NodeRequest::new("edge", "node.example.com", 2053).with_api_token("private-node-token");
    request.base_path = "/private-panel-path/".into();
    request.outbound_tag = "private-egress-route".into();
    assert_eq!(request.api_token(), Some("private-node-token"));
    assert!(!request.clears_stored_api_token());
    let debug = format!("{request:?}");
    assert!(!debug.contains("private-node-token"));
    assert!(!debug.contains("private-panel-path"));
    assert!(!debug.contains("private-egress-route"));

    let body = serde_json::to_value(&request).unwrap();
    assert_eq!(body["apiToken"], "private-node-token");
    assert!(body.get("clearApiToken").is_none());
    let mut request_fields = body
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    request.clear_stored_api_token();
    let body = serde_json::to_value(&request).unwrap();
    assert!(body.get("apiToken").is_none());
    assert_eq!(body["clearApiToken"], true);
    request_fields.extend(body.as_object().unwrap().keys().cloned());
    let expected_fields = [
        "address",
        "allowPrivateAddress",
        "apiToken",
        "basePath",
        "clearApiToken",
        "enable",
        "id",
        "inboundSyncMode",
        "inboundTags",
        "name",
        "outboundTag",
        "pinnedCertSha256",
        "port",
        "remark",
        "scheme",
        "tlsVerifyMode",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    assert_eq!(request_fields, expected_fields);

    request.retain_stored_api_token();
    let body = serde_json::to_value(&request).unwrap();
    assert!(body.get("apiToken").is_none());
    assert!(body.get("clearApiToken").is_none());

    let future: NodeTlsVerifyMode = serde_json::from_value(json!("future-mode")).unwrap();
    assert_eq!(future, NodeTlsVerifyMode::Other("future-mode".into()));
    assert_eq!(serde_json::to_value(future).unwrap(), "future-mode");
    assert_eq!(NodeScheme::default(), NodeScheme::Https);
}

#[test]
fn node_view_matches_every_v370_source_field_and_never_api_token() {
    let object = serde_json::to_value(NodeView::default())
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = [
        "activeCount",
        "address",
        "allowPrivateAddress",
        "basePath",
        "clientCount",
        "configDirty",
        "configDirtyAt",
        "cpuPct",
        "createdAt",
        "depletedCount",
        "disabledCount",
        "enable",
        "guid",
        "hasApiToken",
        "id",
        "inboundCount",
        "inboundSyncMode",
        "inboundTags",
        "lastError",
        "lastHeartbeat",
        "latencyMs",
        "memPct",
        "name",
        "netDown",
        "netUp",
        "onlineCount",
        "outboundTag",
        "panelVersion",
        "parentGuid",
        "pinnedCertSha256",
        "port",
        "remark",
        "scheme",
        "status",
        "tlsVerifyMode",
        "transitive",
        "updatedAt",
        "uptimeSecs",
        "xrayError",
        "xrayState",
        "xrayVersion",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();

    assert_eq!(object, expected);
    assert!(!object.contains("apiToken"));
    assert_eq!(object.len(), 41);

    let view = NodeView {
        base_path: "/private-panel-path/".into(),
        outbound_tag: "private-egress-route".into(),
        ..NodeView::default()
    };
    let debug = format!("{view:?}");
    assert!(!debug.contains("private-panel-path"));
    assert!(!debug.contains("private-egress-route"));
}
