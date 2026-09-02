#![allow(missing_docs)]

use std::collections::BTreeSet;

use reqwest::Method;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, Error, HostGroup, HostJsonOverride, HostSecurity, MihomoIpVersion, SubscriptionFormat,
    VlessRoute,
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

fn group_json() -> Value {
    json!({
        "groupId": "cdn/prod",
        "inboundIds": [7, 8],
        "hosts": ["cdn.example.com:8443", "2001:db8::1"],
        "sortOrder": 2,
        "remark": "production CDN",
        "serverDescription": "edge",
        "isDisabled": false,
        "isHidden": false,
        "tags": null,
        "port": 8443,
        "security": "future-security",
        "sni": "origin.example.com",
        "hostHeader": "origin.example.com",
        "path": "/edge",
        "alpn": null,
        "fingerprint": "chrome",
        "overrideSniFromAddress": false,
        "keepSniBlank": false,
        "pinnedPeerCertSha256": null,
        "verifyPeerCertByName": "origin.example.com",
        "allowInsecure": false,
        "echConfigList": "public-ech-config",
        "muxParams": null,
        "sockoptParams": "",
        "finalMask": "",
        "vlessRoute": "443",
        "excludeFromSubTypes": null,
        "nodeGuids": null,
        "mihomoIpVersion": "ipv6-prefer",
        "mihomoX25519": true,
        "shuffleHost": true
    })
}

fn row_json(id: i64, inbound_id: i64, address: &str) -> Value {
    json!({
        "id": id,
        "groupId": "cdn/prod",
        "inboundId": inbound_id,
        "address": address,
        "sortOrder": 2,
        "remark": "production CDN",
        "serverDescription": "edge",
        "isDisabled": false,
        "isHidden": false,
        "tags": ["CDN", "PROD"],
        "port": 8443,
        "security": "tls",
        "sni": "origin.example.com",
        "hostHeader": "origin.example.com",
        "path": "/edge",
        "alpn": ["h2", "http/1.1"],
        "fingerprint": "chrome",
        "overrideSniFromAddress": true,
        "keepSniBlank": false,
        "pinnedPeerCertSha256": ["sha256-pin"],
        "verifyPeerCertByName": "origin.example.com",
        "allowInsecure": false,
        "echConfigList": "public-ech-config",
        "muxParams": "{\"enabled\":true}",
        "sockoptParams": "{\"tcpKeepAliveIdle\":60}",
        "finalMask": "{\"tcp\":[]}",
        "vlessRoute": "443",
        "excludeFromSubTypes": ["clash"],
        "nodeGuids": ["node-a"],
        "mihomoIpVersion": "dual",
        "mihomoX25519": true,
        "shuffleHost": true,
        "createdAt": 100,
        "updatedAt": 200
    })
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn every_v370_host_route_is_wired_with_exact_payloads() {
    let server = MockServer::start().await;
    let created = json!([
        row_json(1, 7, "cdn.example.com"),
        row_json(2, 8, "cdn.example.com")
    ]);
    let routes = [
        (
            Method::GET,
            "/panel/api/hosts/list",
            Some(json!([group_json()])),
        ),
        (
            Method::GET,
            "/panel/api/hosts/get/cdn%2Fprod",
            Some(group_json()),
        ),
        (
            Method::GET,
            "/panel/api/hosts/byInbound/999",
            Some(Value::Null),
        ),
        (
            Method::GET,
            "/panel/api/hosts/tags",
            Some(json!(["CDN", "PROD"])),
        ),
        (Method::POST, "/panel/api/hosts/add", Some(created.clone())),
        (
            Method::POST,
            "/panel/api/hosts/bulk/add",
            Some(created.clone()),
        ),
        (
            Method::POST,
            "/panel/api/hosts/update/cdn%2Fprod",
            Some(created),
        ),
        (Method::POST, "/panel/api/hosts/del/cdn%2Fprod", None),
        (Method::POST, "/panel/api/hosts/setEnable/cdn%2Fprod", None),
        (Method::POST, "/panel/api/hosts/reorder", None),
        (Method::POST, "/panel/api/hosts/bulk/setEnable", None),
        (Method::POST, "/panel/api/hosts/bulk/del", None),
    ];
    for (method, path, object) in routes {
        mount_endpoint(&server, method, path, object).await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let hosts = client.hosts();

    let listed = hosts.list().await.unwrap();
    assert_eq!(listed[0].group_id, "cdn/prod");
    assert!(listed[0].options.tags.is_empty());
    assert_eq!(
        listed[0].options.security,
        HostSecurity::Other("future-security".into())
    );
    assert_eq!(listed[0].options.vless_route.port(), Some(443));
    assert_eq!(
        listed[0].options.mihomo_ip_version,
        MihomoIpVersion::Ipv6Prefer
    );
    assert!(listed[0].options.mux_params.is_empty());

    assert_eq!(
        hosts.get("cdn/prod").await.unwrap().options.remark,
        "production CDN"
    );
    assert!(hosts.list_by_inbound(999).await.unwrap().is_empty());
    assert_eq!(hosts.tags().await.unwrap(), ["CDN", "PROD"]);

    let mut group = HostGroup::new(vec![7, 8], "production CDN");
    group.group_id = "cdn/prod".into();
    group.hosts = vec!["cdn.example.com".into()];
    group.options.port = 8443;
    group.options.security = HostSecurity::Tls;
    group.options.tags = vec!["CDN".into(), "PROD".into()];
    group.options.vless_route = VlessRoute::new(443);
    group.options.exclude_from_sub_types = vec![SubscriptionFormat::Clash];
    group.options.mihomo_ip_version = MihomoIpVersion::Dual;
    group.options.mux_params = HostJsonOverride::from_value(&json!({"enabled": true})).unwrap();
    group.options.sockopt_params =
        HostJsonOverride::from_value(&json!({"tcpKeepAliveIdle": 60})).unwrap();
    group.options.final_mask = HostJsonOverride::from_value(&json!({"tcp": []})).unwrap();

    assert_eq!(hosts.create(&group).await.unwrap().len(), 2);
    assert_eq!(
        hosts.bulk_create(&group).await.unwrap()[0].options.security,
        HostSecurity::Tls
    );
    assert_eq!(
        hosts.update("cdn/prod", &group).await.unwrap()[0].group_id,
        "cdn/prod"
    );
    hosts.delete("cdn/prod").await.unwrap();
    hosts.set_enabled("cdn/prod", true).await.unwrap();
    hosts.reorder(&["cdn/prod", "backup"]).await.unwrap();
    hosts
        .bulk_set_enabled(&["cdn/prod", "backup"], false)
        .await
        .unwrap();
    hosts.bulk_delete(&["cdn/prod", "backup"]).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let add = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/hosts/add")
        .unwrap();
    let body: Value = serde_json::from_slice(&add.body).unwrap();
    assert_eq!(body["inboundIds"], json!([7, 8]));
    assert_eq!(body["security"], "tls");
    assert_eq!(body["vlessRoute"], "443");
    assert_eq!(body["muxParams"], "{\"enabled\":true}");
    assert_eq!(body["sockoptParams"], "{\"tcpKeepAliveIdle\":60}");
    assert_eq!(body["finalMask"], "{\"tcp\":[]}");
    assert_eq!(body["excludeFromSubTypes"], json!(["clash"]));

    let reordered = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/hosts/reorder")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&reordered.body).unwrap(),
        json!({"ids": ["cdn/prod", "backup"]})
    );
    let enabled = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/hosts/setEnable/cdn%2Fprod")
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&enabled.body).unwrap(),
        json!({"enable": true})
    );
}

#[tokio::test]
async fn host_failures_keep_method_path_and_message() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/hosts/add"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "inbound not found"
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
        .hosts()
        .create(&HostGroup::new(vec![999], "missing"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::Api { method, url, message }
            if method == Method::POST
                && url.path() == "/panel/api/hosts/add"
                && message == "inbound not found"
    ));
}

#[test]
fn typed_host_values_round_trip_and_json_debug_is_redacted() {
    let future: HostSecurity = serde_json::from_value(json!("next-security")).unwrap();
    assert_eq!(future, HostSecurity::Other("next-security".into()));
    assert_eq!(serde_json::to_value(&future).unwrap(), "next-security");

    let route: VlessRoute = serde_json::from_value(json!(65535)).unwrap();
    assert_eq!(route.port(), Some(65535));
    assert_eq!(serde_json::to_value(route).unwrap(), "65535");
    assert!(serde_json::from_value::<VlessRoute>(json!(65536)).is_err());

    let override_value = HostJsonOverride::from_value(&json!({
        "dialerProxy": "private-routing-tag"
    }))
    .unwrap();
    assert_eq!(
        override_value.value().unwrap().unwrap()["dialerProxy"],
        "private-routing-tag"
    );
    assert!(!format!("{override_value:?}").contains("private-routing-tag"));
    let null_override: HostJsonOverride = serde_json::from_value(Value::Null).unwrap();
    assert!(null_override.is_empty());
}

#[test]
fn host_group_matches_every_v370_source_field() {
    let object = serde_json::to_value(HostGroup::default())
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = [
        "allowInsecure",
        "alpn",
        "echConfigList",
        "excludeFromSubTypes",
        "finalMask",
        "fingerprint",
        "groupId",
        "hostHeader",
        "hosts",
        "inboundIds",
        "isDisabled",
        "isHidden",
        "keepSniBlank",
        "mihomoIpVersion",
        "mihomoX25519",
        "muxParams",
        "nodeGuids",
        "overrideSniFromAddress",
        "path",
        "pinnedPeerCertSha256",
        "port",
        "remark",
        "security",
        "serverDescription",
        "shuffleHost",
        "sni",
        "sockoptParams",
        "sortOrder",
        "tags",
        "verifyPeerCertByName",
        "vlessRoute",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();

    assert_eq!(object, expected);
}
