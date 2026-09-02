use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, Option<&str>)] = &[
    (
        "get",
        "/panel/api/hosts/list",
        Some("get_panel_api_hosts_list"),
    ),
    (
        "get",
        "/panel/api/hosts/get/{groupId}",
        Some("get_panel_api_hosts_get_groupId"),
    ),
    (
        "get",
        "/panel/api/hosts/byInbound/{inboundId}",
        Some("get_panel_api_hosts_byInbound_inboundId"),
    ),
    (
        "get",
        "/panel/api/hosts/tags",
        Some("get_panel_api_hosts_tags"),
    ),
    (
        "post",
        "/panel/api/hosts/add",
        Some("post_panel_api_hosts_add"),
    ),
    (
        "post",
        "/panel/api/hosts/update/{groupId}",
        Some("post_panel_api_hosts_update_groupId"),
    ),
    (
        "post",
        "/panel/api/hosts/del/{groupId}",
        Some("post_panel_api_hosts_del_groupId"),
    ),
    (
        "post",
        "/panel/api/hosts/setEnable/{groupId}",
        Some("post_panel_api_hosts_setEnable_groupId"),
    ),
    (
        "post",
        "/panel/api/hosts/reorder",
        Some("post_panel_api_hosts_reorder"),
    ),
    (
        "post",
        "/panel/api/hosts/bulk/add",
        Some("post_panel_api_hosts_bulk_add"),
    ),
    (
        "post",
        "/panel/api/hosts/bulk/setEnable",
        Some("post_panel_api_hosts_bulk_setEnable"),
    ),
    (
        "post",
        "/panel/api/hosts/bulk/del",
        Some("post_panel_api_hosts_bulk_del"),
    ),
];

#[test]
fn sdk_covers_openapi_and_source_routes() {
    let openapi: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.7.0.openapi.json")).unwrap();
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
                            .is_some_and(|tags| tags.iter().any(|tag| tag == "Hosts"))
                    })
                    .map(move |(method, operation)| {
                        (
                            method.as_str(),
                            canonical_path(path),
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
    assert_eq!(documented.len(), 12);
    assert_eq!(documented, implemented_openapi);

    let snapshot: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.7.0.hosts-routes.json")).unwrap();
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
    assert_eq!(source.len(), 12);
    assert_eq!(source, implemented);
}

fn canonical_path(path: &str) -> &str {
    match path {
        "/panel/api/hosts/get/{id}" => "/panel/api/hosts/get/{groupId}",
        "/panel/api/hosts/update/{id}" => "/panel/api/hosts/update/{groupId}",
        "/panel/api/hosts/del/{id}" => "/panel/api/hosts/del/{groupId}",
        "/panel/api/hosts/setEnable/{id}" => "/panel/api/hosts/setEnable/{groupId}",
        _ => path,
    }
}
