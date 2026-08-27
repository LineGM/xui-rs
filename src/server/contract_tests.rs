use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, Option<&str>)] = &[
    (
        "get",
        "/panel/api/server/clientIps",
        Some("get_panel_api_server_clientIps"),
    ),
    (
        "post",
        "/panel/api/server/clientIps",
        Some("post_panel_api_server_clientIps"),
    ),
    (
        "get",
        "/panel/api/server/cpuHistory/{bucket}",
        Some("get_panel_api_server_cpuHistory_bucket"),
    ),
    (
        "get",
        "/panel/api/server/descendants",
        Some("get_panel_api_server_descendants"),
    ),
    (
        "get",
        "/panel/api/server/fail2banStatus",
        Some("get_panel_api_server_fail2banStatus"),
    ),
    (
        "post",
        "/panel/api/server/getCertHash",
        Some("post_panel_api_server_getCertHash"),
    ),
    (
        "get",
        "/panel/api/server/getConfigJson",
        Some("get_panel_api_server_getConfigJson"),
    ),
    (
        "get",
        "/panel/api/server/getDb",
        Some("get_panel_api_server_getDb"),
    ),
    (
        "get",
        "/panel/api/server/getMigration",
        Some("get_panel_api_server_getMigration"),
    ),
    (
        "post",
        "/panel/api/server/getNewEchCert",
        Some("post_panel_api_server_getNewEchCert"),
    ),
    (
        "get",
        "/panel/api/server/getNewUUID",
        Some("get_panel_api_server_getNewUUID"),
    ),
    (
        "get",
        "/panel/api/server/getNewVlessEnc",
        Some("get_panel_api_server_getNewVlessEnc"),
    ),
    (
        "get",
        "/panel/api/server/getNewX25519Cert",
        Some("get_panel_api_server_getNewX25519Cert"),
    ),
    (
        "get",
        "/panel/api/server/getNewmldsa65",
        Some("get_panel_api_server_getNewmldsa65"),
    ),
    (
        "get",
        "/panel/api/server/getNewmlkem768",
        Some("get_panel_api_server_getNewmlkem768"),
    ),
    (
        "get",
        "/panel/api/server/getPanelUpdateInfo",
        Some("get_panel_api_server_getPanelUpdateInfo"),
    ),
    ("get", "/panel/api/server/getUpdateStatus", None),
    (
        "post",
        "/panel/api/server/getRemoteCertHash",
        Some("post_panel_api_server_getRemoteCertHash"),
    ),
    (
        "get",
        "/panel/api/server/getWebCertFiles",
        Some("get_panel_api_server_getWebCertFiles"),
    ),
    (
        "get",
        "/panel/api/server/getXrayVersion",
        Some("get_panel_api_server_getXrayVersion"),
    ),
    (
        "get",
        "/panel/api/server/history/{metric}/{bucket}",
        Some("get_panel_api_server_history_metric_bucket"),
    ),
    (
        "post",
        "/panel/api/server/importDB",
        Some("post_panel_api_server_importDB"),
    ),
    (
        "post",
        "/panel/api/server/installXray/{version}",
        Some("post_panel_api_server_installXray_version"),
    ),
    (
        "post",
        "/panel/api/server/logs/{count}",
        Some("post_panel_api_server_logs_count"),
    ),
    (
        "post",
        "/panel/api/server/restartXrayService",
        Some("post_panel_api_server_restartXrayService"),
    ),
    ("post", "/panel/api/server/scanRealityTarget", None),
    ("post", "/panel/api/server/scanRealityTargets", None),
    (
        "post",
        "/panel/api/server/setUpdateChannel",
        Some("post_panel_api_server_setUpdateChannel"),
    ),
    (
        "get",
        "/panel/api/server/status",
        Some("get_panel_api_server_status"),
    ),
    (
        "post",
        "/panel/api/server/stopXrayService",
        Some("post_panel_api_server_stopXrayService"),
    ),
    (
        "post",
        "/panel/api/server/updateGeofile",
        Some("post_panel_api_server_updateGeofile"),
    ),
    (
        "post",
        "/panel/api/server/updateGeofile/{fileName}",
        Some("post_panel_api_server_updateGeofile_fileName"),
    ),
    (
        "post",
        "/panel/api/server/updatePanel",
        Some("post_panel_api_server_updatePanel"),
    ),
    (
        "get",
        "/panel/api/server/xrayMetricsHistory/{metric}/{bucket}",
        Some("get_panel_api_server_xrayMetricsHistory_metric_bucket"),
    ),
    (
        "get",
        "/panel/api/server/xrayMetricsState",
        Some("get_panel_api_server_xrayMetricsState"),
    ),
    (
        "get",
        "/panel/api/server/xrayObservatory",
        Some("get_panel_api_server_xrayObservatory"),
    ),
    (
        "get",
        "/panel/api/server/xrayObservatoryHistory/{tag}/{bucket}",
        Some("get_panel_api_server_xrayObservatoryHistory_tag_bucket"),
    ),
    (
        "post",
        "/panel/api/server/xraylogs/{count}",
        Some("post_panel_api_server_xraylogs_count"),
    ),
];

#[test]
fn sdk_covers_openapi_and_source_routes() {
    let openapi: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.6.0.openapi.json")).unwrap();
    let paths = openapi["paths"].as_object().unwrap();
    let documented = paths
        .iter()
        .flat_map(|(path, item)| {
            item.as_object().into_iter().flat_map(move |operations| {
                operations
                    .iter()
                    .filter(|(_, operation)| {
                        operation["tags"]
                            .as_array()
                            .is_some_and(|tags| tags.iter().any(|tag| tag == "Server"))
                    })
                    .map(move |(method, operation)| {
                        (
                            method.as_str(),
                            path.as_str(),
                            operation["operationId"].as_str().unwrap(),
                        )
                    })
            })
        })
        .collect::<BTreeSet<_>>();
    let implemented_openapi = SDK_ROUTES
        .iter()
        .filter_map(|(method, path, operation)| {
            operation.map(|operation| (*method, *path, operation))
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(documented.len(), 35);
    assert_eq!(documented, implemented_openapi);

    let source: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.6.0.server-routes.json")).unwrap();
    let source_routes = source["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|route| {
            (
                route["method"].as_str().unwrap(),
                route["path"].as_str().unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    let implemented_routes = SDK_ROUTES
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .collect::<BTreeSet<_>>();
    assert_eq!(source_routes.len(), 38);
    assert_eq!(source_routes, implemented_routes);
}
