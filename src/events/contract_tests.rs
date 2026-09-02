use std::collections::BTreeSet;

use serde_json::Value;

use super::EventMessageType;

const SDK_MESSAGE_TYPES: &[EventMessageType] = &[
    EventMessageType::Status,
    EventMessageType::Traffic,
    EventMessageType::Inbounds,
    EventMessageType::Outbounds,
    EventMessageType::Nodes,
    EventMessageType::Notification,
    EventMessageType::XrayState,
    EventMessageType::ClientStats,
    EventMessageType::Clients,
    EventMessageType::Invalidate,
];

#[test]
fn sdk_covers_websocket_route_and_every_source_message_type() {
    let source: Value = serde_json::from_str(include_str!(
        "../../spec/3x-ui-v3.7.0.websocket-contract.json"
    ))
    .unwrap();
    assert_eq!(source["route"]["method"], "get");
    assert_eq!(source["route"]["path"], "/ws");
    assert_eq!(source["route"]["authentication"], "session-cookie");
    assert_eq!(source["route"]["bearerTokenSupported"], false);
    assert_eq!(
        source["envelopeFields"],
        serde_json::json!(["type", "payload", "time"])
    );
    assert_eq!(source["maxMessageBytes"], 10 * 1024 * 1024);

    let source_types = source["messageTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| message["type"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let sdk_types = SDK_MESSAGE_TYPES
        .iter()
        .map(EventMessageType::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(source_types.len(), 10);
    assert_eq!(source_types, sdk_types);

    let openapi: Value =
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.7.0.openapi.json")).unwrap();
    assert_eq!(openapi["paths"]["/ws"]["get"]["operationId"], "get_ws");
    let documented_messages = openapi["paths"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|path| path.get("ws"))
        .map(|operation| operation["operationId"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        documented_messages,
        BTreeSet::from([
            "ws_type_invalidate",
            "ws_type_notification",
            "ws_type_status",
            "ws_type_xrayState",
        ])
    );
}
