use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, &str)] = &[
    (
        "get",
        "/panel/api/sub-balancers",
        "get_panel_api_sub_balancers",
    ),
    (
        "post",
        "/panel/api/sub-balancers",
        "post_panel_api_sub_balancers",
    ),
    (
        "post",
        "/panel/api/sub-balancers/{id}",
        "post_panel_api_sub_balancers_id",
    ),
    (
        "delete",
        "/panel/api/sub-balancers/{id}",
        "delete_panel_api_sub_balancers_id",
    ),
    (
        "post",
        "/panel/api/sub-balancers/{id}/del",
        "post_panel_api_sub_balancers_id_del",
    ),
];

#[test]
fn sdk_covers_every_openapi_and_source_route() {
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
                        operation["tags"].as_array().is_some_and(|tags| {
                            tags.iter().any(|tag| tag == "Subscription Balancers")
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
    assert_eq!(documented, SDK_ROUTES.iter().copied().collect());

    let source: Value = serde_json::from_str(include_str!(
        "../../spec/3x-ui-v3.7.0.subscription-balancers-routes.json"
    ))
    .unwrap();
    let source = source["routes"]
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
    assert_eq!(source, implemented);
}
