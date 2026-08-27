use std::{collections::HashMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ClientTraffic, InboundProtocol};

pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// VLESS reverse-proxy configuration attached to a client.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClientReverse {
    /// Xray reverse tag.
    pub tag: String,
}

/// Complete writable client configuration accepted by 3x-ui.
///
/// Protocol-specific credentials may be left empty on create; 3x-ui generates
/// the appropriate UUID, password, auth value, or `MTProto` secret from the
/// target inbound. Use [`ClientRecord::to_config`] before replacement updates.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientConfig {
    /// VMess/VLESS UUID.
    #[serde(rename = "id", skip_serializing_if = "String::is_empty")]
    pub protocol_id: String,
    /// Protocol security method.
    pub security: String,
    /// Trojan or Shadowsocks password/key.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    /// XTLS flow value.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub flow: String,
    /// VLESS reverse-proxy settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<ClientReverse>,
    /// Hysteria authentication value.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub auth: String,
    /// `WireGuard` private key.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub private_key: String,
    /// `WireGuard` public key.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub public_key: String,
    /// `WireGuard` allowed networks.
    #[serde(
        rename = "allowedIPs",
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_default"
    )]
    pub allowed_ips: Vec<String>,
    /// `WireGuard` pre-shared key.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pre_shared_key: String,
    /// `WireGuard` keepalive interval.
    #[serde(skip_serializing_if = "is_zero_i32")]
    pub keep_alive: i32,
    /// `MTProto` client secret.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub secret: String,
    /// `MTProto` advertisement tag.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ad_tag: String,
    /// Unique client email/name.
    pub email: String,
    /// Maximum concurrent source IP count.
    pub limit_ip: i32,
    /// Traffic quota in bytes, despite the historical `totalGB` wire name.
    #[serde(rename = "totalGB")]
    pub total_gb: i64,
    /// Expiration timestamp in milliseconds; zero means unlimited.
    pub expiry_time: i64,
    /// Whether the client is enabled.
    pub enable: bool,
    /// Telegram user identifier for notifications.
    pub tg_id: i64,
    /// Subscription identifier.
    pub sub_id: String,
    /// Optional logical group label.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    /// Operator comment.
    pub comment: String,
    /// Automatic reset period encoded by 3x-ui.
    pub reset: i32,
    /// Creation timestamp in milliseconds used by import/export.
    #[serde(rename = "created_at", skip_serializing_if = "is_zero_i64")]
    pub created_at: i64,
    /// Update timestamp in milliseconds used by import/export.
    #[serde(rename = "updated_at", skip_serializing_if = "is_zero_i64")]
    pub updated_at: i64,
}

impl ClientConfig {
    /// Creates an enabled, unlimited client and lets 3x-ui generate credentials.
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            security: "auto".to_owned(),
            enable: true,
            ..Self::default()
        }
    }
}

impl fmt::Debug for ClientConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientConfig")
            .field("protocol_id", &"[REDACTED]")
            .field("security", &self.security)
            .field("password", &"[REDACTED]")
            .field("flow", &self.flow)
            .field("reverse", &self.reverse)
            .field("auth", &"[REDACTED]")
            .field("private_key", &"[REDACTED]")
            .field("public_key", &"[REDACTED]")
            .field("allowed_ips", &self.allowed_ips)
            .field("pre_shared_key", &"[REDACTED]")
            .field("keep_alive", &self.keep_alive)
            .field("secret", &"[REDACTED]")
            .field("ad_tag", &"[REDACTED]")
            .field("email", &self.email)
            .field("limit_ip", &self.limit_ip)
            .field("total_gb", &self.total_gb)
            .field("expiry_time", &self.expiry_time)
            .field("enable", &self.enable)
            .field("tg_id", &self.tg_id)
            .field("sub_id", &"[REDACTED]")
            .field("group", &self.group)
            .field("comment", &self.comment)
            .field("reset", &self.reset)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Canonical database-backed client record returned by list/get endpoints.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientRecord {
    /// Database identifier.
    pub id: i64,
    /// Unique client email/name.
    pub email: String,
    /// Subscription identifier.
    pub sub_id: String,
    /// VMess/VLESS UUID.
    pub uuid: String,
    /// Trojan or Shadowsocks password/key.
    pub password: String,
    /// Hysteria authentication value.
    pub auth: String,
    /// Effective XTLS flow.
    pub flow: String,
    /// Protocol security method.
    pub security: String,
    /// VLESS reverse-proxy settings, serialized as a nested object by v3.6.0.
    pub reverse: Option<ClientReverse>,
    /// `WireGuard` private key.
    pub private_key: String,
    /// `WireGuard` public key.
    pub public_key: String,
    /// Comma-separated `WireGuard` allowed networks.
    #[serde(rename = "allowedIPs")]
    pub allowed_ips: String,
    /// `WireGuard` pre-shared key.
    pub pre_shared_key: String,
    /// `WireGuard` keepalive interval.
    pub keep_alive: i32,
    /// `MTProto` client secret.
    pub secret: String,
    /// `MTProto` advertisement tag.
    pub ad_tag: String,
    /// Maximum concurrent source IP count.
    pub limit_ip: i32,
    /// Traffic quota in bytes.
    #[serde(rename = "totalGB")]
    pub total_gb: i64,
    /// Expiration timestamp in milliseconds.
    pub expiry_time: i64,
    /// Whether the client is enabled.
    pub enable: bool,
    /// Telegram user identifier.
    pub tg_id: i64,
    /// Logical group label.
    pub group: String,
    /// Operator comment.
    pub comment: String,
    /// Automatic reset period encoded by 3x-ui.
    pub reset: i32,
    /// Creation timestamp in milliseconds.
    pub created_at: i64,
    /// Last update timestamp in milliseconds.
    pub updated_at: i64,
}

impl ClientRecord {
    /// Converts the response into the full writable replacement payload.
    pub fn to_config(&self) -> ClientConfig {
        ClientConfig {
            protocol_id: self.uuid.clone(),
            security: self.security.clone(),
            password: self.password.clone(),
            flow: self.flow.clone(),
            reverse: self.reverse.clone(),
            auth: self.auth.clone(),
            private_key: self.private_key.clone(),
            public_key: self.public_key.clone(),
            allowed_ips: self
                .allowed_ips
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            pre_shared_key: self.pre_shared_key.clone(),
            keep_alive: self.keep_alive,
            secret: self.secret.clone(),
            ad_tag: self.ad_tag.clone(),
            email: self.email.clone(),
            limit_ip: self.limit_ip,
            total_gb: self.total_gb,
            expiry_time: self.expiry_time,
            enable: self.enable,
            tg_id: self.tg_id,
            sub_id: self.sub_id.clone(),
            group: self.group.clone(),
            comment: self.comment.clone(),
            reset: self.reset,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl fmt::Debug for ClientRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientRecord")
            .field("id", &self.id)
            .field("email", &self.email)
            .field("sub_id", &"[REDACTED]")
            .field("uuid", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .field("auth", &"[REDACTED]")
            .field("flow", &self.flow)
            .field("security", &self.security)
            .field("reverse", &self.reverse)
            .field("private_key", &"[REDACTED]")
            .field("public_key", &"[REDACTED]")
            .field("allowed_ips", &self.allowed_ips)
            .field("pre_shared_key", &"[REDACTED]")
            .field("keep_alive", &self.keep_alive)
            .field("secret", &"[REDACTED]")
            .field("ad_tag", &"[REDACTED]")
            .field("limit_ip", &self.limit_ip)
            .field("total_gb", &self.total_gb)
            .field("expiry_time", &self.expiry_time)
            .field("enable", &self.enable)
            .field("tg_id", &self.tg_id)
            .field("group", &self.group)
            .field("comment", &self.comment)
            .field("reset", &self.reset)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Client record enriched with attachments and traffic state.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientWithAttachments {
    /// Canonical client record.
    #[serde(flatten)]
    pub client: ClientRecord,
    /// Attached inbound identifiers.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub inbound_ids: Vec<i64>,
    /// Shared traffic record, when one exists.
    pub traffic: Option<ClientTraffic>,
}

/// Full payload returned by the single-client lookup endpoints.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetails {
    /// Canonical client record.
    pub client: ClientRecord,
    /// Attached inbound identifiers.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub inbound_ids: Vec<i64>,
    /// Persisted external links and subscriptions.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub external_links: Vec<ClientExternalLink>,
    /// Sum of uploaded and downloaded bytes.
    pub used_traffic: i64,
}

/// Lightweight client row returned by paginated listing.
#[derive(Clone, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientSlim {
    /// Unique client email/name.
    pub email: String,
    /// Subscription identifier.
    pub sub_id: String,
    /// Whether the client is enabled.
    pub enable: bool,
    /// Traffic quota in bytes.
    #[serde(rename = "totalGB")]
    pub total_gb: i64,
    /// Expiration timestamp in milliseconds.
    pub expiry_time: i64,
    /// Maximum concurrent source IP count.
    pub limit_ip: i32,
    /// Automatic reset period encoded by 3x-ui.
    pub reset: i32,
    /// Logical group label.
    pub group: String,
    /// Operator comment.
    pub comment: String,
    /// Attached inbound identifiers.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub inbound_ids: Vec<i64>,
    /// Shared traffic state, when present.
    pub traffic: Option<ClientTraffic>,
    /// Creation timestamp in milliseconds.
    pub created_at: i64,
    /// Last update timestamp in milliseconds.
    pub updated_at: i64,
}

impl fmt::Debug for ClientSlim {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSlim")
            .field("email", &self.email)
            .field("sub_id", &"[REDACTED]")
            .field("enable", &self.enable)
            .field("total_gb", &self.total_gb)
            .field("expiry_time", &self.expiry_time)
            .field("limit_ip", &self.limit_ip)
            .field("reset", &self.reset)
            .field("group", &self.group)
            .field("comment", &self.comment)
            .field("inbound_ids", &self.inbound_ids)
            .field("traffic", &self.traffic)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Aggregate counts and capped email buckets returned with a client page.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientSummary {
    /// Total client count.
    pub total: u64,
    /// Enabled, non-depleted, non-expiring client count.
    pub active: u64,
    /// Online client count.
    pub online_count: u64,
    /// Depleted client count.
    pub depleted_count: u64,
    /// Expiring client count.
    pub expiring_count: u64,
    /// Disabled, non-depleted client count.
    pub deactive_count: u64,
    /// Capped list of online emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub online: Vec<String>,
    /// Capped list of depleted emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub depleted: Vec<String>,
    /// Capped list of expiring emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub expiring: Vec<String>,
    /// Capped list of disabled emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub deactive: Vec<String>,
}

/// One page of clients and the global dashboard summary.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientPage {
    /// Page rows.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub items: Vec<ClientSlim>,
    /// Unfiltered database row count.
    pub total: u64,
    /// Count after filters but before pagination.
    pub filtered: u64,
    /// One-indexed page number selected by the server.
    pub page: u32,
    /// Effective page size, capped by 3x-ui at 200.
    pub page_size: u16,
    /// Global dashboard summary.
    pub summary: ClientSummary,
    /// Available group names.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub groups: Vec<String>,
}

/// Status bucket used by paginated client filtering.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ClientStatusFilter {
    /// Enabled and not depleted.
    Active,
    /// Disabled.
    Deactive,
    /// Traffic- or time-depleted.
    Depleted,
    /// Near expiry or quota according to panel thresholds.
    Expiring,
    /// Currently connected.
    Online,
}

impl ClientStatusFilter {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deactive => "deactive",
            Self::Depleted => "depleted",
            Self::Expiring => "expiring",
            Self::Online => "online",
        }
    }
}

/// Sort key used by paginated client listing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ClientSort {
    /// Enabled state.
    Enable,
    /// Email/name.
    Email,
    /// Number of attached inbounds.
    InboundCount,
    /// Consumed traffic.
    Traffic,
    /// Remaining quota.
    Remaining,
    /// Expiration timestamp.
    ExpiryTime,
    /// Creation timestamp.
    CreatedAt,
    /// Update timestamp.
    UpdatedAt,
    /// Last online timestamp.
    LastOnline,
}

impl ClientSort {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Email => "email",
            Self::InboundCount => "inboundIds",
            Self::Traffic => "traffic",
            Self::Remaining => "remaining",
            Self::ExpiryTime => "expiryTime",
            Self::CreatedAt => "createdAt",
            Self::UpdatedAt => "updatedAt",
            Self::LastOnline => "lastOnline",
        }
    }
}

/// Sort direction used by paginated client listing.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SortOrder {
    /// Ascending order.
    #[default]
    Ascending,
    /// Descending order.
    Descending,
}

impl SortOrder {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascend",
            Self::Descending => "descend",
        }
    }
}

/// Filters, ranges, and ordering accepted by the paginated listing endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientPageRequest {
    /// One-indexed page number.
    pub page: u32,
    /// Requested rows per page; the server caps this at 200.
    pub page_size: u16,
    /// Case-insensitive search across identity and comment fields.
    pub search: String,
    /// OR-combined status buckets.
    pub statuses: Vec<ClientStatusFilter>,
    /// OR-combined attached inbound protocols.
    pub protocols: Vec<InboundProtocol>,
    /// OR-combined attached inbound identifiers.
    pub inbound_ids: Vec<i64>,
    /// Optional sort key.
    pub sort: Option<ClientSort>,
    /// Sort direction.
    pub order: SortOrder,
    /// Inclusive minimum expiry timestamp in milliseconds.
    pub expiry_from: Option<i64>,
    /// Inclusive maximum expiry timestamp in milliseconds.
    pub expiry_to: Option<i64>,
    /// Inclusive minimum used bytes.
    pub usage_from: Option<i64>,
    /// Inclusive maximum used bytes.
    pub usage_to: Option<i64>,
    /// Filters by whether automatic renewal is configured.
    pub auto_renew: Option<bool>,
    /// Filters by whether a Telegram ID is present.
    pub has_telegram_id: Option<bool>,
    /// Filters by whether a comment is present.
    pub has_comment: Option<bool>,
    /// OR-combined, case-insensitive group names.
    pub groups: Vec<String>,
}

impl Default for ClientPageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 25,
            search: String::new(),
            statuses: Vec::new(),
            protocols: Vec::new(),
            inbound_ids: Vec::new(),
            sort: None,
            order: SortOrder::Ascending,
            expiry_from: None,
            expiry_to: None,
            usage_from: None,
            usage_to: None,
            auto_renew: None,
            has_telegram_id: None,
            has_comment: None,
            groups: Vec::new(),
        }
    }
}

/// Request accepted by create, bulk-create, export, and import.
#[derive(Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCreateRequest {
    /// Client configuration.
    pub client: ClientConfig,
    /// Inbounds to attach the client to.
    pub inbound_ids: Vec<i64>,
}

impl ClientCreateRequest {
    /// Creates a payload from a client and target inbound identifiers.
    pub fn new(client: ClientConfig, inbound_ids: Vec<i64>) -> Self {
        Self {
            client,
            inbound_ids,
        }
    }
}

impl fmt::Debug for ClientCreateRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientCreateRequest")
            .field("client", &self.client)
            .field("inbound_ids", &self.inbound_ids)
            .finish()
    }
}

impl<'de> Deserialize<'de> for ClientCreateRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            client: ClientConfig,
            #[serde(default, deserialize_with = "deserialize_null_default")]
            inbound_ids: Vec<i64>,
        }

        Wire::deserialize(deserializer).map(|wire| Self {
            client: wire.client,
            inbound_ids: wire.inbound_ids,
        })
    }
}

/// Additional state returned after a mutation involving remote nodes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientMutationStatus {
    /// The change is committed locally and awaits an offline node.
    pub node_pending: bool,
}

/// Kind of per-client external subscription entry.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ClientExternalLinkKind {
    /// One share link.
    #[default]
    Link,
    /// Remote HTTP(S) subscription.
    Subscription,
}

/// Writable external link row.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientExternalLinkInput {
    /// Link or subscription kind.
    pub kind: ClientExternalLinkKind,
    /// Share link or subscription URL.
    pub value: String,
    /// Optional display label.
    pub remark: String,
}

impl fmt::Debug for ClientExternalLinkInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientExternalLinkInput")
            .field("kind", &self.kind)
            .field("value", &"[REDACTED]")
            .field("remark", &self.remark)
            .finish()
    }
}

/// Persisted external link row returned with client details.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientExternalLink {
    /// Database identifier.
    pub id: i64,
    /// Parent client record identifier.
    pub client_id: i64,
    /// Link or subscription kind.
    pub kind: ClientExternalLinkKind,
    /// Share link or subscription URL.
    pub value: String,
    /// Optional display label.
    pub remark: String,
    /// Stable display order.
    pub sort_index: i32,
    /// Creation timestamp in milliseconds.
    pub created_at: i64,
}

impl fmt::Debug for ClientExternalLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientExternalLink")
            .field("id", &self.id)
            .field("client_id", &self.client_id)
            .field("kind", &self.kind)
            .field("value", &"[REDACTED]")
            .field("remark", &self.remark)
            .field("sort_index", &self.sort_index)
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Typed directive for the optional flow part of a bulk adjustment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BulkFlowAdjustment {
    /// Leave flow unchanged.
    #[default]
    Unchanged,
    /// Clear flow.
    Clear,
    /// Set `xtls-rprx-vision` where supported.
    Vision,
    /// Set `xtls-rprx-vision-udp443` where supported.
    VisionUdp443,
}

impl BulkFlowAdjustment {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "",
            Self::Clear => "none",
            Self::Vision => "xtls-rprx-vision",
            Self::VisionUdp443 => "xtls-rprx-vision-udp443",
        }
    }
}

/// Bulk expiry/quota/flow adjustment request.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BulkAdjustRequest {
    /// Target emails.
    pub emails: Vec<String>,
    /// Signed number of days added to finite expiry times.
    pub add_days: i32,
    /// Signed bytes added to finite quotas.
    pub add_bytes: i64,
    /// Optional typed flow directive.
    pub flow: BulkFlowAdjustment,
}

/// Per-client failure or skip report used by bulk operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BulkClientIssue {
    /// Client email/name.
    pub email: String,
    /// Server-provided reason.
    pub reason: String,
}

/// Result of bulk expiry/quota/flow adjustment.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct BulkAdjustResult {
    /// Number of adjusted clients.
    pub adjusted: u64,
    /// Per-client skips.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<BulkClientIssue>,
}

/// Result shared by bulk create and import.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct BulkCreateResult {
    /// Number of created clients.
    pub created: u64,
    /// Per-client skips.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<BulkClientIssue>,
}

/// Result of bulk deletion.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct BulkDeleteResult {
    /// Number of deleted clients.
    pub deleted: u64,
    /// Per-client skips.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<BulkClientIssue>,
}

/// Result of bulk enable/disable.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct BulkSetEnabledResult {
    /// Number of clients whose state changed.
    pub changed: u64,
    /// Per-client skips.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<BulkClientIssue>,
}

/// Result of attaching multiple clients to multiple inbounds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct BulkAttachResult {
    /// Successfully attached emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub attached: Vec<String>,
    /// Emails that were already in the desired state.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<String>,
    /// Human-readable per-target errors.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub errors: Vec<String>,
}

/// Result of detaching multiple clients from multiple inbounds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct BulkDetachResult {
    /// Successfully detached emails.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub detached: Vec<String>,
    /// Emails that were not attached to the requested inbounds.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub skipped: Vec<String>,
    /// Human-readable per-target errors.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub errors: Vec<String>,
}

/// A simple deleted-row count.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct DeletedCount {
    /// Number of deleted rows.
    pub deleted: u64,
}

/// A simple affected-row count.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct AffectedCount {
    /// Number of affected rows.
    pub affected: u64,
}

/// Name echoed after group creation or traffic reset.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct GroupName {
    /// Group name.
    pub name: String,
}

/// Group membership and traffic totals since the last group reset.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GroupSummary {
    /// Group name.
    pub name: String,
    /// Number of clients in the group.
    pub client_count: u64,
    /// Uploaded plus downloaded bytes since group reset.
    pub traffic_used: i64,
    /// Uploaded bytes since group reset.
    pub up: i64,
    /// Downloaded bytes since group reset.
    pub down: i64,
}

/// One observed source IP and its last-seen Unix timestamp in seconds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ClientIpEntry {
    /// Source IP address.
    pub ip: String,
    /// Last-seen Unix timestamp in seconds.
    pub timestamp: i64,
}

/// Display-ready client IP annotated with its node.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ClientIpInfo {
    /// Source IP address.
    pub ip: String,
    /// Panel-formatted local timestamp.
    pub time: String,
    /// Node display name, or empty for the local panel.
    pub node: String,
}

/// Online emails grouped by stable panel GUID.
pub type ClientsByGuid = HashMap<String, Vec<String>>;

/// Active inbound tags grouped by stable panel GUID.
pub type ActiveInboundsByGuid = HashMap<String, Vec<String>>;

/// Per-client IP observations grouped by panel GUID and email.
pub type ClientIpsByGuid = HashMap<String, HashMap<String, Vec<ClientIpEntry>>>;

/// Last-online timestamps keyed by client email.
pub type LastOnlineByEmail = HashMap<String, i64>;

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_conversion_preserves_exact_acronym_wire_fields() {
        let record: ClientRecord = serde_json::from_value(serde_json::json!({
            "id": 9,
            "email": "alice",
            "uuid": "uuid-secret",
            "subId": "sub-secret",
            "allowedIPs": "10.0.0.2/32, fd00::2/128",
            "totalGB": 4096,
            "enable": true,
            "reverse": {"tag": "reverse-tag"}
        }))
        .unwrap();
        let config = record.to_config();
        let wire = serde_json::to_value(&config).unwrap();

        assert_eq!(config.allowed_ips, ["10.0.0.2/32", "fd00::2/128"]);
        assert_eq!(
            wire["allowedIPs"],
            serde_json::json!(["10.0.0.2/32", "fd00::2/128"])
        );
        assert_eq!(wire["totalGB"], 4096);
        assert!(wire.get("allowedIps").is_none());
        assert!(wire.get("totalGb").is_none());
    }

    #[test]
    fn client_debug_redacts_protocol_and_subscription_secrets() {
        let mut client = ClientConfig::new("alice");
        client.protocol_id = "uuid-secret".to_owned();
        client.password = "password-secret".to_owned();
        client.auth = "auth-secret".to_owned();
        client.private_key = "private-secret".to_owned();
        client.pre_shared_key = "psk-secret".to_owned();
        client.secret = "mtproto-secret".to_owned();
        client.sub_id = "subscription-secret".to_owned();
        let output = format!("{client:?}");

        for secret in [
            "uuid-secret",
            "password-secret",
            "auth-secret",
            "private-secret",
            "psk-secret",
            "mtproto-secret",
            "subscription-secret",
        ] {
            assert!(!output.contains(secret));
        }
        assert!(output.contains("alice"));

        let slim = ClientSlim {
            email: "alice".to_owned(),
            sub_id: "slim-subscription-secret".to_owned(),
            ..ClientSlim::default()
        };
        assert!(!format!("{slim:?}").contains("slim-subscription-secret"));
    }

    #[test]
    fn nil_go_slices_decode_as_empty_rust_vectors() {
        let result: BulkAttachResult = serde_json::from_value(serde_json::json!({
            "attached": null,
            "skipped": null,
            "errors": null
        }))
        .unwrap();
        let client: ClientWithAttachments = serde_json::from_value(serde_json::json!({
            "email": "orphan",
            "inboundIds": null
        }))
        .unwrap();

        assert!(result.attached.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());
        assert!(client.inbound_ids.is_empty());
    }
}
