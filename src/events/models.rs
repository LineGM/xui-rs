use std::{collections::HashMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    ActiveInboundsByGuid, ClientTraffic, ClientsByGuid, Error, Inbound, NodeView, OutboundTraffic,
    ProcessState, Result, ServerStatus,
};

macro_rules! string_enum {
    ($name:ident { $($wire:literal => $variant:ident),+ $(,)? }) => {
        impl $name {
            /// Returns the exact v3.7.0 wire value.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Other(value) => value,
                }
            }

            fn from_wire(value: String) -> Self {
                match value.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Other(value),
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer).map(Self::from_wire)
            }
        }
    };
}

/// Message names declared by the v3.7.0 WebSocket hub.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum EventMessageType {
    /// Complete machine and Xray status snapshot.
    Status,
    /// Live local/node traffic update.
    Traffic,
    /// Complete inbound-list replacement.
    Inbounds,
    /// Complete outbound-traffic replacement.
    Outbounds,
    /// Complete node-list replacement.
    Nodes,
    /// In-panel operator notification.
    Notification,
    /// Xray process-state transition.
    XrayState,
    /// Absolute client and inbound traffic counters.
    ClientStats,
    /// Reserved client-collection message name.
    Clients,
    /// Instruction to refresh a resource through HTTP.
    Invalidate,
    /// Message introduced by a newer panel version.
    Other(String),
}

string_enum!(EventMessageType {
    "status" => Status,
    "traffic" => Traffic,
    "inbounds" => Inbounds,
    "outbounds" => Outbounds,
    "nodes" => Nodes,
    "notification" => Notification,
    "xray_state" => XrayState,
    "client_stats" => ClientStats,
    "clients" => Clients,
    "invalidate" => Invalidate,
});

/// Severity attached to a panel notification.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NotificationLevel {
    /// Successful operation.
    Success,
    /// Informational notice.
    Info,
    /// Warning that may require attention.
    Warning,
    /// Failed operation or unhealthy state.
    Error,
    /// Level introduced by a newer panel version.
    Other(String),
}

string_enum!(NotificationLevel {
    "success" => Success,
    "info" => Info,
    "warning" => Warning,
    "error" => Error,
});

/// One Xray traffic delta. Its capitalized field names are the exact Go wire
/// representation used by v3.7.0.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct TrafficDelta {
    /// Whether the tag identifies an inbound.
    pub is_inbound: bool,
    /// Whether the tag identifies an outbound.
    pub is_outbound: bool,
    /// Stable Xray tag.
    pub tag: String,
    /// Uploaded bytes during the collection window.
    pub up: i64,
    /// Downloaded bytes during the collection window.
    pub down: i64,
}

/// Partial live-traffic payload emitted independently by the local Xray and
/// remote-node synchronization jobs.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrafficUpdate {
    /// Local Xray inbound/outbound deltas, when this is a local poll.
    pub traffics: Option<Vec<TrafficDelta>>,
    /// Remote-node inbound deltas, when this is a node synchronization poll.
    pub node_traffics: Option<Vec<TrafficDelta>>,
    /// Per-client deltas that moved bytes in the latest local poll.
    pub client_traffics: Option<Vec<ClientTraffic>>,
    /// Globally online client emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub online_clients: Vec<String>,
    /// Online client emails grouped by panel GUID.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub online_by_guid: ClientsByGuid,
    /// Active inbound tags grouped by panel GUID.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub active_inbounds: ActiveInboundsByGuid,
    /// Last-online timestamps in milliseconds keyed by client email.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub last_online_map: HashMap<String, i64>,
}

impl fmt::Debug for TrafficUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrafficUpdate")
            .field("traffics", &self.traffics.as_ref().map(Vec::len))
            .field("node_traffics", &self.node_traffics.as_ref().map(Vec::len))
            .field(
                "client_traffics",
                &self.client_traffics.as_ref().map(Vec::len),
            )
            .field("online_clients", &self.online_clients.len())
            .field("online_by_guid", &self.online_by_guid.len())
            .field("active_inbounds", &self.active_inbounds.len())
            .field("last_online_map", &self.last_online_map.len())
            .finish()
    }
}

/// Absolute traffic summary for one inbound.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct InboundTrafficSummary {
    /// Inbound database identifier.
    pub id: i64,
    /// Cumulative uploaded bytes.
    pub up: i64,
    /// Cumulative downloaded bytes.
    pub down: i64,
    /// Traffic allowance in bytes.
    pub total: i64,
    /// Whether the inbound is enabled.
    pub enable: bool,
}

/// Absolute client/inbound counters emitted after a traffic collection.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientStatsUpdate {
    /// `true` when `clients` replaces the complete client-stat collection;
    /// `false` when it contains only active rows.
    pub snapshot: bool,
    /// Absolute per-client counters.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub clients: Vec<ClientTraffic>,
    /// Absolute per-inbound summaries.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub inbounds: Vec<InboundTrafficSummary>,
}

impl fmt::Debug for ClientStatsUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientStatsUpdate")
            .field("snapshot", &self.snapshot)
            .field("clients", &self.clients.len())
            .field("inbounds", &self.inbounds.len())
            .finish()
    }
}

/// Operator notification pushed by 3x-ui.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelNotification {
    /// Localized notification title.
    pub title: String,
    /// Notification body.
    pub message: String,
    /// Notification severity.
    pub level: NotificationLevel,
}

impl fmt::Debug for PanelNotification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PanelNotification")
            .field("title", &"[REDACTED]")
            .field("message", &"[REDACTED]")
            .field("level", &self.level)
            .finish()
    }
}

/// Xray process-state transition pushed after lifecycle operations.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XrayStateChange {
    /// New process state.
    pub state: ProcessState,
    /// Startup/reload error, when `state` is `error`.
    pub error_msg: String,
}

impl fmt::Debug for XrayStateChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("XrayStateChange")
            .field("state", &self.state)
            .field("error_msg", &"[REDACTED]")
            .finish()
    }
}

/// Lightweight request to refresh one resource through its typed HTTP API.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Invalidation {
    /// Original data type that could not or should not be pushed directly.
    #[serde(rename = "type")]
    pub target: EventMessageType,
}

/// Decoded payload of one v3.7.0 real-time message.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum PanelEventKind {
    /// Complete server-status snapshot.
    Status(Box<ServerStatus>),
    /// Partial live traffic update.
    Traffic(Box<TrafficUpdate>),
    /// Complete inbound-list replacement.
    Inbounds(Vec<Inbound>),
    /// Complete outbound-traffic replacement.
    Outbounds(Vec<OutboundTraffic>),
    /// Complete node-tree replacement.
    Nodes(Vec<NodeView>),
    /// Operator notification.
    Notification(PanelNotification),
    /// Xray process transition.
    XrayState(XrayStateChange),
    /// Absolute client/inbound counters.
    ClientStats(ClientStatsUpdate),
    /// Reserved source-declared clients message. v3.7.0 has no direct
    /// broadcaster or stable payload schema for this type.
    Clients(Value),
    /// Request to refresh a resource through HTTP.
    Invalidate(Invalidation),
    /// Message introduced by a newer panel version. Its payload is preserved.
    Unknown {
        /// Exact unrecognized message name.
        message_type: String,
        /// Exact unrecognized JSON payload.
        payload: Value,
    },
}

impl PanelEventKind {
    /// Returns the source-defined or forward-compatible message name.
    pub fn message_type(&self) -> EventMessageType {
        match self {
            Self::Status(_) => EventMessageType::Status,
            Self::Traffic(_) => EventMessageType::Traffic,
            Self::Inbounds(_) => EventMessageType::Inbounds,
            Self::Outbounds(_) => EventMessageType::Outbounds,
            Self::Nodes(_) => EventMessageType::Nodes,
            Self::Notification(_) => EventMessageType::Notification,
            Self::XrayState(_) => EventMessageType::XrayState,
            Self::ClientStats(_) => EventMessageType::ClientStats,
            Self::Clients(_) => EventMessageType::Clients,
            Self::Invalidate(_) => EventMessageType::Invalidate,
            Self::Unknown { message_type, .. } => EventMessageType::Other(message_type.clone()),
        }
    }
}

impl fmt::Debug for PanelEventKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status(status) => formatter
                .debug_tuple("Status")
                .field(&status.xray.state)
                .finish(),
            Self::Traffic(update) => formatter.debug_tuple("Traffic").field(update).finish(),
            Self::Inbounds(rows) => formatter
                .debug_struct("Inbounds")
                .field("count", &rows.len())
                .finish(),
            Self::Outbounds(rows) => formatter
                .debug_struct("Outbounds")
                .field("count", &rows.len())
                .finish(),
            Self::Nodes(rows) => formatter
                .debug_struct("Nodes")
                .field("count", &rows.len())
                .finish(),
            Self::Notification(value) => {
                formatter.debug_tuple("Notification").field(value).finish()
            }
            Self::XrayState(value) => formatter.debug_tuple("XrayState").field(value).finish(),
            Self::ClientStats(value) => formatter.debug_tuple("ClientStats").field(value).finish(),
            Self::Clients(_) => formatter.write_str("Clients([REDACTED])"),
            Self::Invalidate(value) => formatter.debug_tuple("Invalidate").field(value).finish(),
            Self::Unknown { message_type, .. } => formatter
                .debug_struct("Unknown")
                .field("message_type", message_type)
                .field("payload", &"[REDACTED]")
                .finish(),
        }
    }
}

/// One typed real-time event from the panel.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelEvent {
    /// Server-generated Unix timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Typed message payload.
    pub kind: PanelEventKind,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    message_type: String,
    payload: Value,
    time: i64,
}

impl PanelEvent {
    pub(crate) fn decode(text: &str) -> Result<Self> {
        let raw: RawEvent = serde_json::from_str(text).map_err(|source| Error::EventDecode {
            message_type: None,
            source,
        })?;
        let message_type = EventMessageType::from_wire(raw.message_type.clone());
        let payload = raw.payload;
        let kind = match message_type {
            EventMessageType::Status => {
                PanelEventKind::Status(Box::new(decode_payload(&raw.message_type, payload)?))
            }
            EventMessageType::Traffic => {
                PanelEventKind::Traffic(Box::new(decode_payload(&raw.message_type, payload)?))
            }
            EventMessageType::Inbounds => {
                PanelEventKind::Inbounds(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::Outbounds => {
                PanelEventKind::Outbounds(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::Nodes => {
                PanelEventKind::Nodes(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::Notification => {
                PanelEventKind::Notification(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::XrayState => {
                PanelEventKind::XrayState(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::ClientStats => {
                PanelEventKind::ClientStats(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::Clients => PanelEventKind::Clients(payload),
            EventMessageType::Invalidate => {
                PanelEventKind::Invalidate(decode_payload(&raw.message_type, payload)?)
            }
            EventMessageType::Other(message_type) => PanelEventKind::Unknown {
                message_type,
                payload,
            },
        };
        Ok(Self {
            timestamp_ms: raw.time,
            kind,
        })
    }
}

fn decode_payload<T>(message_type: &str, payload: Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload).map_err(|source| Error::EventDecode {
        message_type: Some(message_type.to_owned()),
        source,
    })
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extensible_string_enums_round_trip_every_wire_value() {
        let message_types = [
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
            EventMessageType::Other("future_event".into()),
        ];
        for value in message_types {
            let wire = value.as_str().to_owned();
            assert_eq!(serde_json::to_value(&value).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<EventMessageType>(json!(wire)).unwrap(),
                value
            );
        }

        let levels = [
            NotificationLevel::Success,
            NotificationLevel::Info,
            NotificationLevel::Warning,
            NotificationLevel::Error,
            NotificationLevel::Other("critical".into()),
        ];
        for value in levels {
            let wire = value.as_str().to_owned();
            assert_eq!(serde_json::to_value(&value).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<NotificationLevel>(json!(wire)).unwrap(),
                value
            );
        }
    }

    #[test]
    fn event_debug_views_redact_payloads_and_report_shape() {
        let traffic = TrafficUpdate {
            traffics: Some(vec![TrafficDelta::default()]),
            node_traffics: Some(vec![]),
            online_clients: vec!["private@example.com".into()],
            ..TrafficUpdate::default()
        };
        let stats = ClientStatsUpdate {
            snapshot: true,
            inbounds: vec![InboundTrafficSummary::default()],
            ..ClientStatsUpdate::default()
        };
        let notification = PanelNotification {
            title: "private-title".into(),
            message: "private-message".into(),
            level: NotificationLevel::Warning,
        };
        let xray_state = XrayStateChange {
            state: ProcessState::Error,
            error_msg: "private-xray-error".into(),
        };

        assert!(!format!("{traffic:?}").contains("private@example.com"));
        assert!(format!("{stats:?}").contains("snapshot: true"));
        assert!(!format!("{notification:?}").contains("private-title"));
        assert!(!format!("{notification:?}").contains("private-message"));
        assert!(!format!("{xray_state:?}").contains("private-xray-error"));
    }

    #[test]
    fn every_event_kind_reports_its_type_without_exposing_payloads() {
        let values = vec![
            PanelEventKind::Status(Box::default()),
            PanelEventKind::Traffic(Box::default()),
            PanelEventKind::Inbounds(Vec::new()),
            PanelEventKind::Outbounds(Vec::new()),
            PanelEventKind::Nodes(Vec::new()),
            PanelEventKind::Notification(PanelNotification {
                title: "notification-secret".into(),
                message: "message-secret".into(),
                level: NotificationLevel::Info,
            }),
            PanelEventKind::XrayState(XrayStateChange {
                state: ProcessState::Running,
                error_msg: "state-secret".into(),
            }),
            PanelEventKind::ClientStats(ClientStatsUpdate::default()),
            PanelEventKind::Clients(json!({"password": "clients-secret"})),
            PanelEventKind::Invalidate(Invalidation {
                target: EventMessageType::Inbounds,
            }),
            PanelEventKind::Unknown {
                message_type: "future".into(),
                payload: json!({"password": "future-secret"}),
            },
        ];
        let expected = [
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
            EventMessageType::Other("future".into()),
        ];
        for (value, expected) in values.into_iter().zip(expected) {
            assert_eq!(value.message_type(), expected);
            let output = format!("{value:?}");
            for secret in [
                "notification-secret",
                "message-secret",
                "state-secret",
                "clients-secret",
                "future-secret",
            ] {
                assert!(!output.contains(secret));
            }
        }
    }

    #[test]
    fn malformed_envelope_has_no_claimed_message_type() {
        let error = PanelEvent::decode("not-json").unwrap_err();
        assert!(matches!(
            error,
            Error::EventDecode {
                message_type: None,
                ..
            }
        ));
    }
}
