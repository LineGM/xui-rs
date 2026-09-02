#![allow(missing_docs)]

use reqwest::Method;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
use xui_rs::{
    Client, ClientIpObservation, ClientIpRecord, Error, HistoryBucket, LogLevel, PanelLogRequest,
    PanelUpdateState, ProcessState, RealityScanRequest, SystemMetric, XrayLogRequest, XrayMetric,
};

async fn mount_json(server: &MockServer, method: Method, path: &str, object: Option<Value>) {
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

fn reality_json() -> Value {
    json!({
        "target": "example.com:443",
        "host": "example.com",
        "ip": "203.0.113.1",
        "port": 443,
        "feasible": true,
        "TLS13": true,
        "TLSVersion": "1.3",
        "H2": true,
        "ALPN": "h2",
        "X25519": true,
        "curveID": "X25519",
        "certValid": true,
        "certSubject": "example.com",
        "certIssuer": "Test CA",
        "notAfter": "2027-01-01T00:00:00Z",
        "serverNames": ["example.com"],
        "latencyMs": 42,
        "reason": ""
    })
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn every_v370_server_route_is_wired() {
    let server = MockServer::start().await;
    let status = json!({
        "cpu": 12.5,
        "cpuCores": 4,
        "logicalPro": 8,
        "cpuSpeedMhz": 3200.0,
        "mem": {"current": 10, "total": 20},
        "swap": {"current": 1, "total": 2},
        "disk": {"current": 30, "total": 40},
        "diskIO": {"read": 50, "write": 60},
        "diskTraffic": {"read": 5, "write": 6},
        "xray": {"state": "running", "errorMsg": "", "version": "v25.8.3"},
        "panelVersion": "v3.7.0",
        "panelGuid": "panel-guid",
        "uptime": 100,
        "loads": [0.1, 0.2, 0.3],
        "tcpCount": 7,
        "udpCount": 8,
        "netIO": {"up": 9, "down": 10, "pktUp": 11, "pktDown": 12},
        "netTraffic": {"sent": 13, "recv": 14, "pktSent": 15, "pktRecv": 16},
        "publicIP": {"ipv4": "203.0.113.1", "ipv6": "2001:db8::1"},
        "appStats": {"threads": 17, "mem": 18, "uptime": 19}
    });
    let point = json!([{"t": 1_700_000_000, "v": 12.5}]);
    let client_ips = json!([{
        "id": 1,
        "clientEmail": "alice@example.com",
        "ips": [{"ip": "198.51.100.10", "timestamp": 1_700_000_000}]
    }]);

    let routes = [
        (Method::GET, "/panel/api/server/status", Some(status)),
        (
            Method::GET,
            "/panel/api/server/cpuHistory/2",
            Some(json!([{"t": 1_700_000_000, "cpu": 12.5}])),
        ),
        (
            Method::GET,
            "/panel/api/server/history/cpu/2",
            Some(point.clone()),
        ),
        (
            Method::GET,
            "/panel/api/server/xrayMetricsState",
            Some(json!({"enabled": true, "listen": "127.0.0.1:11111"})),
        ),
        (
            Method::GET,
            "/panel/api/server/xrayMetricsHistory/xrAlloc/2",
            Some(point.clone()),
        ),
        (
            Method::GET,
            "/panel/api/server/xrayObservatory",
            Some(json!([{
                "tag": "proxy", "alive": true, "delay": 20,
                "lastSeenTime": 1, "lastTryTime": 2, "updatedAt": 3
            }])),
        ),
        (
            Method::GET,
            "/panel/api/server/xrayObservatoryHistory/proxy/2",
            Some(point),
        ),
        (
            Method::GET,
            "/panel/api/server/getXrayVersion",
            Some(json!(["v25.8.3"])),
        ),
        (
            Method::GET,
            "/panel/api/server/getPanelUpdateInfo",
            Some(json!({
                "channel": "stable", "currentVersion": "v3.7.0",
                "latestVersion": "v3.6.1", "updateAvailable": true
            })),
        ),
        (
            Method::GET,
            "/panel/api/server/getUpdateStatus",
            Some(json!({
                "runId": "1735689600123456789", "state": "success",
                "exitCode": 0, "finishedAt": 1_735_689_612_i64
            })),
        ),
        (
            Method::GET,
            "/panel/api/server/getConfigJson",
            Some(json!({"inbounds": [], "outbounds": []})),
        ),
        (
            Method::GET,
            "/panel/api/server/getNewUUID",
            Some(json!({"uuid": "550e8400-e29b-41d4-a716-446655440000"})),
        ),
        (
            Method::GET,
            "/panel/api/server/getWebCertFiles",
            Some(json!({"webCertFile": "/cert.pem", "webKeyFile": "/key.pem"})),
        ),
        (
            Method::GET,
            "/panel/api/server/descendants",
            Some(json!([{
                "guid": "child", "parentGuid": "parent", "name": "node",
                "address": "node.example", "scheme": "https", "port": 443,
                "status": "online", "lastHeartbeat": 1, "latencyMs": 2,
                "panelVersion": "v3.7.0", "xrayVersion": "v25.8.3",
                "xrayState": "running"
            }])),
        ),
        (
            Method::GET,
            "/panel/api/server/getNewX25519Cert",
            Some(json!({"privateKey": "private", "publicKey": "public"})),
        ),
        (
            Method::GET,
            "/panel/api/server/getNewmldsa65",
            Some(json!({"seed": "seed", "verify": "verify"})),
        ),
        (
            Method::GET,
            "/panel/api/server/getNewmlkem768",
            Some(json!({"seed": "seed", "client": "client"})),
        ),
        (
            Method::GET,
            "/panel/api/server/getNewVlessEnc",
            Some(json!({"auths": [{
                "id": "x25519", "label": "X25519",
                "encryption": "enc", "decryption": "dec"
            }]})),
        ),
        (Method::GET, "/panel/api/server/clientIps", Some(client_ips)),
        (
            Method::GET,
            "/panel/api/server/fail2banStatus",
            Some(json!({
                "enabled": true, "installed": true, "usable": true, "windows": false
            })),
        ),
        (Method::POST, "/panel/api/server/stopXrayService", None),
        (Method::POST, "/panel/api/server/restartXrayService", None),
        (Method::POST, "/panel/api/server/installXray/latest", None),
        (
            Method::POST,
            "/panel/api/server/updatePanel",
            Some(json!({"runId": "1735689600123456789"})),
        ),
        (Method::POST, "/panel/api/server/setUpdateChannel", None),
        (Method::POST, "/panel/api/server/updateGeofile", None),
        (Method::POST, "/panel/api/server/updateGeofile/geoip", None),
        (
            Method::POST,
            "/panel/api/server/logs/50",
            Some(json!(["panel log"])),
        ),
        (
            Method::POST,
            "/panel/api/server/xraylogs/60",
            Some(json!([{
                "DateTime": "2026-08-27T12:00:00Z",
                "FromAddress": "198.51.100.10:1234",
                "ToAddress": "example.com:443",
                "Inbound": "vless-in",
                "Outbound": "proxy",
                "Email": "alice@example.com",
                "Event": 2
            }])),
        ),
        (
            Method::POST,
            "/panel/api/server/amneziawglogs/25",
            Some(json!({
                "peers": [{
                    "interface": "awg1", "tag": "inbound-51820", "inboundId": 1,
                    "email": "alice@example.com", "endpoint": "203.0.113.9:51820",
                    "allowedIPs": "10.8.1.2/32", "handshake": 1_735_732_800_000_i64,
                    "up": 1024, "down": 2048, "online": true
                }],
                "events": ["amneziawg: started awg1"],
                "running": true
            })),
        ),
        (
            Method::POST,
            "/panel/api/server/getNewEchCert",
            Some(json!({"echServerKeys": "secret", "echConfigList": "public"})),
        ),
        (
            Method::POST,
            "/panel/api/server/getCertHash",
            Some(json!(["abc123"])),
        ),
        (
            Method::POST,
            "/panel/api/server/getRemoteCertHash",
            Some(json!(["def456"])),
        ),
        (
            Method::POST,
            "/panel/api/server/scanRealityTarget",
            Some(reality_json()),
        ),
        (Method::POST, "/panel/api/server/clientIps", None),
    ];
    for (method, path, object) in routes {
        mount_json(&server, method, path, object).await;
    }

    for body in ["targets=example.com%3A443", "targets="] {
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/panel/api/server/scanRealityTargets"))
            .and(matchers::header("authorization", "Bearer api-secret"))
            .and(matchers::body_string(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "msg": "success",
                "obj": [reality_json()]
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    for (path, filename, bytes) in [
        ("/panel/api/server/getDb", "x-ui.db", b"database".as_slice()),
        (
            "/panel/api/server/getMigration",
            "migration.dump",
            b"migration".as_slice(),
        ),
    ] {
        Mock::given(matchers::method("GET"))
            .and(matchers::path(path))
            .and(matchers::header("authorization", "Bearer api-secret"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/octet-stream")
                    .insert_header(
                        "content-disposition",
                        format!("attachment; filename=\"{filename}\""),
                    )
                    .set_body_bytes(bytes),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/server/importDB"))
        .and(matchers::header("authorization", "Bearer api-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"success": true, "obj": "imported"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    let api = client.server();

    let status = api.status().await.unwrap();
    assert_eq!(status.xray.state, ProcessState::Running);
    assert_eq!(status.net_io.pkt_down, 12);
    let cpu = api
        .legacy_cpu_history(HistoryBucket::Minutes2)
        .await
        .unwrap()[0]
        .cpu;
    assert!((cpu - 12.5).abs() < f64::EPSILON);
    api.system_history(SystemMetric::Cpu, HistoryBucket::Minutes2)
        .await
        .unwrap();
    assert!(api.xray_metrics_state().await.unwrap().enabled);
    api.xray_metrics_history(XrayMetric::Alloc, HistoryBucket::Minutes2)
        .await
        .unwrap();
    api.xray_observatory().await.unwrap();
    api.xray_observatory_history("proxy", HistoryBucket::Minutes2)
        .await
        .unwrap();
    api.xray_versions().await.unwrap();
    assert!(api.panel_update_info().await.unwrap().update_available);
    assert_eq!(
        api.panel_update_status().await.unwrap().state,
        PanelUpdateState::Success
    );
    api.xray_config().await.unwrap();
    assert_eq!(
        api.download_database().await.unwrap().filename.as_deref(),
        Some("x-ui.db")
    );
    assert_eq!(api.download_migration().await.unwrap().bytes, b"migration");
    assert_eq!(
        api.generate_uuid().await.unwrap(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    api.web_certificate_files().await.unwrap();
    api.descendants().await.unwrap();
    api.generate_x25519().await.unwrap();
    api.generate_mldsa65().await.unwrap();
    api.generate_mlkem768().await.unwrap();
    api.generate_vless_encryption().await.unwrap();
    api.client_ips().await.unwrap();
    api.fail2ban_status().await.unwrap();
    api.stop_xray().await.unwrap();
    api.restart_xray().await.unwrap();
    api.install_xray("latest").await.unwrap();
    api.update_panel(Some(true)).await.unwrap();
    api.set_update_channel(false).await.unwrap();
    api.update_geofiles().await.unwrap();
    api.update_geofile("geoip").await.unwrap();
    assert_eq!(
        api.panel_logs(&PanelLogRequest {
            count: 50,
            level: LogLevel::Info,
            syslog: false,
        })
        .await
        .unwrap(),
        vec!["panel log"]
    );
    assert_eq!(
        api.xray_logs(&XrayLogRequest {
            count: 60,
            filter: "accepted".to_owned(),
            show_direct: true,
            show_blocked: false,
            show_proxy: true,
        })
        .await
        .unwrap(),
        vec![xui_rs::XrayLogEntry {
            date_time: "2026-08-27T12:00:00Z".to_owned(),
            from_address: "198.51.100.10:1234".to_owned(),
            to_address: "example.com:443".to_owned(),
            inbound: "vless-in".to_owned(),
            outbound: "proxy".to_owned(),
            email: "alice@example.com".to_owned(),
            event: xui_rs::XrayLogEvent::Proxied,
        }]
    );
    let awg = api.amneziawg_logs(25, "alice").await.unwrap();
    assert!(awg.running);
    assert_eq!(awg.peers[0].interface_name, "awg1");
    api.import_database("restore.db", b"restore").await.unwrap();
    api.generate_ech("example.com").await.unwrap();
    api.certificate_content_hashes("certificate").await.unwrap();
    api.remote_certificate_hashes("example.com:443")
        .await
        .unwrap();
    let scan = api
        .scan_reality_target(&RealityScanRequest::new("example.com:443"))
        .await
        .unwrap();
    assert!(scan.tls13);
    api.scan_reality_targets(&["example.com:443"])
        .await
        .unwrap();
    api.scan_default_reality_targets().await.unwrap();
    api.merge_client_ips(&[ClientIpRecord {
        id: 0,
        client_email: "alice@example.com".to_owned(),
        ips: vec![ClientIpObservation {
            ip: "198.51.100.10".to_owned(),
            timestamp: 1_700_000_000,
        }],
    }])
    .await
    .unwrap();
}

#[tokio::test]
async fn form_and_multipart_wire_formats_match_the_go_controller() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/server/getCertHash"))
        .and(matchers::body_string("certFile=%2Fcert.pem&certContent="))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "obj": ["hash"]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/panel/api/server/getDb"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "msg": "backup failed"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/panel/api/server/importDB"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder(server.uri())
        .unwrap()
        .bearer_token("api-secret")
        .build()
        .unwrap();
    client
        .server()
        .certificate_file_hashes("/cert.pem")
        .await
        .unwrap();
    client
        .server()
        .import_database("restore.db", b"db-bytes")
        .await
        .unwrap();
    assert!(matches!(
        client.server().download_database().await.unwrap_err(),
        Error::Api { message, .. } if message == "backup failed"
    ));
}

#[test]
fn null_go_slices_decode_as_empty_and_generated_secrets_are_redacted() {
    let records: Vec<ClientIpRecord> = serde_json::from_value(json!([{
        "id": 1,
        "clientEmail": "alice",
        "ips": null
    }]))
    .unwrap();
    assert!(records[0].ips.is_empty());

    let pair: xui_rs::X25519KeyPair = serde_json::from_value(json!({
        "privateKey": "private-secret",
        "publicKey": "public"
    }))
    .unwrap();
    let debug = format!("{pair:?}");
    assert!(!debug.contains("private-secret"));
    assert!(debug.contains("[REDACTED]"));

    let config: xui_rs::XrayConfig = serde_json::from_value(json!({
        "privateKey": "config-secret"
    }))
    .unwrap();
    assert!(!format!("{config:?}").contains("config-secret"));

    let database = xui_rs::DatabaseFile {
        filename: Some("backup.db".to_owned()),
        content_type: Some("application/octet-stream".to_owned()),
        bytes: b"database-secret".to_vec(),
    };
    let debug = format!("{database:?}");
    assert!(!debug.contains("database-secret"));
    assert!(debug.contains("15 bytes"));
}
