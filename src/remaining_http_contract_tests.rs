use std::collections::BTreeSet;

use serde_json::Value;

const SDK_ROUTES: &[(&str, &str, Option<&str>)] = &[
    ("get", "/panel/api/openapi.json", None),
    (
        "post",
        "/panel/api/backuptotgbot",
        Some("post_panel_api_backuptotgbot"),
    ),
    ("get", "/{subPath}{subid}", Some("get_subPath_subid")),
    ("head", "/{subPath}{subid}", None),
    ("get", "/{jsonPath}{subid}", Some("get_jsonPath_subid")),
    ("head", "/{jsonPath}{subid}", None),
    ("get", "/{clashPath}{subid}", Some("get_clashPath_subid")),
    ("head", "/{clashPath}{subid}", None),
];

#[test]
fn sdk_covers_every_remaining_openapi_and_source_http_route() {
    let openapi: Value =
        serde_json::from_str(include_str!("../spec/3x-ui-v3.6.0.openapi.json")).unwrap();
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
                                matches!(tag.as_str(), Some("Backup" | "Subscription Server"))
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
    assert_eq!(documented.len(), 4);
    assert_eq!(documented, implemented_openapi);

    let snapshot: Value = serde_json::from_str(include_str!(
        "../spec/3x-ui-v3.6.0.remaining-http-routes.json"
    ))
    .unwrap();
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
    assert_eq!(source.len(), 8);
    assert_eq!(source, implemented);
}
