use std::fmt;

use reqwest::Method;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{Client, Error, Result, client::AuthenticationScope, response::ApiResponse};

const ROOT: &str = "panel/api/inbounds";

/// Xray protocols accepted by 3x-ui v3.6.0 inbound endpoints.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum InboundProtocol {
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
    /// Hysteria (including Hysteria 2 through stream settings).
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
}

/// Schedule used to reset an inbound's traffic counters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TrafficReset {
    /// Never reset counters automatically.
    #[default]
    Never,
    /// Reset every hour.
    Hourly,
    /// Reset every day.
    Daily,
    /// Reset every week.
    Weekly,
    /// Reset every month.
    Monthly,
}

/// Strategy used to choose the public address in generated share links.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ShareAddressStrategy {
    /// Prefer the hosting node's public address.
    #[default]
    Node,
    /// Use the inbound listen address.
    Listen,
    /// Use the explicitly configured share address.
    Custom,
}

/// Per-client traffic state returned with an inbound.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTraffic {
    /// Database row identifier.
    pub id: i64,
    /// Parent inbound identifier.
    pub inbound_id: i64,
    /// Whether this client is enabled.
    pub enable: bool,
    /// Globally unique client email/name.
    pub email: String,
    /// Protocol client identifier, when applicable.
    #[serde(default)]
    pub uuid: String,
    /// Subscription identifier.
    #[serde(default)]
    pub sub_id: String,
    /// Uploaded bytes.
    pub up: i64,
    /// Downloaded bytes.
    pub down: i64,
    /// Expiration timestamp in milliseconds, or zero when unlimited.
    pub expiry_time: i64,
    /// Traffic limit in bytes, or zero when unlimited.
    pub total: i64,
    /// Client traffic reset policy encoded by the panel.
    pub reset: i32,
    /// Most recent online timestamp in milliseconds.
    pub last_online: i64,
}

impl fmt::Debug for ClientTraffic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientTraffic")
            .field("id", &self.id)
            .field("inbound_id", &self.inbound_id)
            .field("enable", &self.enable)
            .field("email", &self.email)
            .field("uuid", &"[REDACTED]")
            .field("sub_id", &"[REDACTED]")
            .field("up", &self.up)
            .field("down", &self.down)
            .field("expiry_time", &self.expiry_time)
            .field("total", &self.total)
            .field("reset", &self.reset)
            .field("last_online", &self.last_online)
            .finish()
    }
}

/// Complete writable configuration accepted by create, update, and import.
///
/// Use [`InboundConfig::new`] for safe panel defaults. Convert a fetched
/// [`Inbound`] with [`Inbound::to_config`] before replacing it through
/// [`InboundsApi::update`]. The open-ended JSON fields intentionally mirror
/// Xray's protocol-specific configuration fragments.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundConfig {
    /// Uploaded bytes to seed during import.
    pub up: i64,
    /// Downloaded bytes to seed during import.
    pub down: i64,
    /// Traffic limit in bytes, or zero when unlimited.
    pub total: i64,
    /// Human-readable label.
    pub remark: String,
    /// One-based position in subscription output.
    pub sub_sort_index: i32,
    /// Whether the inbound is enabled.
    pub enable: bool,
    /// Expiration timestamp in milliseconds, or zero when unlimited.
    pub expiry_time: i64,
    /// Automatic traffic reset schedule.
    pub traffic_reset: TrafficReset,
    /// Day of month used by monthly reset schedules.
    pub traffic_reset_day: u8,
    /// Most recent automatic reset timestamp in milliseconds.
    pub last_traffic_reset_time: i64,
    /// Per-client traffic rows preserved by imports.
    #[serde(default)]
    pub client_stats: Vec<ClientTraffic>,
    /// Local listen address, or an empty string for all interfaces.
    pub listen: String,
    /// Listen port; zero lets compatible server flows choose it.
    pub port: u16,
    /// Xray inbound protocol.
    pub protocol: InboundProtocol,
    /// Protocol-specific Xray settings object.
    pub settings: Value,
    /// Transport and security settings object.
    pub stream_settings: Value,
    /// Stable Xray tag; leave empty on create to let 3x-ui generate it.
    pub tag: String,
    /// Xray sniffing settings object.
    pub sniffing: Value,
    /// Hosting node identifier, or `None` for the local panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<i64>,
    /// Public share-address selection strategy.
    pub share_addr_strategy: ShareAddressStrategy,
    /// Explicit share address used with the custom strategy.
    pub share_addr: String,
}

impl InboundConfig {
    /// Creates an enabled configuration with unlimited traffic and safe panel defaults.
    pub fn new(protocol: InboundProtocol, port: u16) -> Self {
        Self {
            up: 0,
            down: 0,
            total: 0,
            remark: String::new(),
            sub_sort_index: 1,
            enable: true,
            expiry_time: 0,
            traffic_reset: TrafficReset::Never,
            traffic_reset_day: 1,
            last_traffic_reset_time: 0,
            client_stats: Vec::new(),
            listen: String::new(),
            port,
            protocol,
            settings: Value::Object(serde_json::Map::new()),
            stream_settings: Value::Object(serde_json::Map::new()),
            tag: String::new(),
            sniffing: Value::Object(serde_json::Map::new()),
            node_id: None,
            share_addr_strategy: ShareAddressStrategy::Node,
            share_addr: String::new(),
        }
    }
}

impl fmt::Debug for InboundConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InboundConfig")
            .field("up", &self.up)
            .field("down", &self.down)
            .field("total", &self.total)
            .field("remark", &self.remark)
            .field("sub_sort_index", &self.sub_sort_index)
            .field("enable", &self.enable)
            .field("expiry_time", &self.expiry_time)
            .field("traffic_reset", &self.traffic_reset)
            .field("traffic_reset_day", &self.traffic_reset_day)
            .field("last_traffic_reset_time", &self.last_traffic_reset_time)
            .field("client_stats", &self.client_stats)
            .field("listen", &self.listen)
            .field("port", &self.port)
            .field("protocol", &self.protocol)
            .field("settings", &"[REDACTED]")
            .field("stream_settings", &"[REDACTED]")
            .field("tag", &self.tag)
            .field("sniffing", &"[REDACTED]")
            .field("node_id", &self.node_id)
            .field("share_addr_strategy", &self.share_addr_strategy)
            .field("share_addr", &self.share_addr)
            .finish()
    }
}

/// Metadata describing a master inbound used by a fallback child.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackParent {
    /// Master inbound identifier.
    pub master_id: i64,
    /// Match path on the master.
    #[serde(default)]
    pub path: String,
}

/// Full inbound record returned by 3x-ui.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inbound {
    /// Inbound identifier.
    pub id: i64,
    /// Writable configuration and current traffic state.
    #[serde(flatten)]
    pub config: InboundConfig,
    /// GUID of the node where the inbound originated.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub origin_node_guid: String,
    /// Master relationship when this inbound is used as a fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_parent: Option<FallbackParent>,
}

impl Inbound {
    /// Clones the complete writable payload for update or import.
    pub fn to_config(&self) -> InboundConfig {
        self.config.clone()
    }

    /// Consumes the response and returns its writable payload.
    pub fn into_config(self) -> InboundConfig {
        self.config
    }
}

impl fmt::Debug for Inbound {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Inbound")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("origin_node_guid", &self.origin_node_guid)
            .field("fallback_parent", &self.fallback_parent)
            .finish()
    }
}

/// Lightweight inbound projection used by picker UIs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundOption {
    /// Inbound identifier.
    pub id: i64,
    /// Human-readable label.
    pub remark: String,
    /// Stable Xray tag.
    pub tag: String,
    /// Xray inbound protocol.
    pub protocol: InboundProtocol,
    /// Listen port.
    pub port: u16,
    /// Whether the inbound is enabled.
    pub enable: bool,
    /// Whether the transport supports VLESS Vision flow.
    pub tls_flow_capable: bool,
    /// Shadowsocks method, when applicable.
    #[serde(default)]
    pub ss_method: String,
    /// `WireGuard` public key, when applicable.
    #[serde(default)]
    pub wg_public_key: String,
    /// `WireGuard` MTU, when applicable.
    #[serde(default)]
    pub wg_mtu: u32,
    /// `WireGuard` DNS value, when applicable.
    #[serde(default)]
    pub wg_dns: String,
    /// `MTProto` domain, when applicable.
    #[serde(default)]
    pub mtproto_domain: String,
    /// Hosting node identifier.
    #[serde(default)]
    pub node_id: Option<i64>,
    /// Hosting node's externally reachable address.
    #[serde(default)]
    pub node_address: String,
    /// Local listen address.
    #[serde(default)]
    pub listen: String,
    /// Explicit public share address.
    #[serde(default)]
    pub share_addr: String,
    /// Share-address strategy, when populated by the panel.
    #[serde(default)]
    pub share_addr_strategy: Option<ShareAddressStrategy>,
}

/// A persisted fallback rule attached to a master inbound.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundFallback {
    /// Fallback row identifier.
    pub id: i64,
    /// Master inbound identifier.
    pub master_id: i64,
    /// Child inbound identifier.
    pub child_id: i64,
    /// Optional SNI/server name match.
    pub name: String,
    /// Optional ALPN match.
    pub alpn: String,
    /// Optional path match.
    pub path: String,
    /// Explicit Xray destination, or empty to derive it from the child.
    pub dest: String,
    /// PROXY protocol version.
    pub xver: i32,
    /// Stable ordering value.
    pub sort_order: i32,
}

/// Input used when atomically replacing a master's fallback rules.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackInput {
    /// Child inbound identifier.
    pub child_id: i64,
    /// Optional SNI/server name match.
    pub name: String,
    /// Optional ALPN match.
    pub alpn: String,
    /// Optional path match.
    pub path: String,
    /// Explicit Xray destination, or empty to derive it from the child.
    pub dest: String,
    /// PROXY protocol version.
    pub xver: i32,
    /// Stable ordering value.
    pub sort_order: i32,
}

/// An inbound that could not be deleted by a bulk operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SkippedInbound {
    /// Inbound identifier.
    pub id: i64,
    /// Server-provided failure reason.
    pub reason: String,
}

/// Result of deleting multiple inbounds.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BulkDeleteInboundsResult {
    /// Number of deleted inbounds.
    pub deleted: u64,
    /// Per-inbound failures; other inbounds were still processed.
    #[serde(default)]
    pub skipped: Vec<SkippedInbound>,
}

/// A client that could not be deleted by a bulk operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SkippedClient {
    /// Client email/name.
    pub email: String,
    /// Server-provided failure reason.
    pub reason: String,
}

/// Result of deleting all clients from one inbound.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BulkDeleteClientsResult {
    /// Number of deleted clients.
    pub deleted: u64,
    /// Per-client failures; other clients were still processed.
    #[serde(default)]
    pub skipped: Vec<SkippedClient>,
}

/// Minimal per-client aggregate consumed by traffic synchronization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTrafficUsage {
    /// Client email/name known to the receiving panel.
    pub email: String,
    /// Aggregated uploaded bytes.
    pub up: i64,
    /// Aggregated downloaded bytes.
    pub down: i64,
}

/// Traffic snapshot pushed from a master panel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPushRequest {
    /// Stable GUID of the sending master panel.
    pub master_guid: String,
    /// Aggregated per-client usage rows.
    pub traffics: Vec<ClientTrafficUsage>,
}

impl TrafficPushRequest {
    /// Creates a traffic snapshot for a master panel.
    pub fn new(master_guid: impl Into<String>, traffics: Vec<ClientTrafficUsage>) -> Self {
        Self {
            master_guid: master_guid.into(),
            traffics,
        }
    }
}

/// Inbound management endpoints for a [`Client`].
#[derive(Clone, Copy, Debug)]
pub struct InboundsApi<'client> {
    client: &'client Client,
}

impl<'client> InboundsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Returns all inbounds with complete client configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response violates the
    /// v3.6.0 contract.
    pub async fn list(self) -> Result<Vec<Inbound>> {
        self.get_object("list").await
    }

    /// Returns all inbounds with slimmed `settings.clients` entries.
    ///
    /// The outer response remains [`Inbound`]; protocol-specific client JSON
    /// contains only the fields selected by 3x-ui's slim endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn list_slim(self) -> Result<Vec<Inbound>> {
        self.get_object("list/slim").await
    }

    /// Returns lightweight inbound options for selectors and dashboards.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn options(self) -> Result<Vec<InboundOption>> {
        self.get_object("options").await
    }

    /// Returns every generated inbound share link.
    ///
    /// This endpoint exists in 3x-ui v3.6.0 source but is missing from its
    /// published `OpenAPI` document.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn all_links(self) -> Result<Vec<String>> {
        self.get_object("allLinks").await
    }

    /// Fetches one inbound by identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn get(self, id: i64) -> Result<Inbound> {
        self.get_object(&format!("get/{id}")).await
    }

    /// Creates a new inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails, the request fails, or the
    /// response is invalid.
    pub async fn create(self, inbound: &InboundConfig) -> Result<Inbound> {
        self.post_object("add", Some(inbound)).await
    }

    /// Permanently deletes one inbound and returns its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when deletion fails or the response is invalid.
    pub async fn delete(self, id: i64) -> Result<i64> {
        self.post_object::<i64, ()>(&format!("del/{id}"), None)
            .await
    }

    /// Deletes multiple inbounds, retaining per-item failure reports.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation itself fails or its response
    /// is invalid.
    pub async fn delete_many(self, ids: &[i64]) -> Result<BulkDeleteInboundsResult> {
        #[derive(Serialize)]
        struct Body<'a> {
            ids: &'a [i64],
        }

        self.post_object("bulkDel", Some(&Body { ids })).await
    }

    /// Replaces an inbound's complete writable configuration.
    ///
    /// Start from [`Inbound::to_config`] when editing an existing inbound;
    /// 3x-ui's update endpoint is a replacement operation rather than PATCH.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails, the request fails, or the
    /// response is invalid.
    pub async fn update(self, id: i64, inbound: &InboundConfig) -> Result<Inbound> {
        self.post_object(&format!("update/{id}"), Some(inbound))
            .await
    }

    /// Enables or disables an inbound without sending its large settings JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot persist or apply the change.
    pub async fn set_enabled(self, id: i64, enable: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            enable: bool,
        }

        self.post_empty(&format!("setEnable/{id}"), Some(&Body { enable }))
            .await
    }

    /// Resets all traffic counters for one inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot reset the counters.
    pub async fn reset_traffic(self, id: i64) -> Result<()> {
        self.post_empty::<()>(&format!("{id}/resetTraffic"), None)
            .await
    }

    /// Deletes every client attached to one inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation itself fails or its response
    /// is invalid.
    pub async fn delete_all_clients(self, id: i64) -> Result<BulkDeleteClientsResult> {
        self.post_object::<BulkDeleteClientsResult, ()>(&format!("{id}/delAllClients"), None)
            .await
    }

    /// Resets traffic counters across every inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot reset the counters.
    pub async fn reset_all_traffic(self) -> Result<()> {
        self.post_empty::<()>("resetAllTraffics", None).await
    }

    /// Imports a complete inbound configuration using the panel's form endpoint.
    ///
    /// Unlike [`InboundsApi::create`], this preserves supplied client traffic
    /// rows while resetting panel-local database identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when JSON/form encoding, validation, transport, or
    /// response decoding fails.
    pub async fn import(self, inbound: &InboundConfig) -> Result<Inbound> {
        #[derive(Serialize)]
        struct Form<'a> {
            data: &'a str,
        }

        let data = serde_json::to_string(inbound).map_err(|source| Error::Encode {
            operation: "import inbound",
            source,
        })?;
        let path = format!("{ROOT}/import");
        let envelope = self
            .client
            .execute_form::<Inbound, _>(
                Method::POST,
                &path,
                &Form { data: &data },
                AuthenticationScope::PanelApi,
            )
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    /// Pushes a master's aggregate client traffic snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the receiving panel rejects the snapshot.
    pub async fn push_client_traffic(self, traffic: &TrafficPushRequest) -> Result<()> {
        self.post_empty("pushClientTraffics", Some(traffic)).await
    }

    /// Returns fallback rules attached to a master inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn fallbacks(self, id: i64) -> Result<Vec<InboundFallback>> {
        self.get_object(&format!("{id}/fallbacks")).await
    }

    /// Atomically replaces every fallback rule on a master inbound.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or replacement fails.
    pub async fn set_fallbacks(self, id: i64, fallbacks: &[FallbackInput]) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> {
            fallbacks: &'a [FallbackInput],
        }

        self.post_empty(&format!("{id}/fallbacks"), Some(&Body { fallbacks }))
            .await
    }

    async fn get_object<T>(self, suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute::<T, ()>(Method::GET, &path, None, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::GET, &path, envelope)
    }

    async fn post_object<T, B>(self, suffix: &str, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    async fn post_empty<B>(self, suffix: &str, body: Option<&B>) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        self.client
            .execute::<Value, B>(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }

    fn required_object<T>(self, method: Method, path: &str, envelope: ApiResponse<T>) -> Result<T> {
        let url = self.client.endpoint(path)?;
        envelope.obj.ok_or_else(|| Error::MissingObject {
            method,
            url: Box::new(url),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const SDK_ROUTES: &[(&str, &str, Option<&str>)] = &[
        (
            "get",
            "/panel/api/inbounds/list",
            Some("get_panel_api_inbounds_list"),
        ),
        (
            "get",
            "/panel/api/inbounds/list/slim",
            Some("get_panel_api_inbounds_list_slim"),
        ),
        (
            "get",
            "/panel/api/inbounds/options",
            Some("get_panel_api_inbounds_options"),
        ),
        ("get", "/panel/api/inbounds/allLinks", None),
        (
            "get",
            "/panel/api/inbounds/get/{id}",
            Some("get_panel_api_inbounds_get_id"),
        ),
        (
            "get",
            "/panel/api/inbounds/{id}/fallbacks",
            Some("get_panel_api_inbounds_id_fallbacks"),
        ),
        (
            "post",
            "/panel/api/inbounds/add",
            Some("post_panel_api_inbounds_add"),
        ),
        (
            "post",
            "/panel/api/inbounds/del/{id}",
            Some("post_panel_api_inbounds_del_id"),
        ),
        (
            "post",
            "/panel/api/inbounds/bulkDel",
            Some("post_panel_api_inbounds_bulkDel"),
        ),
        (
            "post",
            "/panel/api/inbounds/update/{id}",
            Some("post_panel_api_inbounds_update_id"),
        ),
        (
            "post",
            "/panel/api/inbounds/setEnable/{id}",
            Some("post_panel_api_inbounds_setEnable_id"),
        ),
        (
            "post",
            "/panel/api/inbounds/{id}/resetTraffic",
            Some("post_panel_api_inbounds_id_resetTraffic"),
        ),
        (
            "post",
            "/panel/api/inbounds/{id}/delAllClients",
            Some("post_panel_api_inbounds_id_delAllClients"),
        ),
        (
            "post",
            "/panel/api/inbounds/resetAllTraffics",
            Some("post_panel_api_inbounds_resetAllTraffics"),
        ),
        (
            "post",
            "/panel/api/inbounds/import",
            Some("post_panel_api_inbounds_import"),
        ),
        (
            "post",
            "/panel/api/inbounds/{id}/fallbacks",
            Some("post_panel_api_inbounds_id_fallbacks"),
        ),
        (
            "post",
            "/panel/api/inbounds/pushClientTraffics",
            Some("post_panel_api_inbounds_pushClientTraffics"),
        ),
    ];

    #[test]
    fn sdk_covers_openapi_and_source_routes() {
        let openapi: Value =
            serde_json::from_str(include_str!("../spec/3x-ui-v3.6.0.openapi.json")).unwrap();
        let paths = openapi["paths"].as_object().unwrap();
        let http_methods = [
            "get", "post", "put", "patch", "delete", "head", "options", "trace",
        ];
        let operation_count = paths
            .values()
            .filter_map(Value::as_object)
            .flat_map(serde_json::Map::keys)
            .filter(|method| http_methods.contains(&method.as_str()))
            .count();
        assert_eq!(operation_count, 160, "vendored OpenAPI changed");

        let documented = paths
            .iter()
            .flat_map(|(path, item)| {
                item.as_object().into_iter().flat_map(move |operations| {
                    operations
                        .iter()
                        .filter(|(_, operation)| {
                            operation["tags"]
                                .as_array()
                                .is_some_and(|tags| tags.iter().any(|tag| tag == "Inbounds"))
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
        assert_eq!(documented, implemented_openapi);

        let source: Value =
            serde_json::from_str(include_str!("../spec/3x-ui-v3.6.0.inbounds-routes.json"))
                .unwrap();
        let source_routes = source["routes"]
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
        let implemented_routes = SDK_ROUTES
            .iter()
            .map(|(method, path, _)| (*method, *path))
            .collect::<BTreeSet<_>>();
        assert_eq!(source_routes, implemented_routes);
    }

    #[test]
    fn defaults_match_server_normalization() {
        let config = InboundConfig::new(InboundProtocol::Vless, 443);
        assert!(config.enable);
        assert_eq!(config.sub_sort_index, 1);
        assert_eq!(config.traffic_reset, TrafficReset::Never);
        assert_eq!(config.traffic_reset_day, 1);
        assert_eq!(config.share_addr_strategy, ShareAddressStrategy::Node);
    }

    #[test]
    fn sensitive_xray_values_are_redacted_from_debug() {
        let mut config = InboundConfig::new(InboundProtocol::Trojan, 443);
        config.settings = serde_json::json!({"clients": [{"password": "secret"}]});
        config.stream_settings = serde_json::json!({"privateKey": "private-secret"});
        let output = format!("{config:?}");
        assert!(!output.contains("secret"));
        assert!(!output.contains("private-secret"));
    }

    #[test]
    fn actual_v360_option_extensions_are_typed() {
        let option: InboundOption = serde_json::from_value(serde_json::json!({
            "id": 7,
            "remark": "wg",
            "tag": "in-wg",
            "protocol": "wireguard",
            "port": 51820,
            "enable": true,
            "tlsFlowCapable": false,
            "ssMethod": "",
            "wgPublicKey": "public",
            "wgMtu": 1420,
            "wgDns": "1.1.1.1",
            "nodeId": 3,
            "nodeAddress": "node.example.com",
            "shareAddrStrategy": "node"
        }))
        .unwrap();

        assert_eq!(option.wg_mtu, 1420);
        assert_eq!(option.node_id, Some(3));
        assert_eq!(option.node_address, "node.example.com");
    }

    #[test]
    fn config_round_trips_nested_xray_json() {
        let mut config = InboundConfig::new(InboundProtocol::Vmess, 20_000);
        config.settings = serde_json::json!({"clients": [{"id": "client-secret"}]});
        let value = serde_json::to_value(&config).unwrap();
        let decoded: InboundConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(decoded, config);
        assert!(value["settings"].is_object());
    }
}
