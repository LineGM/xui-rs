use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, Option<&str>)] = &[
    (
        "post",
        "/panel/api/setting/all",
        Some("post_panel_api_setting_all"),
    ),
    (
        "post",
        "/panel/api/setting/defaultSettings",
        Some("post_panel_api_setting_defaultSettings"),
    ),
    ("post", "/panel/api/setting/factoryDefaults", None),
    (
        "post",
        "/panel/api/setting/update",
        Some("post_panel_api_setting_update"),
    ),
    ("post", "/panel/api/setting/validateRegex", None),
    (
        "post",
        "/panel/api/setting/updateUser",
        Some("post_panel_api_setting_updateUser"),
    ),
    (
        "post",
        "/panel/api/setting/restartPanel",
        Some("post_panel_api_setting_restartPanel"),
    ),
    (
        "get",
        "/panel/api/setting/getDefaultJsonConfig",
        Some("get_panel_api_setting_getDefaultJsonConfig"),
    ),
    (
        "get",
        "/panel/api/setting/apiTokens",
        Some("get_panel_api_setting_apiTokens"),
    ),
    (
        "post",
        "/panel/api/setting/apiTokens/create",
        Some("post_panel_api_setting_apiTokens_create"),
    ),
    (
        "post",
        "/panel/api/setting/apiTokens/delete/{id}",
        Some("post_panel_api_setting_apiTokens_delete_id"),
    ),
    (
        "post",
        "/panel/api/setting/apiTokens/setEnabled/{id}",
        Some("post_panel_api_setting_apiTokens_setEnabled_id"),
    ),
    (
        "post",
        "/panel/api/setting/testSmtp",
        Some("post_panel_api_setting_testSmtp"),
    ),
    (
        "post",
        "/panel/api/setting/testTgBot",
        Some("post_panel_api_setting_testTgBot"),
    ),
    (
        "get",
        "/panel/api/xray/getDefaultJsonConfig",
        Some("get_panel_api_xray_getDefaultJsonConfig"),
    ),
    (
        "get",
        "/panel/api/xray/getOutboundsTraffic",
        Some("get_panel_api_xray_getOutboundsTraffic"),
    ),
    (
        "get",
        "/panel/api/xray/getXrayResult",
        Some("get_panel_api_xray_getXrayResult"),
    ),
    ("post", "/panel/api/xray/", Some("post_panel_api_xray")),
    (
        "post",
        "/panel/api/xray/warp/{action}",
        Some("post_panel_api_xray_warp_action"),
    ),
    (
        "post",
        "/panel/api/xray/nord/{action}",
        Some("post_panel_api_xray_nord_action"),
    ),
    (
        "post",
        "/panel/api/xray/update",
        Some("post_panel_api_xray_update"),
    ),
    (
        "post",
        "/panel/api/xray/resetOutboundsTraffic",
        Some("post_panel_api_xray_resetOutboundsTraffic"),
    ),
    (
        "post",
        "/panel/api/xray/testOutbound",
        Some("post_panel_api_xray_testOutbound"),
    ),
    (
        "post",
        "/panel/api/xray/testOutbounds",
        Some("post_panel_api_xray_testOutbounds"),
    ),
    (
        "post",
        "/panel/api/xray/balancerStatus",
        Some("post_panel_api_xray_balancerStatus"),
    ),
    (
        "post",
        "/panel/api/xray/balancerOverride",
        Some("post_panel_api_xray_balancerOverride"),
    ),
    (
        "post",
        "/panel/api/xray/routeTest",
        Some("post_panel_api_xray_routeTest"),
    ),
    (
        "get",
        "/panel/api/xray/outbound-subs",
        Some("get_panel_api_xray_outbound_subs"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs",
        Some("post_panel_api_xray_outbound_subs"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs/{id}/refresh",
        Some("post_panel_api_xray_outbound_subs_id_refresh"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs/{id}/move",
        Some("post_panel_api_xray_outbound_subs_id_move"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs/{id}",
        Some("post_panel_api_xray_outbound_subs_id"),
    ),
    (
        "delete",
        "/panel/api/xray/outbound-subs/{id}",
        Some("delete_panel_api_xray_outbound_subs_id"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs/{id}/del",
        Some("post_panel_api_xray_outbound_subs_id_del"),
    ),
    (
        "post",
        "/panel/api/xray/outbound-subs/parse",
        Some("post_panel_api_xray_outbound_subs_parse"),
    ),
];

#[test]
fn sdk_covers_openapi_and_source_routes() {
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
                        operation["tags"].as_array().is_some_and(|tags| {
                            tags.iter().any(|tag| {
                                matches!(
                                    tag.as_str(),
                                    Some("Settings" | "API Tokens" | "Xray Settings")
                                )
                            })
                        })
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
    assert_eq!(documented.len(), 33);
    assert_eq!(documented, implemented_openapi);

    let snapshot: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.6.0.settings-routes.json")).unwrap();
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
    assert_eq!(source.len(), 35);
    assert_eq!(source, implemented);
}
