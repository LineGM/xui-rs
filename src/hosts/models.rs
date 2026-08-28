#![allow(clippy::struct_excessive_bools)]

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// TLS/security override applied to generated host links.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HostSecurity {
    /// Inherit security from the inbound.
    #[default]
    Same,
    /// Force TLS.
    Tls,
    /// Disable transport security.
    None,
    /// Force REALITY.
    Reality,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl HostSecurity {
    /// Returns the exact 3x-ui wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Same => "same",
            Self::Tls => "tls",
            Self::None => "none",
            Self::Reality => "reality",
            Self::Other(value) => value,
        }
    }
}

impl Serialize for HostSecurity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HostSecurity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "" | "same" => Self::Same,
            "tls" => Self::Tls,
            "none" => Self::None,
            "reality" => Self::Reality,
            _ => Self::Other(value),
        })
    }
}

/// IP-family preference emitted for Mihomo subscriptions.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MihomoIpVersion {
    /// No override.
    #[default]
    Inherit,
    /// Dual stack without preference.
    Dual,
    /// IPv4 only.
    Ipv4,
    /// IPv6 only.
    Ipv6,
    /// Dual stack preferring IPv4.
    Ipv4Prefer,
    /// Dual stack preferring IPv6.
    Ipv6Prefer,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl MihomoIpVersion {
    /// Returns the exact 3x-ui wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inherit => "",
            Self::Dual => "dual",
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Ipv4Prefer => "ipv4-prefer",
            Self::Ipv6Prefer => "ipv6-prefer",
            Self::Other(value) => value,
        }
    }
}

impl Serialize for MihomoIpVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MihomoIpVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "" => Self::Inherit,
            "dual" => Self::Dual,
            "ipv4" => Self::Ipv4,
            "ipv6" => Self::Ipv6,
            "ipv4-prefer" => Self::Ipv4Prefer,
            "ipv6-prefer" => Self::Ipv6Prefer,
            _ => Self::Other(value),
        })
    }
}

/// Subscription representation from which a host should be excluded.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SubscriptionFormat {
    /// Raw share-link subscription.
    Raw,
    /// JSON subscription.
    Json,
    /// Clash/Mihomo subscription.
    Clash,
    /// Value introduced by a newer panel release.
    Other(String),
}

impl SubscriptionFormat {
    /// Returns the exact 3x-ui wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Raw => "raw",
            Self::Json => "json",
            Self::Clash => "clash",
            Self::Other(value) => value,
        }
    }
}

impl Serialize for SubscriptionFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SubscriptionFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "raw" => Self::Raw,
            "json" => Self::Json,
            "clash" => Self::Clash,
            _ => Self::Other(value),
        })
    }
}

/// Optional VLESS route port embedded into generated subscription identities.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct VlessRoute(Option<u16>);

impl VlessRoute {
    /// Creates a configured route port.
    pub const fn new(port: u16) -> Self {
        Self(Some(port))
    }

    /// Returns the configured port, or `None` when the override is disabled.
    pub const fn port(self) -> Option<u16> {
        self.0
    }
}

impl From<u16> for VlessRoute {
    fn from(port: u16) -> Self {
        Self::new(port)
    }
}

impl Serialize for VlessRoute {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Some(port) => serializer.serialize_str(&port.to_string()),
            None => serializer.serialize_str(""),
        }
    }
}

impl<'de> Deserialize<'de> for VlessRoute {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = VlessRoute;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an empty string or a port from 0 through 65535")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(VlessRoute::default())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(VlessRoute::default())
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                u16::try_from(value)
                    .map(VlessRoute::new)
                    .map_err(|_| E::custom("VLESS route is outside the port range"))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                u16::try_from(value)
                    .map(VlessRoute::new)
                    .map_err(|_| E::custom("VLESS route is outside the port range"))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.is_empty() {
                    return Ok(VlessRoute::default());
                }
                value.parse::<u16>().map(VlessRoute::new).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

/// JSON override stored by 3x-ui as a string field.
///
/// The SDK preserves the raw value for forward compatibility while offering
/// structured construction and parsing, so callers never have to stringify
/// JSON manually.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct HostJsonOverride(String);

impl HostJsonOverride {
    /// Serializes a structured JSON override.
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON value cannot be encoded.
    pub fn from_value(value: &Value) -> serde_json::Result<Self> {
        serde_json::to_string(value).map(Self)
    }

    /// Creates an empty override that inherits the panel/inbound setting.
    pub const fn empty() -> Self {
        Self(String::new())
    }

    /// Borrows the exact string stored by 3x-ui.
    pub fn as_raw(&self) -> &str {
        &self.0
    }

    /// Returns whether the override is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Parses the configured JSON, returning `None` for an empty override.
    ///
    /// # Errors
    ///
    /// Returns an error when a non-empty legacy value is not valid JSON.
    pub fn value(&self) -> serde_json::Result<Option<Value>> {
        if self.is_empty() {
            Ok(None)
        } else {
            serde_json::from_str(&self.0).map(Some)
        }
    }
}

impl fmt::Debug for HostJsonOverride {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("HostJsonOverride")
            .field(&if self.is_empty() {
                "[EMPTY]"
            } else {
                "[CONFIGURED]"
            })
            .finish()
    }
}

impl Serialize for HostJsonOverride {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for HostJsonOverride {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer).map(|value| Self(value.unwrap_or_default()))
    }
}

/// Writable override fields shared by grouped and database-row host views.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HostOptions {
    /// Group order among generated host links.
    pub sort_order: i64,
    /// User-visible name.
    pub remark: String,
    /// Optional short server description.
    pub server_description: String,
    /// Whether the host is disabled.
    pub is_disabled: bool,
    /// Whether the host is hidden from normal subscription output.
    pub is_hidden: bool,
    /// Operator-defined uppercase tags.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub tags: Vec<String>,
    /// Default port used when a host string does not include one.
    pub port: u16,
    /// TLS/security override.
    pub security: HostSecurity,
    /// TLS/REALITY server name override.
    pub sni: String,
    /// HTTP Host header override.
    pub host_header: String,
    /// Transport path override.
    pub path: String,
    /// ALPN override list.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub alpn: Vec<String>,
    /// uTLS fingerprint identifier.
    pub fingerprint: String,
    /// Derives SNI from each configured host address.
    pub override_sni_from_address: bool,
    /// Keeps SNI blank instead of inheriting it.
    pub keep_sni_blank: bool,
    /// Accepted peer certificate SHA-256 pins.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub pinned_peer_cert_sha256: Vec<String>,
    /// Peer certificate DNS name to verify.
    pub verify_peer_cert_by_name: String,
    /// Disables peer certificate verification.
    pub allow_insecure: bool,
    /// ECH configuration list.
    pub ech_config_list: String,
    /// Xray mux override stored as nested JSON.
    pub mux_params: HostJsonOverride,
    /// Xray socket options stored as nested JSON.
    pub sockopt_params: HostJsonOverride,
    /// Xray final-mask override stored as nested JSON.
    pub final_mask: HostJsonOverride,
    /// Optional VLESS route port.
    pub vless_route: VlessRoute,
    /// Subscription formats that should omit this host.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub exclude_from_sub_types: Vec<SubscriptionFormat>,
    /// Node GUID allowlist for this override.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub node_guids: Vec<String>,
    /// Mihomo IP-family preference.
    #[serde(rename = "mihomoIpVersion")]
    pub mihomo_ip_version: MihomoIpVersion,
    /// Enables Mihomo X25519 output.
    pub mihomo_x25519: bool,
    /// Randomizes host selection in generated output.
    pub shuffle_host: bool,
}

/// One logical host override shared across one or more inbounds and addresses.
///
/// The same type is accepted for create and full-replacement update operations.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HostGroup {
    /// Stable logical group identifier. Leave blank on create to generate one.
    pub group_id: String,
    /// Inbounds to which this group applies.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub inbound_ids: Vec<i64>,
    /// Hostnames/IPs, optionally including per-address ports.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub hosts: Vec<String>,
    /// Shared host override options.
    #[serde(flatten)]
    pub options: HostOptions,
}

impl HostGroup {
    /// Creates an enabled group that inherits inbound security.
    pub fn new(inbound_ids: Vec<i64>, remark: impl Into<String>) -> Self {
        Self {
            inbound_ids,
            options: HostOptions {
                remark: remark.into(),
                ..HostOptions::default()
            },
            ..Self::default()
        }
    }
}

/// One physical database row created for a group/address/inbound combination.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HostRow {
    /// Database identifier.
    pub id: i64,
    /// Logical group identifier.
    pub group_id: String,
    /// Parent inbound identifier.
    pub inbound_id: i64,
    /// Hostname or IP without its port.
    pub address: String,
    /// Shared host override options.
    #[serde(flatten)]
    pub options: HostOptions,
    /// Unix creation timestamp in milliseconds.
    pub created_at: i64,
    /// Unix update timestamp in milliseconds.
    pub updated_at: i64,
}
