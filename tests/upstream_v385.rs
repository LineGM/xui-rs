#![allow(missing_docs)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{self, method, path},
};
use xui_rs::{
    Client, ClientConfig, InboundProtocol, PanelSettings, PanelSettingsUpdate,
    SubscriptionBalancerInput, SubscriptionBalancerStrategy, SubscriptionClient,
    SubscriptionSettings,
};

fn client(server: &MockServer) -> Client {
    Client::builder(format!("{}/secret/", server.uri()))
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap()
}

#[tokio::test]
async fn new_panel_actions_use_base_path_auth_and_propagate_errors() {
    for success in [true, false] {
        let server = MockServer::start().await;
        for (route, obj) in [
            ("setting/testDiscord", Value::Null),
            (
                "clients/happLink/7",
                json!({"encryptedLink":"happ://crypt5/private-link"}),
            ),
        ] {
            Mock::given(method("POST"))
                .and(path(format!("/secret/panel/api/{route}")))
                .and(matchers::header("authorization", "Bearer api-secret"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json!({"success":success,"msg":"result","obj":obj})),
                )
                .expect(1)
                .mount(&server)
                .await;
        }
        let client = client(&server);
        assert_eq!(client.settings().test_discord().await.is_ok(), success);
        let link = client.clients().happ_link(7).await;
        if success {
            let link = link.unwrap();
            assert_eq!(link.encrypted_link, "happ://crypt5/private-link");
            assert!(!format!("{link:?}").contains("private-link"));
        } else {
            assert!(link.is_err());
        }
    }
}

#[tokio::test]
async fn happ_link_requires_an_object_and_preserves_http_auth_failures() {
    for (status, body, kind) in [
        (
            200,
            json!({"success":true,"obj":null}),
            xui_rs::ErrorKind::MissingObject,
        ),
        (
            200,
            json!({"success":true,"obj":{}}),
            xui_rs::ErrorKind::Decode,
        ),
        (
            401,
            json!({"success":false}),
            xui_rs::ErrorKind::Unauthorized,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/secret/panel/api/clients/happLink/7"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            client(&server)
                .clients()
                .happ_link(7)
                .await
                .unwrap_err()
                .kind(),
            kind
        );
    }
}

#[tokio::test]
async fn settings_round_trip_preserves_every_tagged_field_and_redacts_new_secrets() {
    let spec: Value =
        serde_json::from_str(include_str!("../spec/3x-ui-v3.8.5.openapi.json")).unwrap();
    let props = spec["components"]["schemas"]["AllSetting"]["properties"]
        .as_object()
        .unwrap();
    let wire: serde_json::Map<String, Value> = props
        .iter()
        .map(|(name, prop)| {
            let value = match prop["type"].as_str().unwrap() {
                "boolean" => json!(true),
                "integer" => json!(7),
                "string" => json!(format!("private-{name}")),
                other => panic!("unexpected setting type: {other}"),
            };
            (name.clone(), value)
        })
        .collect();
    let settings: PanelSettings = serde_json::from_value(json!(wire)).unwrap();
    assert_eq!(serde_json::to_value(&settings).unwrap(), json!(wire));
    let debug = format!("{settings:?}");
    for name in [
        "discordBotToken",
        "subHappNewUrl",
        "subHappFallbackUrl",
        "subExpiredTemplate",
    ] {
        assert!(!debug.contains(&format!("private-{name}")));
    }
    let server = MockServer::start().await;
    let mut update = PanelSettingsUpdate::new(settings);
    update.clear_discord_bot_token = true;
    Mock::given(method("POST")).and(path("/secret/panel/api/setting/update"))
        .and(matchers::body_partial_json(json!({"discordBotToken":"private-discordBotToken","clearDiscordBotToken":true,"subProfileMode":"private-subProfileMode","subHappAlwaysHwid":true})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true})))
        .expect(1).mount(&server).await;
    client(&server).settings().update(&update).await.unwrap();
}

#[tokio::test]
async fn bulk_adjust_and_keepalive_distinguish_omitted_from_explicit_zero() {
    let config = ClientConfig::new("alice");
    assert!(
        serde_json::to_value(&config)
            .unwrap()
            .get("keepAlive")
            .is_none()
    );
    let mut config = config;
    config.keep_alive = Some(0);
    assert_eq!(serde_json::to_value(&config).unwrap()["keepAlive"], 0);
    for success in [true, false] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/secret/panel/api/clients/bulkAdjust"))
            .and(matchers::body_partial_json(
                json!({"limitHwid":0,"adTag":"none"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"success":success,"msg":"result","obj":{"adjusted":1,"skipped":[]}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let request = xui_rs::BulkAdjustRequest {
            emails: vec!["alice".into()],
            limit_hwid: Some(0),
            ad_tag: "none".into(),
            ..Default::default()
        };
        assert_eq!(
            client(&server)
                .clients()
                .bulk_adjust(&request)
                .await
                .is_ok(),
            success
        );
    }
}

#[tokio::test]
async fn client_updates_send_omission_zero_and_stored_keepalive_distinctly() {
    let server = MockServer::start().await;
    for keep_alive in [None, Some(0), Some(25)] {
        let mut config = ClientConfig::new("alice");
        config.keep_alive = keep_alive;
        Mock::given(method("POST"))
            .and(path("/secret/panel/api/clients/update/alice"))
            .and(move |request: &wiremock::Request| {
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                match keep_alive {
                    None => body.get("keepAlive").is_none(),
                    Some(value) => body["keepAlive"] == value,
                }
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .clients()
            .update("alice", &config)
            .await
            .unwrap();
    }
    for seconds in [0, 25] {
        let record: xui_rs::ClientRecord = serde_json::from_value(json!({
            "email":"alice", "keepAlive":seconds
        }))
        .unwrap();
        assert_eq!(record.to_config().keep_alive, Some(seconds));
    }
}

#[tokio::test]
async fn default_bulk_adjust_does_not_reset_hwid_or_advertisement_settings() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/secret/panel/api/clients/bulkAdjust"))
        .and(|request: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            body.get("limitHwid").is_none()
                && body.get("adTag").is_none()
                && body["emails"] == json!(["alice"])
                && body["addDays"] == 7
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success":true,"obj":{"adjusted":1,"skipped":[]}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut request = xui_rs::BulkAdjustRequest {
        emails: vec!["alice".into()],
        add_days: 7,
        ..Default::default()
    };
    assert_eq!(
        client(&server)
            .clients()
            .bulk_adjust(&request)
            .await
            .unwrap()
            .adjusted,
        1
    );
    request.ad_tag = "private-advertisement-tag".into();
    assert!(!format!("{request:?}").contains("private-advertisement-tag"));
}

#[tokio::test]
async fn balancer_weights_are_a_json_form_field_and_survive_reads() {
    for success in [true, false] {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/secret/panel/api/sub-balancers"))
            .and(|request: &wiremock::Request| {
                let pairs = url::form_urlencoded::parse(&request.body).into_owned().collect::<Vec<_>>();
                pairs.contains(&("memberWeights".into(), "{\"7\":0.5,\"9\":2.0}".into())) && pairs.iter().filter(|(key,_)| key=="inboundIds").count()==2
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":success,"msg":"result","obj":{"id":1,"memberWeights":{"7":0.5,"9":2.0}}})))
            .expect(1).mount(&server).await;
        let mut input = SubscriptionBalancerInput::new("weighted", vec![7, 9]);
        input.strategy = SubscriptionBalancerStrategy::LeastLoad;
        input.member_weights = BTreeMap::from([(7, 0.5), (9, 2.0)]);
        let result = client(&server)
            .subscription_balancers()
            .create(&input)
            .await;
        if success {
            assert_eq!(result.unwrap().member_weights, input.member_weights);
        } else {
            assert!(result.is_err());
        }
        input.member_weights.insert(7, f64::NAN);
        assert!(
            client(&server)
                .subscription_balancers()
                .create(&input)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn balancer_weight_updates_propagate_errors_and_null_weights_decode() {
    for success in [true, false] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/secret/panel/api/sub-balancers/5"))
            .and(|request: &wiremock::Request| {
                url::form_urlencoded::parse(&request.body)
                    .any(|(k, v)| k == "memberWeights" && v == "{\"7\":1.25}")
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success":success,"msg":"result","obj":{"id":5,"memberWeights":{"7":1.25}}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let mut input = SubscriptionBalancerInput::new("weighted", vec![7]);
        input.strategy = SubscriptionBalancerStrategy::LeastLoad;
        input.member_weights.insert(7, 1.25);
        let result = client(&server)
            .subscription_balancers()
            .update(5, &input)
            .await;
        if success {
            assert_eq!(result.unwrap().member_weights, input.member_weights);
        } else {
            assert_eq!(result.unwrap_err().kind(), xui_rs::ErrorKind::Api);
        }
    }
    for wire in [
        json!({}),
        json!({"memberWeights":null}),
        json!({"memberWeights":{}}),
    ] {
        let balancer: xui_rs::SubscriptionBalancer = serde_json::from_value(wire).unwrap();
        assert!(balancer.member_weights.is_empty());
    }
}

#[tokio::test]
async fn invalid_balancer_weights_fail_before_any_http_mutation() {
    let server = MockServer::start().await;
    let client = client(&server);
    for weight in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(f64::from(f32::from_bits(1)).to_bits() - 1),
        f64::from_bits(f64::from(f32::MAX).to_bits() + 1),
    ] {
        let mut input = SubscriptionBalancerInput::new("weighted", vec![7]);
        input.strategy = SubscriptionBalancerStrategy::LeastLoad;
        input.member_weights.insert(7, weight);
        for result in [
            client.subscription_balancers().create(&input).await,
            client.subscription_balancers().update(5, &input).await,
        ] {
            assert_eq!(result.unwrap_err().kind(), xui_rs::ErrorKind::Configuration);
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn balancer_weight_float32_boundaries_are_accepted_on_create_and_update() {
    for weight in [f64::from(f32::from_bits(1)), f64::from(f32::MAX)] {
        let server = MockServer::start().await;
        let weights = BTreeMap::from([(7, weight)]);
        let encoded = serde_json::to_string(&weights).unwrap();
        for route in ["sub-balancers", "sub-balancers/5"] {
            let encoded = encoded.clone();
            Mock::given(method("POST"))
                .and(path(format!("/secret/panel/api/{route}")))
                .and(move |request: &wiremock::Request| {
                    url::form_urlencoded::parse(&request.body)
                        .any(|(key, value)| key == "memberWeights" && value == encoded)
                })
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "success":true,"obj":{"id":5,"memberWeights":weights}
                })))
                .expect(1)
                .mount(&server)
                .await;
        }
        let client = client(&server);
        let mut input = SubscriptionBalancerInput::new("weighted", vec![7]);
        input.strategy = SubscriptionBalancerStrategy::LeastLoad;
        input.member_weights = weights;
        client
            .subscription_balancers()
            .create(&input)
            .await
            .unwrap();
        client
            .subscription_balancers()
            .update(5, &input)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn public_aliases_preserve_the_subscription_server_base_path() {
    let server = MockServer::start().await;
    for alias in ["mihomo", "clash-legacy"] {
        for verb in ["GET", "HEAD"] {
            Mock::given(method(verb))
                .and(path(format!("/tenant/subscriptions/{alias}/private%2Fid")))
                .respond_with(ResponseTemplate::new(200).set_body_string("proxies: []"))
                .expect(1)
                .mount(&server)
                .await;
        }
    }
    let subs = SubscriptionClient::builder(format!("{}/tenant/subscriptions", server.uri()))
        .unwrap()
        .clash_path("custom/clash/")
        .build()
        .unwrap();
    subs.mihomo("private/id").await.unwrap();
    subs.mihomo_metadata("private/id").await.unwrap();
    subs.clash_legacy("private/id").await.unwrap();
    subs.clash_legacy_metadata("private/id").await.unwrap();
}

#[tokio::test]
async fn shadowed_aliases_fail_before_http_while_configured_routes_remain_usable() {
    for base_path in ["/", "/tenant/"] {
        for alias in ["mihomo", "clash-legacy"] {
            for owner in ["raw", "json", "clash"] {
                let server = MockServer::start().await;
                let builder =
                    SubscriptionClient::builder(format!("{}{base_path}", server.uri())).unwrap();
                let configured_path = format!("/{alias}/");
                let subs = match owner {
                    "raw" => builder.raw_path(configured_path),
                    "json" => builder.json_path(configured_path),
                    _ => builder.clash_path(configured_path),
                }
                .build()
                .unwrap();
                let body = if owner == "json" {
                    "[]"
                } else {
                    "configured content"
                };
                Mock::given(path(format!("{base_path}{alias}/private-id")))
                    .respond_with(ResponseTemplate::new(200).set_body_string(body))
                    .mount(&server)
                    .await;
                if alias == "mihomo" && owner == "clash" {
                    // Upstream deliberately reuses the configured Clash handler.
                    assert_eq!(
                        subs.mihomo("private-id").await.unwrap().content.as_str(),
                        body
                    );
                    subs.mihomo_metadata("private-id").await.unwrap();
                } else {
                    let errors = if alias == "mihomo" {
                        [
                            subs.mihomo("private-id").await.unwrap_err(),
                            subs.mihomo_metadata("private-id").await.unwrap_err(),
                        ]
                    } else {
                        [
                            subs.clash_legacy("private-id").await.unwrap_err(),
                            subs.clash_legacy_metadata("private-id").await.unwrap_err(),
                        ]
                    };
                    for error in errors {
                        assert_eq!(error.kind(), xui_rs::ErrorKind::Configuration);
                        assert!(!error.to_string().contains("private-id"));
                    }
                    assert!(server.received_requests().await.unwrap().is_empty());
                }
                match owner {
                    "raw" => {
                        subs.raw("private-id").await.unwrap();
                    }
                    "json" => {
                        subs.json("private-id").await.unwrap();
                    }
                    _ => {
                        subs.clash("private-id").await.unwrap();
                    }
                }
                let other_alias = if alias == "mihomo" {
                    "clash-legacy"
                } else {
                    "mihomo"
                };
                Mock::given(path(format!("{base_path}{other_alias}/private-id")))
                    .respond_with(ResponseTemplate::new(200).set_body_string("proxies: []"))
                    .expect(1)
                    .mount(&server)
                    .await;
                if alias == "mihomo" {
                    subs.clash_legacy("private-id").await.unwrap();
                } else {
                    subs.mihomo("private-id").await.unwrap();
                }
            }
        }
    }
}

#[tokio::test]
async fn settings_aliases_check_public_uris_and_configured_router_paths() {
    let server = MockServer::start().await;
    for public_uri_collision in [false, true] {
        let settings = SubscriptionSettings {
            sub_uri: format!(
                "{}/{}",
                server.uri(),
                if public_uri_collision {
                    "mihomo"
                } else {
                    "custom/raw"
                }
            ),
            sub_path: if public_uri_collision {
                "/configured-raw/"
            } else {
                "/mihomo/"
            }
            .into(),
            sub_json_uri: format!("{}/custom/json/", server.uri()),
            sub_json_path: "/json/".into(),
            sub_clash_uri: format!("{}/clash-legacy/", server.uri()),
            sub_clash_path: if public_uri_collision {
                "/configured-clash/"
            } else {
                "/clash-legacy/"
            }
            .into(),
            ..SubscriptionSettings::default()
        };
        let subs = SubscriptionClient::from_settings(&settings).unwrap();
        for error in [
            subs.mihomo("private-id").await.unwrap_err(),
            subs.mihomo_metadata("private-id").await.unwrap_err(),
            subs.clash_legacy("private-id").await.unwrap_err(),
            subs.clash_legacy_metadata("private-id").await.unwrap_err(),
        ] {
            assert_eq!(error.kind(), xui_rs::ErrorKind::Configuration);
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn public_aliases_and_hwid_status_are_encoded_bounded_and_unauthenticated() {
    let server = MockServer::start().await;
    for prefix in ["mihomo", "clash-legacy", "custom/raw"] {
        let suffix = if prefix == "custom/raw" {
            "/hwid-status"
        } else {
            ""
        };
        for verb in ["GET", "HEAD"] {
            Mock::given(method(verb))
                .and(path(format!("/{prefix}/private%2Fid{suffix}")))
                .and(|r: &wiremock::Request| {
                    !r.headers.contains_key("authorization") && !r.headers.contains_key("cookie")
                })
                .respond_with(
                    ResponseTemplate::new(200).set_body_string(if suffix.is_empty() {
                        "proxies: []"
                    } else {
                        r#"{"active":true,"limit":2,"registered":1,"remaining":1,"full":false}"#
                    }),
                )
                .expect(1)
                .mount(&server)
                .await;
        }
    }
    let subs = SubscriptionClient::builder(server.uri())
        .unwrap()
        .raw_path("custom/raw/")
        .clash_path("custom/clash/")
        .build()
        .unwrap();
    assert_eq!(
        subs.mihomo("private/id").await.unwrap().content.as_str(),
        "proxies: []"
    );
    subs.mihomo_metadata("private/id").await.unwrap();
    subs.clash_legacy("private/id").await.unwrap();
    subs.clash_legacy_metadata("private/id").await.unwrap();
    assert_eq!(subs.hwid_status("private/id").await.unwrap().remaining, 1);
    subs.hwid_status_metadata("private/id").await.unwrap();
    let error = subs.hwid_status("missing-private-id").await.unwrap_err();
    assert!(!format!("{error:?}").contains("missing-private-id"));
    let limited = SubscriptionClient::builder(server.uri())
        .unwrap()
        .response_body_limit(2)
        .build()
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/sub/oversize/hwid-status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1234"))
        .mount(&server)
        .await;
    assert_eq!(
        limited.hwid_status("oversize").await.unwrap_err().kind(),
        xui_rs::ErrorKind::ResponseTooLarge
    );
}

#[tokio::test]
async fn hwid_status_rejects_malformed_or_incomplete_json_with_a_redacted_url() {
    for body in ["not json", "null", "{}"] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sub/private-subscription/hwid-status"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = SubscriptionClient::new(server.uri())
            .unwrap()
            .hwid_status("private-subscription")
            .await
            .unwrap_err();
        assert_eq!(error.kind(), xui_rs::ErrorKind::Decode);
        assert!(!format!("{error:?} {error}").contains("private-subscription"));
    }
}

#[tokio::test]
async fn nullable_link_and_log_collections_are_empty() {
    let server = MockServer::start().await;
    for (verb, route) in [
        ("GET", "inbounds/allLinks"),
        ("POST", "server/logs/0"),
        ("POST", "server/xraylogs/0"),
    ] {
        Mock::given(method(verb))
            .and(path(format!("/secret/panel/api/{route}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"success":true,"obj":null})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    assert!(client.inbounds().all_links().await.unwrap().is_empty());
    assert!(
        client
            .server()
            .panel_logs(&xui_rs::PanelLogRequest::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .server()
            .xray_logs(&xui_rs::XrayLogRequest::default())
            .await
            .unwrap()
            .is_empty()
    );
}

#[test]
fn tuic_and_reality_models_decode_actual_upstream_wire_names() {
    for (wire, expected) in [
        ("tuic", xui_rs::RemoteInboundProtocol::Tuic),
        ("amneziawg", xui_rs::RemoteInboundProtocol::Amneziawg),
    ] {
        let protocol: xui_rs::RemoteInboundProtocol = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(protocol, expected);
        assert_eq!(serde_json::to_value(protocol).unwrap(), wire);
    }
    let snapshot: xui_rs::XraySettingsSnapshot = serde_json::from_value(json!({
        "geodataSources": [{"url":"https://example.com/geoip.dat","file":"geoip.dat"}]
    }))
    .unwrap();
    assert_eq!(snapshot.geodata_sources[0].file, "geoip.dat");
    assert_eq!(
        serde_json::to_value(&snapshot).unwrap()["geodataSources"][0]["url"],
        "https://example.com/geoip.dat"
    );
    assert_eq!(
        serde_json::from_value::<InboundProtocol>(json!("tuic")).unwrap(),
        InboundProtocol::Tuic
    );
    let tuic: xui_rs::TuicServerSettings = serde_json::from_value(
        json!({"private_key":"private-key","zero_rtt_handshake":true,"max_idle_time":15}),
    )
    .unwrap();
    assert!(tuic.zero_rtt_handshake);
    assert_eq!(tuic.max_idle_time, 15);
    assert!(!format!("{tuic:?}").contains("private-key"));
    let scan: xui_rs::RealityScanResult = serde_json::from_value(json!({"tls13":true,"tlsVersion":"1.3","h2":true,"alpn":"h2","x25519":true,"curveID":"X25519","privateTarget":true,"certChainValid":true,"certChainBytes":4000})).unwrap();
    assert!(scan.tls13 && scan.h2 && scan.x25519 && scan.private_target && scan.cert_chain_valid);
    assert_eq!(scan.cert_chain_bytes, 4000);
    assert_eq!(scan.curve_id, "X25519");
}

#[tokio::test]
async fn outbound_user_agent_is_sent_for_create_update_and_preview() {
    let server = MockServer::start().await;
    for (route, obj) in [
        ("", json!({"id":7,"userAgent":"custom/private-agent"})),
        ("/7", Value::Null),
        ("/parse", json!([])),
    ] {
        Mock::given(method("POST"))
            .and(path(format!("/secret/panel/api/xray/outbound-subs{route}")))
            .and(|r: &wiremock::Request| {
                url::form_urlencoded::parse(&r.body)
                    .any(|(k, v)| k == "userAgent" && v == "custom/private-agent")
            })
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"success":true,"obj":obj})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    let mut input = xui_rs::OutboundSubscriptionInput::new("https://provider.example/sub");
    input.user_agent = "custom/private-agent".into();
    let output = client
        .xray_settings()
        .create_outbound_subscription(&input)
        .await
        .unwrap();
    assert_eq!(output.user_agent, input.user_agent);
    assert!(!format!("{input:?} {output:?}").contains("private-agent"));
    client
        .xray_settings()
        .update_outbound_subscription(7, &input)
        .await
        .unwrap();
    client
        .xray_settings()
        .parse_outbound_subscription_with_user_agent(&input.url, false, false, &input.user_agent)
        .await
        .unwrap();
}

#[tokio::test]
async fn happ_headers_and_absent_profile_url_are_preserved_without_debug_leaks() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/sub/id"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("New-Url", "https://example/new-private-sub")
                .insert_header("ProviderID", "provider")
                .insert_header("Subscription-Always-Hwid-Enable", "1")
                .insert_header("Exclude-Apns-Enable", "true")
                .insert_header("Subscription-Autoconnect-Type", "lowestdelay")
                .insert_header("Per-App-Proxy-List", "private-app-list"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let metadata = SubscriptionClient::new(server.uri())
        .unwrap()
        .raw_metadata("id")
        .await
        .unwrap();
    assert!(metadata.profile_web_page_url().is_none());
    assert_eq!(
        metadata.happ.new_url.as_deref(),
        Some("https://example/new-private-sub")
    );
    assert_eq!(metadata.happ.provider_id.as_deref(), Some("provider"));
    assert!(metadata.happ.always_hwid && metadata.happ.exclude_apns);
    assert_eq!(
        metadata.happ.autoconnect_type.as_deref(),
        Some("lowestdelay")
    );
    assert_eq!(
        metadata.happ.per_app_proxy_list.as_deref(),
        Some("private-app-list")
    );
    assert!(!format!("{metadata:?}").contains("private"));
}
