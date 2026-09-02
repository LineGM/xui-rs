#![allow(clippy::struct_excessive_bools)]

use std::fmt;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[allow(clippy::ref_option)] // serde's field callback receives `&Option<T>`.
fn serialize_optional_secret<S>(
    value: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(value.expose_secret()),
        None => serializer.serialize_none(),
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's predicate receives `&T`.
const fn is_false(value: &bool) -> bool {
    !*value
}

macro_rules! impl_string_enum {
    ($name:ident { $($wire:literal => $variant:ident),+ $(,)? }) => {
        impl $name {
            /// Returns the exact 3x-ui wire value.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Other(value) => value,
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Ok(match value.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Other(value),
                })
            }
        }
    };
}

/// URL scheme used to contact a remote panel.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeScheme {
    /// Plain HTTP.
    Http,
    /// HTTPS.
    #[default]
    Https,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl_string_enum!(NodeScheme {
    "http" => Http,
    "https" => Https,
});

/// TLS verification/authentication mode for a remote panel.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeTlsVerifyMode {
    /// Verify the node certificate against system roots.
    #[default]
    Verify,
    /// Skip certificate verification.
    Skip,
    /// Verify the exact SHA-256 leaf-certificate fingerprint.
    Pin,
    /// Authenticate with a panel-generated mTLS client certificate.
    Mtls,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl_string_enum!(NodeTlsVerifyMode {
    "verify" => Verify,
    "skip" => Skip,
    "pin" => Pin,
    "mtls" => Mtls,
});

/// Which remote inbounds a node imports into the managing panel.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeInboundSyncMode {
    /// Import all remote inbounds.
    #[default]
    All,
    /// Import only explicitly selected tags.
    Selected,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl_string_enum!(NodeInboundSyncMode {
    "all" => All,
    "selected" => Selected,
});

/// Cached panel reachability state.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeStatus {
    /// No successful heartbeat has established state yet.
    #[default]
    Unknown,
    /// The remote panel API is reachable.
    Online,
    /// The remote panel API is unreachable.
    Offline,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl_string_enum!(NodeStatus {
    "unknown" => Unknown,
    "online" => Online,
    "offline" => Offline,
});

/// Protocol reported by a remote inbound-discovery request.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RemoteInboundProtocol {
    /// No protocol was reported.
    #[default]
    Unknown,
    /// `VMess`.
    Vmess,
    /// VLESS.
    Vless,
    /// Trojan.
    Trojan,
    /// Shadowsocks.
    Shadowsocks,
    /// `WireGuard`.
    Wireguard,
    /// Hysteria.
    Hysteria,
    /// HTTP proxy.
    Http,
    /// Mixed HTTP/SOCKS proxy.
    Mixed,
    /// Tunnel.
    Tunnel,
    /// TUN interface.
    Tun,
    /// `MTProto` proxy.
    Mtproto,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl_string_enum!(RemoteInboundProtocol {
    "" => Unknown,
    "vmess" => Vmess,
    "vless" => Vless,
    "trojan" => Trojan,
    "shadowsocks" => Shadowsocks,
    "wireguard" => Wireguard,
    "hysteria" => Hysteria,
    "http" => Http,
    "mixed" => Mixed,
    "tunnel" => Tunnel,
    "tun" => Tun,
    "mtproto" => Mtproto,
});

/// One browser-safe view of a direct or transitive node.
///
/// The upstream `OpenAPI` schema is stale and includes `apiToken`; the tagged
/// v3.7.0 controller deliberately returns only `hasApiToken`.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeView {
    /// Panel-local database identifier. Transitive projections use zero.
    pub id: i64,
    /// Unique configured name.
    pub name: String,
    /// Operator-visible description.
    pub remark: String,
    /// Remote panel URL scheme.
    pub scheme: NodeScheme,
    /// Hostname or IP address without a port.
    pub address: String,
    /// Remote panel port.
    pub port: u16,
    /// Remote panel base path.
    pub base_path: String,
    /// Whether a write-only API token is stored.
    pub has_api_token: bool,
    /// Whether node synchronization is enabled.
    pub enable: bool,
    /// Whether guarded node connections may resolve to private addresses.
    pub allow_private_address: bool,
    /// Node TLS verification/authentication mode.
    pub tls_verify_mode: NodeTlsVerifyMode,
    /// Base64 SHA-256 leaf-certificate pin.
    pub pinned_cert_sha256: String,
    /// Remote inbound import mode.
    pub inbound_sync_mode: NodeInboundSyncMode,
    /// Remote inbound tags selected for import.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub inbound_tags: Vec<String>,
    /// Local Xray outbound used to reach the node.
    pub outbound_tag: String,
    /// Stable GUID learned from the remote panel.
    pub guid: String,
    /// Cached remote panel reachability.
    pub status: NodeStatus,
    /// Most recent heartbeat Unix timestamp in seconds.
    pub last_heartbeat: i64,
    /// Most recent request latency in milliseconds.
    pub latency_ms: i64,
    /// Remote Xray version.
    pub xray_version: String,
    /// Remote panel version.
    pub panel_version: String,
    /// Remote CPU utilization percentage.
    pub cpu_pct: f64,
    /// Remote memory utilization percentage.
    pub mem_pct: f64,
    /// Remote host uptime in seconds.
    pub uptime_secs: u64,
    /// Remote interface upload throughput in bytes per second.
    pub net_up: u64,
    /// Remote interface download throughput in bytes per second.
    pub net_down: u64,
    /// Last panel connectivity error.
    pub last_error: String,
    /// Remote Xray process state.
    pub xray_state: String,
    /// Remote Xray process error.
    pub xray_error: String,
    /// Whether node-backed configuration needs reconciliation.
    pub config_dirty: bool,
    /// Unix timestamp associated with the dirty configuration generation.
    pub config_dirty_at: i64,
    /// Number of imported inbounds hosted by the node.
    pub inbound_count: i64,
    /// Number of clients hosted by the node.
    pub client_count: i64,
    /// Number of currently online clients.
    pub online_count: i64,
    /// Number of active clients.
    pub active_count: i64,
    /// Number of disabled clients.
    pub disabled_count: i64,
    /// Number of depleted clients.
    pub depleted_count: i64,
    /// Stable GUID of the managing parent panel/node.
    pub parent_guid: String,
    /// Whether this is a read-only downstream projection.
    pub transitive: bool,
    /// Database creation timestamp in milliseconds.
    pub created_at: i64,
    /// Database update timestamp in milliseconds.
    pub updated_at: i64,
}

impl NodeView {
    /// Builds a full replacement request while retaining the stored API token.
    pub fn to_request(&self) -> NodeRequest {
        NodeRequest {
            id: self.id,
            name: self.name.clone(),
            remark: self.remark.clone(),
            scheme: self.scheme.clone(),
            address: self.address.clone(),
            port: self.port,
            base_path: self.base_path.clone(),
            api_token: None,
            clear_api_token: false,
            enable: self.enable,
            allow_private_address: self.allow_private_address,
            tls_verify_mode: self.tls_verify_mode.clone(),
            pinned_cert_sha256: self.pinned_cert_sha256.clone(),
            inbound_sync_mode: self.inbound_sync_mode.clone(),
            inbound_tags: self.inbound_tags.clone(),
            outbound_tag: self.outbound_tag.clone(),
        }
    }
}

impl fmt::Debug for NodeView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NodeView")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("remark", &self.remark)
            .field("scheme", &self.scheme)
            .field("address", &self.address)
            .field("port", &self.port)
            .field("base_path", &"[REDACTED]")
            .field("has_api_token", &self.has_api_token)
            .field("enable", &self.enable)
            .field("allow_private_address", &self.allow_private_address)
            .field("tls_verify_mode", &self.tls_verify_mode)
            .field("pinned_cert_sha256", &self.pinned_cert_sha256)
            .field("inbound_sync_mode", &self.inbound_sync_mode)
            .field("inbound_tags", &self.inbound_tags)
            .field("outbound_tag", &"[REDACTED]")
            .field("guid", &self.guid)
            .field("status", &self.status)
            .field("last_heartbeat", &self.last_heartbeat)
            .field("latency_ms", &self.latency_ms)
            .field("xray_version", &self.xray_version)
            .field("panel_version", &self.panel_version)
            .field("cpu_pct", &self.cpu_pct)
            .field("mem_pct", &self.mem_pct)
            .field("uptime_secs", &self.uptime_secs)
            .field("net_up", &self.net_up)
            .field("net_down", &self.net_down)
            .field("last_error", &self.last_error)
            .field("xray_state", &self.xray_state)
            .field("xray_error", &self.xray_error)
            .field("config_dirty", &self.config_dirty)
            .field("config_dirty_at", &self.config_dirty_at)
            .field("inbound_count", &self.inbound_count)
            .field("client_count", &self.client_count)
            .field("online_count", &self.online_count)
            .field("active_count", &self.active_count)
            .field("disabled_count", &self.disabled_count)
            .field("depleted_count", &self.depleted_count)
            .field("parent_guid", &self.parent_guid)
            .field("transitive", &self.transitive)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Complete writable node connection and synchronization configuration.
///
/// API tokens are write-only and redacted from `Debug`. The request encodes
/// the controller's three update states without allowing contradictory
/// `apiToken` and `clearApiToken` fields.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRequest {
    /// Existing node ID used when probing an edit form, or zero for a new node.
    pub id: i64,
    /// Unique configured name.
    pub name: String,
    /// Operator-visible description.
    pub remark: String,
    /// Remote panel URL scheme.
    pub scheme: NodeScheme,
    /// Hostname or IP address without a port.
    pub address: String,
    /// Remote panel port.
    pub port: u16,
    /// Remote panel base path.
    pub base_path: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_secret"
    )]
    api_token: Option<SecretString>,
    #[serde(skip_serializing_if = "is_false")]
    clear_api_token: bool,
    /// Whether node synchronization is enabled.
    pub enable: bool,
    /// Whether guarded node connections may resolve to private addresses.
    pub allow_private_address: bool,
    /// Node TLS verification/authentication mode.
    pub tls_verify_mode: NodeTlsVerifyMode,
    /// Base64 SHA-256 leaf-certificate pin.
    pub pinned_cert_sha256: String,
    /// Remote inbound import mode.
    pub inbound_sync_mode: NodeInboundSyncMode,
    /// Remote inbound tags selected for import.
    pub inbound_tags: Vec<String>,
    /// Local Xray outbound used to reach the node.
    pub outbound_tag: String,
}

impl NodeRequest {
    /// Creates an enabled HTTPS node using normal certificate verification.
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            id: 0,
            name: name.into(),
            remark: String::new(),
            scheme: NodeScheme::Https,
            address: address.into(),
            port,
            base_path: "/".to_owned(),
            api_token: None,
            clear_api_token: false,
            enable: true,
            allow_private_address: false,
            tls_verify_mode: NodeTlsVerifyMode::Verify,
            pinned_cert_sha256: String::new(),
            inbound_sync_mode: NodeInboundSyncMode::All,
            inbound_tags: Vec::new(),
            outbound_tag: String::new(),
        }
    }

    /// Configures a replacement API token and returns the request.
    #[must_use]
    pub fn with_api_token(mut self, token: impl Into<String>) -> Self {
        self.set_api_token(token);
        self
    }

    /// Configures a replacement API token.
    pub fn set_api_token(&mut self, token: impl Into<String>) {
        self.api_token = Some(SecretString::from(token.into()));
        self.clear_api_token = false;
    }

    /// Explicitly clears the stored token on update.
    ///
    /// 3x-ui only permits an enabled tokenless node when mTLS is active.
    pub fn clear_stored_api_token(&mut self) {
        self.api_token = None;
        self.clear_api_token = true;
    }

    /// Omits both credential fields so an update retains the stored token.
    pub fn retain_stored_api_token(&mut self) {
        self.api_token = None;
        self.clear_api_token = false;
    }

    /// Exposes the replacement token when explicitly needed by the caller.
    pub fn api_token(&self) -> Option<&str> {
        self.api_token.as_ref().map(ExposeSecret::expose_secret)
    }

    /// Returns whether this update explicitly clears the stored token.
    pub const fn clears_stored_api_token(&self) -> bool {
        self.clear_api_token
    }
}

impl fmt::Debug for NodeRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NodeRequest")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("remark", &self.remark)
            .field("scheme", &self.scheme)
            .field("address", &self.address)
            .field("port", &self.port)
            .field("base_path", &"[REDACTED]")
            .field("api_token", &self.api_token.as_ref().map(|_| "[REDACTED]"))
            .field("clear_api_token", &self.clear_api_token)
            .field("enable", &self.enable)
            .field("allow_private_address", &self.allow_private_address)
            .field("tls_verify_mode", &self.tls_verify_mode)
            .field("pinned_cert_sha256", &self.pinned_cert_sha256)
            .field("inbound_sync_mode", &self.inbound_sync_mode)
            .field("inbound_tags", &self.inbound_tags)
            .field("outbound_tag", &"[REDACTED]")
            .finish()
    }
}

/// Result of probing saved or unsaved node connection details.
///
/// An unreachable node is represented by `status == Offline` and `error`, not
/// by an API-envelope failure.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeProbeResult {
    /// Reachability result.
    pub status: NodeStatus,
    /// Round-trip latency in milliseconds.
    pub latency_ms: i64,
    /// Remote Xray version.
    pub xray_version: String,
    /// Remote panel version.
    pub panel_version: String,
    /// Remote CPU utilization percentage.
    pub cpu_pct: f64,
    /// Remote memory utilization percentage.
    pub mem_pct: f64,
    /// Remote host uptime in seconds.
    pub uptime_secs: u64,
    /// Friendly connectivity diagnostic when offline.
    pub error: String,
    /// Remote Xray process state when the panel itself was reachable.
    pub xray_state: String,
    /// Remote Xray process error.
    pub xray_error: String,
}

/// One inbound available for selective import from a remote node.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteInboundOption {
    /// Stable remote Xray tag.
    pub tag: String,
    /// Human-readable remote label.
    pub remark: String,
    /// Remote inbound protocol.
    pub protocol: RemoteInboundProtocol,
    /// Remote listen port.
    pub port: u16,
}

/// Outcome of triggering the official panel updater on one node.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeUpdateResult {
    /// Requested node ID.
    pub id: i64,
    /// Configured node name when it was found.
    pub name: String,
    /// Whether the update job was accepted.
    pub ok: bool,
    /// Per-node skip or launch diagnostic.
    pub error: String,
}

/// Target release channel for remote panel self-updates.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeUpdateChannel {
    /// Latest stable release.
    #[default]
    Stable,
    /// Rolling per-commit development build.
    Development,
}

impl NodeUpdateChannel {
    pub(crate) const fn is_development(self) -> bool {
        matches!(self, Self::Development)
    }
}

/// Metric accepted by the per-node history endpoint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum NodeMetric {
    /// CPU utilization percentage.
    Cpu,
    /// Memory utilization percentage.
    Memory,
    /// Interface upload throughput in bytes per second.
    NetworkUp,
    /// Interface download throughput in bytes per second.
    NetworkDown,
}

impl NodeMetric {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "mem",
            Self::NetworkUp => "netUp",
            Self::NetworkDown => "netDown",
        }
    }
}

/// Public node-auth CA certificate generated by the managing panel.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeMtlsCa {
    /// PEM-encoded public CA certificate.
    pub ca_cert: String,
}
