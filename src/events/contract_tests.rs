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

/// Pins WebSocket authentication, envelope fields, size limits, and event names to upstream.
/// Distinguishes source-only events from the broadcasts listed in `OpenAPI`.
#[test]
fn sdk_covers_websocket_route_and_every_source_message_type() {
    let source: Value = serde_json::from_str(include_str!(
        "../../spec/3x-ui-v3.8.5.websocket-contract.json"
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
        serde_json::from_str(include_str!("../../spec/3x-ui-v3.8.5.openapi.json")).unwrap();
    assert_eq!(openapi["paths"]["/ws"]["get"]["operationId"], "get_ws");
    assert!(
        openapi["paths"]
            .as_object()
            .unwrap()
            .values()
            .all(|path| path.get("ws").is_none())
    );
    let documented_messages = openapi["paths"]["/ws"]["get"]["x-websocket-events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let broadcast_types = source_types
        .into_iter()
        .filter(|name| *name != "clients")
        .collect::<BTreeSet<_>>();
    assert_eq!(documented_messages, broadcast_types);
    let envelope = &openapi["components"]["schemas"]["WebSocketEnvelope"]["properties"];
    assert!(envelope.get("payload").is_some());
    assert_eq!(envelope["time"]["type"], "integer");
}
