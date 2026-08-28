use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, &str)] = &[
    ("get", "/panel/api/nodes/list", "get_panel_api_nodes_list"),
    (
        "post",
        "/panel/api/nodes/mtls/ca",
        "post_panel_api_nodes_mtls_ca",
    ),
    (
        "post",
        "/panel/api/nodes/mtls/trustCA",
        "post_panel_api_nodes_mtls_trustCA",
    ),
    (
        "get",
        "/panel/api/nodes/get/{id}",
        "get_panel_api_nodes_get_id",
    ),
    (
        "get",
        "/panel/api/nodes/webCert/{id}",
        "get_panel_api_nodes_webCert_id",
    ),
    ("post", "/panel/api/nodes/add", "post_panel_api_nodes_add"),
    (
        "post",
        "/panel/api/nodes/update/{id}",
        "post_panel_api_nodes_update_id",
    ),
    (
        "post",
        "/panel/api/nodes/del/{id}",
        "post_panel_api_nodes_del_id",
    ),
    (
        "post",
        "/panel/api/nodes/setEnable/{id}",
        "post_panel_api_nodes_setEnable_id",
    ),
    ("post", "/panel/api/nodes/test", "post_panel_api_nodes_test"),
    (
        "post",
        "/panel/api/nodes/certFingerprint",
        "post_panel_api_nodes_certFingerprint",
    ),
    (
        "post",
        "/panel/api/nodes/inbounds",
        "post_panel_api_nodes_inbounds",
    ),
    (
        "post",
        "/panel/api/nodes/probe/{id}",
        "post_panel_api_nodes_probe_id",
    ),
    (
        "post",
        "/panel/api/nodes/updatePanel",
        "post_panel_api_nodes_updatePanel",
    ),
    (
        "get",
        "/panel/api/nodes/history/{id}/{metric}/{bucket}",
        "get_panel_api_nodes_history_id_metric_bucket",
    ),
];

#[test]
fn sdk_covers_every_openapi_and_source_route() {
    let openapi: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.6.0.openapi.json")).unwrap();
    let documented = openapi["paths"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(path, item)| {
            item.as_object().into_iter().flat_map(move |operations| {
                operations
                    .iter()
                    .filter(|(_, operation)| {
                        operation["tags"]
                            .as_array()
                            .is_some_and(|tags| tags.iter().any(|tag| tag == "Nodes"))
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
    let implemented_openapi = SDK_ROUTES.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(documented.len(), 15);
    assert_eq!(documented, implemented_openapi);

    let snapshot: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.6.0.nodes-routes.json")).unwrap();
    let source = snapshot["routes"]
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
    let implemented = SDK_ROUTES
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .collect::<BTreeSet<_>>();
    assert_eq!(source.len(), 15);
    assert_eq!(source, implemented);
}
