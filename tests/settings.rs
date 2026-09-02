#![allow(missing_docs)]

use reqwest::Method;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, Error, MoveDirection, OutboundDocuments, OutboundSubscriptionInput, OutboundTestMode,
    PanelSettings, PanelSettingsUpdate, RouteTestRequest, SensitivePayload, UserCredentialsUpdate,
    WarpRegistration, XrayConfig,
};

async fn mount_envelope(server: &MockServer, method: Method, path: &str, object: Option<Value>) {
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
async fn every_v370_settings_and_xray_route_is_wired() {
    let server = MockServer::start().await;
    let subscription = json!({
        "id": 7, "remark": "remote", "url": "https://secret.example/sub?token=abc",
        "enabled": true, "allowPrivate": false, "allowInsecure": false,
        "tagPrefix": "remote-", "updateInterval": 600, "priority": 0,
        "prepend": false, "lastUpdated": 1, "lastError": "",
        "lastFetchedOutbounds": "", "createdAt": 2, "updatedAt": 3,
        "outboundCount": 1
    });
    let outbound_result = json!({
        "tag": "proxy", "success": true, "delay": 25, "mode": "http",
        "httpStatus": 204, "connectMs": 2, "tlsMs": 10, "ttfbMs": 20,
        "endpoints": [], "egress": {"ipv4": "203.0.113.1", "country": "NL"}
    });

    let routes = [
        (
            Method::POST,
            "/panel/api/setting/all",
            Some(json!({"webPort": 2053, "hasApiToken": true})),
        ),
        (
            Method::POST,
            "/panel/api/setting/defaultSettings",
            Some(json!({"pageSize": 50, "subURI": "https://example/sub/"})),
        ),
        (
            Method::POST,
            "/panel/api/setting/factoryDefaults",
            Some(json!({"webPort": "2053"})),
        ),
        (Method::POST, "/panel/api/setting/update", None),
        (Method::POST, "/panel/api/setting/validateRegex", None),
        (Method::POST, "/panel/api/setting/updateUser", None),
        (Method::POST, "/panel/api/setting/restartPanel", None),
        (
            Method::GET,
            "/panel/api/setting/getDefaultJsonConfig",
            Some(json!({"inbounds": []})),
        ),
        (
            Method::GET,
            "/panel/api/setting/apiTokens",
            Some(json!([{
                "id": 1, "name": "automation", "enabled": true, "createdAt": 100,
                "scope": "admin", "expiresAt": 0
            }])),
        ),
        (
            Method::POST,
            "/panel/api/setting/apiTokens/create",
            Some(json!({
                "id": 2, "name": "new", "token": "plaintext-once", "enabled": true,
                "createdAt": 101, "scope": "admin", "expiresAt": 0
            })),
        ),
        (Method::POST, "/panel/api/setting/apiTokens/delete/2", None),
        (
            Method::POST,
            "/panel/api/setting/apiTokens/setEnabled/1",
            None,
        ),
        (Method::POST, "/panel/api/setting/testTgBot", None),
        (
            Method::GET,
            "/panel/api/xray/getDefaultJsonConfig",
            Some(json!({"outbounds": []})),
        ),
        (
            Method::GET,
            "/panel/api/xray/getOutboundsTraffic",
            Some(json!([{"id": 1, "tag": "proxy", "up": 10, "down": 20, "total": 30}])),
        ),
        (
            Method::GET,
            "/panel/api/xray/getXrayResult",
            Some(json!("started")),
        ),
        (
            Method::POST,
            "/panel/api/xray/",
            Some(json!(
                json!({
                    "xraySetting": {"inbounds": [], "outbounds": []},
                    "inboundTags": ["in-1"], "clientReverseTags": [],
                    "outboundTestUrl": "https://example.com/generate_204",
                    "subscriptionOutbounds": [{"tag": "remote-secret"}],
                    "subscriptionOutboundTags": ["remote-secret"]
                })
                .to_string()
            )),
        ),
        (Method::POST, "/panel/api/xray/update", None),
        (
            Method::POST,
            "/panel/api/xray/resetOutboundsTraffic",
            Some(json!("")),
        ),
        (
            Method::POST,
            "/panel/api/xray/testOutbound",
            Some(outbound_result.clone()),
        ),
        (
            Method::POST,
            "/panel/api/xray/testOutbounds",
            Some(json!([outbound_result])),
        ),
        (
            Method::POST,
            "/panel/api/xray/balancerStatus",
            Some(
                json!({"balance": {"tag": "balance", "running": true, "override": "proxy", "selected": ["proxy"]}}),
            ),
        ),
        (
            Method::POST,
            "/panel/api/xray/balancerOverride",
            Some(json!("")),
        ),
        (
            Method::POST,
            "/panel/api/xray/routeTest",
            Some(json!({"matched": true, "outboundTag": "proxy", "groupTags": ["balance"]})),
        ),
        (
            Method::GET,
            "/panel/api/xray/geodata/files",
            Some(json!([{
                "name": "geosite.dat", "kind": "site", "size": 1024,
                "modifiedAt": 1000, "categories": 2
            }])),
        ),
        (
            Method::GET,
            "/panel/api/xray/geodata/categories",
            Some(json!({
                "items": [{"code": "google", "entries": 2, "attributes": ["ads"]}],
                "total": 1
            })),
        ),
        (
            Method::GET,
            "/panel/api/xray/geodata/entries",
            Some(json!({
                "items": [{"kind": "domain", "value": "google.com"}], "total": 1
            })),
        ),
        (
            Method::POST,
            "/panel/api/xray/geodata/validate",
            Some(json!([{
                "token": "geosite:missing", "reason": "categoryMissing",
                "file": "geosite.dat", "code": "missing"
            }])),
        ),
        (
            Method::GET,
            "/panel/api/xray/outbound-subs",
            Some(json!([subscription.clone()])),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs",
            Some(subscription),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs/7/refresh",
            Some(json!([{"tag": "remote-secret"}])),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs/7/move",
            Some(json!("")),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs/7",
            Some(json!("")),
        ),
        (
            Method::DELETE,
            "/panel/api/xray/outbound-subs/7",
            Some(json!("")),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs/8/del",
            Some(json!("")),
        ),
        (
            Method::POST,
            "/panel/api/xray/outbound-subs/parse",
            Some(json!([{"tag": "preview-secret"}])),
        ),
    ];
    for (method, path, object) in routes {
        mount_envelope(&server, method, path, object).await;
    }

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/setting/testSmtp"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"success": false, "stage": "auth", "msg": "bad credentials"}),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let integration_actions = [
        ("warp", "data"),
        ("warp", "config"),
        ("warp", "reg"),
        ("warp", "changeIp"),
        ("warp", "license"),
        ("warp", "del"),
        ("warp", "interval"),
        ("nord", "countries"),
        ("nord", "servers"),
        ("nord", "reg"),
        ("nord", "setKey"),
        ("nord", "data"),
        ("nord", "del"),
    ];
    for (family, action) in integration_actions {
        mount_envelope(
            &server,
            Method::POST,
            &format!("/panel/api/xray/{family}/{action}"),
            Some(json!(if matches!(action, "del" | "interval") {
                ""
            } else {
                "{}"
            })),
        )
        .await;
    }
    for (action, object) in [
        ("countries", json!([{"code": "NL"}])),
        (
            "servers",
            json!({
                "regions": [{"id": "nl_amsterdam", "name": "Netherlands"}],
                "servers": [{
                    "hostname": "nl-amsterdam.example", "ip": "203.0.113.20",
                    "regionId": "nl_amsterdam", "regionName": "Netherlands"
                }]
            }),
        ),
        (
            "reg",
            json!({"username": "pia-user", "accountHint": "pi****er"}),
        ),
        (
            "data",
            json!({"username": "pia-user", "accountHint": "pi****er"}),
        ),
        ("del", Value::Null),
        (
            "addKey",
            json!({
                "tag": "pia-nl-amsterdam", "hostname": "nl-amsterdam.example",
                "secretKey": "private-secret", "address": "10.0.0.2/32",
                "publicKey": "server-public", "endpoint": "203.0.113.20:1337"
            }),
        ),
    ] {
        mount_envelope(
            &server,
            Method::POST,
            &format!("/panel/api/xray/pia/{action}"),
            Some(object),
        )
        .await;
    }

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let settings = client.settings();
    assert_eq!(settings.all().await.unwrap().settings.web.web_port, 2053);
    assert_eq!(settings.defaults().await.unwrap().page_size, 50);
    assert_eq!(
        settings.factory_defaults().await.unwrap()["webPort"],
        "2053"
    );
    settings
        .update(&PanelSettingsUpdate::new(PanelSettings::default()))
        .await
        .unwrap();
    settings.validate_regex(r"^client-[0-9]+$").await.unwrap();
    settings
        .update_user(&UserCredentialsUpdate {
            old_username: "admin".into(),
            old_password: "old-secret".into(),
            new_username: "operator".into(),
            new_password: "new-secret".into(),
            two_factor_code: "123456".into(),
        })
        .await
        .unwrap();
    settings.restart_panel().await.unwrap();
    settings.default_xray_config().await.unwrap();
    assert_eq!(settings.api_tokens().await.unwrap()[0].name, "automation");
    let created = settings
        .create_api_token(&xui_rs::ApiTokenCreateRequest::new("new"))
        .await
        .unwrap();
    assert_eq!(created.token, "plaintext-once");
    settings
        .delete_api_token(2, xui_rs::ApiTokenScope::Admin)
        .await
        .unwrap();
    settings
        .set_api_token_enabled(1, xui_rs::ApiTokenScope::Admin, false)
        .await
        .unwrap();
    let smtp = settings.test_smtp().await.unwrap();
    assert!(!smtp.success);
    assert_eq!(smtp.stage, "auth");
    settings.test_telegram().await.unwrap();

    let xray = client.xray_settings();
    xray.default_config().await.unwrap();
    assert_eq!(xray.outbounds_traffic().await.unwrap()[0].down, 20);
    assert_eq!(xray.xray_result().await.unwrap(), "started");
    let snapshot = xray.settings().await.unwrap();
    assert_eq!(snapshot.inbound_tags, ["in-1"]);
    xray.update(
        &XrayConfig::from(json!({"outbounds": []})),
        "https://example.com/generate_204",
    )
    .await
    .unwrap();
    xray.warp_data().await.unwrap();
    xray.warp_config().await.unwrap();
    xray.register_warp(&WarpRegistration {
        private_key: "private".into(),
        public_key: "public".into(),
    })
    .await
    .unwrap();
    xray.change_warp_ip().await.unwrap();
    xray.set_warp_license("license-secret").await.unwrap();
    xray.delete_warp().await.unwrap();
    xray.set_warp_update_interval(60).await.unwrap();
    xray.nord_countries().await.unwrap();
    xray.nord_servers("NL").await.unwrap();
    xray.register_nord("nord-token").await.unwrap();
    xray.set_nord_key("nord-key").await.unwrap();
    xray.nord_data().await.unwrap();
    xray.delete_nord().await.unwrap();
    xray.pia_countries().await.unwrap();
    xray.pia_servers("NL").await.unwrap();
    xray.register_pia("pia-user", "pia-secret").await.unwrap();
    xray.pia_data().await.unwrap();
    xray.delete_pia().await.unwrap();
    xray.add_pia_key("nl-amsterdam.example").await.unwrap();
    xray.reset_outbound_traffic("proxy").await.unwrap();
    let outbound = json!({"tag": "proxy", "protocol": "freedom"});
    xray.test_outbound(&outbound, None, OutboundTestMode::Http)
        .await
        .unwrap();
    xray.test_outbounds(
        std::slice::from_ref(&outbound),
        Some(std::slice::from_ref(&outbound)),
        OutboundTestMode::Tcp,
    )
    .await
    .unwrap();
    assert!(xray.balancer_status(&["balance"]).await.unwrap()["balance"].running);
    xray.set_balancer_override("balance", "proxy")
        .await
        .unwrap();
    let route = xray
        .test_route(&RouteTestRequest {
            domain: "example.com".into(),
            port: 443,
            network: "tcp".into(),
            ..RouteTestRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(route.outbound_tag, "proxy");
    assert_eq!(xray.geodata_files().await.unwrap()[0].kind, "site");
    assert_eq!(
        xray.geodata_categories("geosite.dat", "goo", 0, 100)
            .await
            .unwrap()
            .items[0]
            .code,
        "google"
    );
    assert_eq!(
        xray.geodata_entries("geosite.dat", "google", "", 0, 100)
            .await
            .unwrap()
            .items[0]
            .value,
        "google.com"
    );
    assert_eq!(
        xray.validate_geodata_tokens(false, &["geosite:missing"])
            .await
            .unwrap()[0]
            .reason,
        "categoryMissing"
    );
    assert_eq!(xray.outbound_subscriptions().await.unwrap()[0].id, 7);
    let input = OutboundSubscriptionInput::new("https://secret.example/sub?token=abc");
    xray.create_outbound_subscription(&input).await.unwrap();
    assert_eq!(
        xray.refresh_outbound_subscription(7)
            .await
            .unwrap()
            .as_slice()
            .len(),
        1
    );
    xray.move_outbound_subscription(7, MoveDirection::Up)
        .await
        .unwrap();
    xray.update_outbound_subscription(7, &input).await.unwrap();
    xray.delete_outbound_subscription(7).await.unwrap();
    xray.delete_outbound_subscription_via_post(8).await.unwrap();
    assert_eq!(
        xray.parse_outbound_subscription("https://secret.example/sub", false, true)
            .await
            .unwrap()
            .as_slice()
            .len(),
        1
    );

    let requests = server.received_requests().await.unwrap();
    let create_token = requests
        .iter()
        .find(|request| request.url.path() == "/panel/api/setting/apiTokens/create")
        .unwrap();
    let create_form = url::form_urlencoded::parse(&create_token.body)
        .into_owned()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(create_form["name"], "new");
    assert_eq!(create_form["scope"], "admin");
    assert_eq!(create_form["expiresAt"], "0");

    for (path, expected) in [
        (
            "/panel/api/setting/apiTokens/delete/2",
            json!({"expectedScope": "admin"}),
        ),
        (
            "/panel/api/setting/apiTokens/setEnabled/1",
            json!({"enabled": false, "expectedScope": "admin"}),
        ),
    ] {
        let request = requests
            .iter()
            .find(|request| request.url.path() == path)
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&request.body).unwrap(),
            expected
        );
    }
}

#[test]
fn settings_debug_output_redacts_every_secret_class() {
    let mut settings = PanelSettings::default();
    settings.web.web_key_file = "private-key-path".into();
    settings.web.web_base_path = "/secret-panel/".into();
    settings.web.panel_outbound = "socks://user:password@proxy".into();
    settings.telegram.tg_bot_token = "telegram-token".into();
    settings.smtp.smtp_password = "smtp-password".into();
    settings.security.two_factor_token = "totp-seed".into();
    settings.ldap.ldap_password = "ldap-password".into();
    settings.subscriptions.sub_key_file = "subscription-private-key".into();
    settings.subscriptions.sub_uri = "https://example/sub/secret-id".into();
    settings.subscriptions.sub_routing_rules = "secret-routing-rule".into();
    let debug = format!("{settings:?}");
    for secret in [
        "private-key-path",
        "/secret-panel/",
        "socks://user:password@proxy",
        "telegram-token",
        "smtp-password",
        "totp-seed",
        "ldap-password",
        "subscription-private-key",
        "https://example/sub/secret-id",
        "secret-routing-rule",
    ] {
        assert!(!debug.contains(secret));
    }
}

#[test]
fn exact_acronym_wire_names_and_sensitive_wrappers_are_safe() {
    let mut settings = PanelSettings::default();
    settings.web.trusted_proxy_cidrs = "127.0.0.1/32".into();
    settings.telegram.tg_bot_api_server = "https://api.example".into();
    settings.subscriptions.sub_uri = "https://sub.example/base".into();
    settings.subscriptions.external_traffic_inform_uri = "https://traffic.example".into();
    settings.ldap.ldap_use_tls = true;
    settings.ldap.ldap_bind_dn = "cn=admin,dc=example".into();
    settings.ldap.ldap_default_total_gb = 25;

    let value = serde_json::to_value(settings).unwrap();
    assert_eq!(value["trustedProxyCIDRs"], "127.0.0.1/32");
    assert_eq!(value["tgBotAPIServer"], "https://api.example");
    assert_eq!(value["subURI"], "https://sub.example/base");
    assert_eq!(value["externalTrafficInformURI"], "https://traffic.example");
    assert_eq!(value["ldapUseTLS"], true);
    assert_eq!(value["ldapBindDN"], "cn=admin,dc=example");
    assert_eq!(value["ldapDefaultTotalGB"], 25);

    let payload: SensitivePayload = serde_json::from_value(json!("integration-secret")).unwrap();
    assert!(!format!("{payload:?}").contains("integration-secret"));
    let documents: OutboundDocuments = serde_json::from_value(json!([{
        "password": "outbound-secret"
    }]))
    .unwrap();
    assert!(!format!("{documents:?}").contains("outbound-secret"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn panel_settings_cover_every_v370_all_setting_field() {
    let actual = serde_json::to_value(PanelSettings::default()).unwrap();
    let actual = actual
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = [
        "datepicker",
        "expireDiff",
        "externalTrafficInformEnable",
        "externalTrafficInformURI",
        "ipLimitAllowlist",
        "ldapAutoCreate",
        "ldapAutoDelete",
        "ldapBaseDN",
        "ldapBindDN",
        "ldapDefaultExpiryDays",
        "ldapDefaultLimitIP",
        "ldapDefaultTotalGB",
        "ldapEnable",
        "ldapFlagField",
        "ldapHost",
        "ldapInboundTags",
        "ldapInsecureSkipVerify",
        "ldapInvertFlag",
        "ldapPassword",
        "ldapPort",
        "ldapSyncCron",
        "ldapTruthyValues",
        "ldapUseTLS",
        "ldapUserAttr",
        "ldapUserFilter",
        "ldapVlessField",
        "outboundDownThreshold",
        "pageSize",
        "panelOutbound",
        "remarkTemplate",
        "restartXrayOnClientDisable",
        "sessionMaxAge",
        "smtpCpu",
        "smtpEnable",
        "smtpEnabledEvents",
        "smtpEncryptionType",
        "smtpFrom",
        "smtpFromName",
        "smtpHost",
        "smtpMemory",
        "smtpPassword",
        "smtpPort",
        "smtpTo",
        "smtpUsername",
        "subAnnounce",
        "subCertFile",
        "subClashAutoDetect",
        "subClashEnable",
        "subClashEnableRouting",
        "subClashPath",
        "subClashRules",
        "subClashURI",
        "subClashUserAgentRegex",
        "subDomain",
        "subEnable",
        "subEnableRouting",
        "subEncrypt",
        "subHideSettings",
        "subIncyEnableRouting",
        "subIncyRoutingRules",
        "subJsonAlwaysArray",
        "subJsonAutoDetect",
        "subJsonEnable",
        "subJsonFinalMask",
        "subJsonMux",
        "subJsonObservatory",
        "subJsonPath",
        "subJsonRules",
        "subJsonURI",
        "subJsonUserAgentRegex",
        "subKeyFile",
        "subListen",
        "subPath",
        "subPort",
        "subProfileUrl",
        "subRoutingRules",
        "subShowIdentityOnAllLinks",
        "subSupportUrl",
        "subThemeDir",
        "subTitle",
        "subURI",
        "subUpdates",
        "tgBotAPIServer",
        "tgBotBackup",
        "tgBotChatId",
        "tgBotEnable",
        "tgBotProxy",
        "tgBotToken",
        "tgCpu",
        "tgEnabledEvents",
        "tgLang",
        "tgMemory",
        "tgRunTime",
        "timeLocation",
        "trafficDiff",
        "trustedProxyCIDRs",
        "twoFactorEnable",
        "twoFactorToken",
        "warpUpdateInterval",
        "webBasePath",
        "webCertFile",
        "webDomain",
        "webKeyFile",
        "webListen",
        "webPort",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(actual.len(), 105);
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn unavailable_smtp_service_remains_a_typed_panel_error() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/setting/testSmtp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "email service not available",
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

    let error = client.settings().test_smtp().await.unwrap_err();
    assert!(
        matches!(error, Error::Api { message, .. } if message == "email service not available")
    );
}
