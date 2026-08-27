use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Current state of the managed Xray process.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ProcessState {
    /// Xray is running normally.
    Running,
    /// Xray is stopped.
    Stop,
    /// Xray failed to start or reload.
    Error,
    /// A newer server returned a state unknown to this SDK version.
    #[default]
    #[serde(other)]
    Unknown,
}

/// Used and total bytes for memory, swap, or disk capacity.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ResourceUsage {
    /// Bytes currently used.
    pub current: u64,
    /// Total available bytes.
    pub total: u64,
}

/// Cumulative disk IO counters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DiskIo {
    /// Bytes read.
    pub read: u64,
    /// Bytes written.
    pub write: u64,
}

/// Current network IO rates and packet rates.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NetworkIo {
    /// Upload bytes per sample interval.
    pub up: u64,
    /// Download bytes per sample interval.
    pub down: u64,
    /// Uploaded packets per sample interval.
    pub pkt_up: u64,
    /// Downloaded packets per sample interval.
    pub pkt_down: u64,
}

/// Cumulative network traffic counters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NetworkTraffic {
    /// Bytes sent.
    pub sent: u64,
    /// Bytes received.
    pub recv: u64,
    /// Packets sent.
    pub pkt_sent: u64,
    /// Packets received.
    pub pkt_recv: u64,
}

/// Managed Xray process state included in [`ServerStatus`].
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct XrayStatus {
    /// Process state.
    pub state: ProcessState,
    /// Last startup or reload error.
    pub error_msg: String,
    /// Installed Xray version.
    pub version: String,
}

/// Complete assembled Xray configuration.
///
/// The inner JSON remains open-ended because it depends on the installed
/// xray-core version. `Debug` is deliberately redacted because the document can
/// contain client credentials, private keys, and certificate material.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct XrayConfig(Value);

impl XrayConfig {
    /// Borrows the complete Xray JSON document.
    pub const fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consumes the wrapper and returns the complete Xray JSON document.
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl From<Value> for XrayConfig {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl fmt::Debug for XrayConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("XrayConfig")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// Public addresses detected by the panel.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct PublicAddresses {
    /// Detected IPv4 address.
    pub ipv4: String,
    /// Detected IPv6 address.
    pub ipv6: String,
}

/// Runtime resource use of the panel process itself.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct AppStats {
    /// Number of Go runtime threads.
    pub threads: u32,
    /// Resident memory in bytes.
    pub mem: u64,
    /// Panel process uptime in seconds.
    pub uptime: u64,
}

/// Complete cached machine and Xray status snapshot.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerStatus {
    /// CPU use percentage.
    pub cpu: f64,
    /// Physical CPU core count.
    pub cpu_cores: i32,
    /// Logical processor count.
    pub logical_pro: i32,
    /// Reported CPU speed in MHz.
    pub cpu_speed_mhz: f64,
    /// Memory usage.
    pub mem: ResourceUsage,
    /// Swap usage.
    pub swap: ResourceUsage,
    /// Disk usage.
    pub disk: ResourceUsage,
    /// Cumulative disk IO.
    #[serde(rename = "diskIO")]
    pub disk_io: DiskIo,
    /// Disk throughput since the previous sample.
    pub disk_traffic: DiskIo,
    /// Xray state.
    pub xray: XrayStatus,
    /// Running panel version.
    pub panel_version: String,
    /// Stable panel identifier.
    pub panel_guid: String,
    /// Host uptime in seconds.
    pub uptime: u64,
    /// Load averages reported by the host.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub loads: Vec<f64>,
    /// Open TCP connection count.
    pub tcp_count: i32,
    /// Open UDP connection count.
    pub udp_count: i32,
    /// Current network IO.
    #[serde(rename = "netIO")]
    pub net_io: NetworkIo,
    /// Cumulative network traffic.
    pub net_traffic: NetworkTraffic,
    /// Detected public addresses.
    #[serde(rename = "publicIP")]
    pub public_ip: PublicAddresses,
    /// Panel process statistics.
    pub app_stats: AppStats,
}

/// History windows accepted by v3.6.0 history endpoints.
///
/// Every response contains at most 60 points. The wire value is the bucket size
/// in seconds, while variant names describe the resulting history window shown
/// by the panel UI.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HistoryBucket {
    /// Approximately two minutes at two-second resolution.
    Minutes2,
    /// Approximately thirty minutes at thirty-second resolution.
    Minutes30,
    /// Approximately one hour at one-minute resolution.
    Hour1,
    /// Approximately three hours at three-minute resolution.
    Hours3,
    /// Approximately six hours at six-minute resolution.
    Hours6,
    /// Approximately twelve hours at twelve-minute resolution.
    Hours12,
    /// Approximately one day at 24-minute resolution.
    Day1,
    /// Approximately two days at 48-minute resolution.
    Days2,
    /// Approximately seven days at 168-minute resolution.
    Days7,
}

impl HistoryBucket {
    pub(crate) const fn seconds(self) -> u32 {
        match self {
            Self::Minutes2 => 2,
            Self::Minutes30 => 30,
            Self::Hour1 => 60,
            Self::Hours3 => 180,
            Self::Hours6 => 360,
            Self::Hours12 => 720,
            Self::Day1 => 1_440,
            Self::Days2 => 2_880,
            Self::Days7 => 10_080,
        }
    }
}

/// Host metric accepted by the uniform system history endpoint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SystemMetric {
    /// CPU percentage.
    Cpu,
    /// Memory usage.
    Memory,
    /// Swap usage.
    Swap,
    /// Upload rate.
    NetworkUp,
    /// Download rate.
    NetworkDown,
    /// Uploaded packet rate.
    PacketUp,
    /// Downloaded packet rate.
    PacketDown,
    /// Disk read throughput.
    DiskRead,
    /// Disk write throughput.
    DiskWrite,
    /// Disk usage.
    DiskUsage,
    /// Open TCP connection count.
    TcpCount,
    /// Open UDP connection count.
    UdpCount,
    /// Online client count.
    Online,
    /// One-minute load average.
    Load1,
    /// Five-minute load average.
    Load5,
    /// Fifteen-minute load average.
    Load15,
}

impl SystemMetric {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "mem",
            Self::Swap => "swap",
            Self::NetworkUp => "netUp",
            Self::NetworkDown => "netDown",
            Self::PacketUp => "pktUp",
            Self::PacketDown => "pktDown",
            Self::DiskRead => "diskRead",
            Self::DiskWrite => "diskWrite",
            Self::DiskUsage => "diskUsage",
            Self::TcpCount => "tcpCount",
            Self::UdpCount => "udpCount",
            Self::Online => "online",
            Self::Load1 => "load1",
            Self::Load5 => "load5",
            Self::Load15 => "load15",
        }
    }
}

/// Xray expvar metric accepted by its history endpoint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum XrayMetric {
    /// Allocated heap bytes.
    Alloc,
    /// Bytes obtained from the operating system.
    Sys,
    /// Live heap object count.
    HeapObjects,
    /// Completed garbage collection cycles.
    NumGc,
    /// Garbage collector pause time.
    PauseNs,
}

impl XrayMetric {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Alloc => "xrAlloc",
            Self::Sys => "xrSys",
            Self::HeapObjects => "xrHeapObjects",
            Self::NumGc => "xrNumGC",
            Self::PauseNs => "xrPauseNs",
        }
    }
}

/// One uniform time-series sample.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MetricPoint {
    /// Unix timestamp in seconds.
    #[serde(rename = "t")]
    pub timestamp: i64,
    /// Aggregated value.
    #[serde(rename = "v")]
    pub value: f64,
}

/// One sample returned by the legacy CPU-only history route.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LegacyCpuPoint {
    /// Unix timestamp in seconds.
    #[serde(rename = "t")]
    pub timestamp: i64,
    /// Aggregated CPU percentage.
    pub cpu: f64,
}

/// Xray metrics discovery and collection state.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct XrayMetricsState {
    /// Whether Xray metrics are configured and reachable.
    pub enabled: bool,
    /// Configured metrics listen address.
    pub listen: String,
    /// Explanation when metrics are unavailable.
    pub reason: String,
}

/// Latest observatory health snapshot for one outbound.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct XrayObservatoryEntry {
    /// Outbound tag.
    pub tag: String,
    /// Whether the latest probe succeeded.
    pub alive: bool,
    /// Probe delay in milliseconds.
    pub delay: i64,
    /// Last successful probe timestamp.
    pub last_seen_time: i64,
    /// Last attempted probe timestamp.
    pub last_try_time: i64,
    /// Snapshot update timestamp.
    pub updated_at: i64,
}

/// Available panel update information.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PanelUpdateInfo {
    /// `stable` or `dev`.
    pub channel: String,
    /// Installed version label.
    pub current_version: String,
    /// Latest available version label.
    pub latest_version: String,
    /// Installed commit for the dev channel.
    pub current_commit: String,
    /// Latest commit for the dev channel.
    pub latest_commit: String,
    /// Whether an update is available.
    pub update_available: bool,
}

/// Identifier returned after starting a panel self-update.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PanelUpdateRun {
    /// Decimal update run identifier.
    pub run_id: String,
}

/// State of a panel self-update.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum PanelUpdateState {
    /// Update is still running.
    Pending,
    /// Update completed successfully.
    Success,
    /// Update failed.
    Failed,
    /// A newer panel returned an unknown state.
    #[default]
    #[serde(other)]
    Unknown,
}

/// Outcome of the most recently started panel self-update.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PanelUpdateStatus {
    /// Decimal update run identifier.
    pub run_id: String,
    /// Current outcome.
    pub state: PanelUpdateState,
    /// Updater process exit code.
    pub exit_code: i32,
    /// Completion timestamp in Unix seconds, or zero while pending.
    pub finished_at: i64,
}

/// Whether IP-limit enforcement is available on this host.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Fail2banStatus {
    /// Whether integration is enabled by environment configuration.
    pub enabled: bool,
    /// Whether Fail2ban is installed.
    pub installed: bool,
    /// Whether client IP limits can be enforced.
    pub usable: bool,
    /// Whether the panel is running on Windows.
    pub windows: bool,
}

/// Read-only identity and health of a descendant panel node.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeSummary {
    /// Stable node GUID.
    pub guid: String,
    /// Stable GUID of its managing panel.
    pub parent_guid: String,
    /// Display name.
    pub name: String,
    /// Host or IP address.
    pub address: String,
    /// HTTP scheme.
    pub scheme: String,
    /// HTTP port.
    pub port: i32,
    /// Connectivity state.
    pub status: String,
    /// Last heartbeat in Unix seconds.
    pub last_heartbeat: i64,
    /// Last heartbeat latency in milliseconds.
    pub latency_ms: i32,
    /// Panel version.
    pub panel_version: String,
    /// Xray version.
    pub xray_version: String,
    /// Xray process state.
    pub xray_state: String,
    /// Last Xray error.
    pub xray_error: String,
}

/// One recently observed client source IP.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClientIpObservation {
    /// IP address.
    pub ip: String,
    /// Last-seen Unix timestamp in seconds.
    pub timestamp: i64,
}

/// Cluster-wide IP observations for one client.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientIpRecord {
    /// Database row identifier. Remote identifiers are ignored during merge.
    pub id: i32,
    /// Unique client email/name.
    pub client_email: String,
    /// Recently observed source IPs; old/invalid panel rows may return `null`.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub ips: Vec<ClientIpObservation>,
}

/// Generated X25519 Reality keypair.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct X25519KeyPair {
    /// Private key.
    pub private_key: String,
    /// Public key.
    pub public_key: String,
}

impl fmt::Debug for X25519KeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X25519KeyPair")
            .field("private_key", &"[REDACTED]")
            .field("public_key", &self.public_key)
            .finish()
    }
}

/// Generated ML-DSA-65 material.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mldsa65KeyPair {
    /// Signing seed.
    pub seed: String,
    /// Verification key.
    pub verify: String,
}

impl fmt::Debug for Mldsa65KeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mldsa65KeyPair")
            .field("seed", &"[REDACTED]")
            .field("verify", &self.verify)
            .finish()
    }
}

/// Generated ML-KEM-768 material.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mlkem768KeyPair {
    /// Server seed.
    pub seed: String,
    /// Client key.
    pub client: String,
}

impl fmt::Debug for Mlkem768KeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mlkem768KeyPair")
            .field("seed", &"[REDACTED]")
            .field("client", &self.client)
            .finish()
    }
}

/// Generated ECH server key and public config list.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EchKeyPair {
    /// Private ECH server keys.
    pub ech_server_keys: String,
    /// Public ECH configuration list.
    pub ech_config_list: String,
}

impl fmt::Debug for EchKeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EchKeyPair")
            .field("ech_server_keys", &"[REDACTED]")
            .field("ech_config_list", &self.ech_config_list)
            .finish()
    }
}

/// One generated VLESS encryption choice.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct VlessEncryptionAuth {
    /// Stable choice identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Xray encryption value.
    pub encryption: String,
    /// Xray decryption value.
    pub decryption: String,
}

impl fmt::Debug for VlessEncryptionAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VlessEncryptionAuth")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("encryption", &"[REDACTED]")
            .field("decryption", &"[REDACTED]")
            .finish()
    }
}

/// Generated VLESS encryption choices.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct VlessEncryptionOptions {
    /// Available authentication/encryption combinations.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub auths: Vec<VlessEncryptionAuth>,
}

/// TLS certificate and key paths configured for the panel web server.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WebCertificateFiles {
    /// Certificate chain path.
    pub web_cert_file: String,
    /// Private key path.
    pub web_key_file: String,
}

/// Structured result of probing a candidate REALITY target.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct RealityScanResult {
    /// Original target.
    pub target: String,
    /// TLS server name.
    pub host: String,
    /// Resolved address.
    #[serde(rename = "ip")]
    pub ip_address: String,
    /// Target port.
    pub port: i32,
    /// Overall REALITY feasibility verdict.
    pub feasible: bool,
    /// Whether TLS 1.3 was negotiated.
    #[serde(rename = "TLS13")]
    pub tls13: bool,
    /// Negotiated TLS version.
    #[serde(rename = "TLSVersion")]
    pub tls_version: String,
    /// Whether HTTP/2 was negotiated.
    #[serde(rename = "H2")]
    pub h2: bool,
    /// Negotiated ALPN.
    #[serde(rename = "ALPN")]
    pub alpn: String,
    /// Whether X25519 was negotiated.
    #[serde(rename = "X25519")]
    pub x25519: bool,
    /// Negotiated curve identifier.
    pub curve_id: String,
    /// Whether the certificate validated.
    pub cert_valid: bool,
    /// Leaf certificate subject.
    pub cert_subject: String,
    /// Leaf certificate issuer.
    pub cert_issuer: String,
    /// Certificate expiration in RFC 3339 form.
    pub not_after: String,
    /// Usable certificate server names.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub server_names: Vec<String>,
    /// End-to-end probe latency in milliseconds.
    pub latency_ms: i32,
    /// Failure or infeasibility reason.
    pub reason: String,
}

/// Input for probing one REALITY target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealityScanRequest {
    /// Host, host:port, IP, or IP:port.
    pub target: String,
    /// Optional Xray version compatibility selector used by the panel.
    pub xray_version: i32,
}

impl RealityScanRequest {
    /// Creates a scan request using the panel's default compatibility mode.
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            xray_version: 0,
        }
    }
}

/// Panel application log level filter.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LogLevel {
    /// No level filtering.
    #[default]
    All,
    /// Debug and above.
    Debug,
    /// Informational and above.
    Info,
    /// Warnings and errors.
    Warning,
    /// Errors only.
    Error,
}

impl LogLevel {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Filters for reading panel logs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PanelLogRequest {
    /// Maximum number of trailing lines.
    pub count: u32,
    /// Minimum log level.
    pub level: LogLevel,
    /// Read the system journal instead of the panel log file.
    pub syslog: bool,
}

/// Filters for reading Xray logs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct XrayLogRequest {
    /// Maximum number of trailing lines.
    pub count: u32,
    /// Free-text filter.
    pub filter: String,
    /// Include direct/freedom traffic.
    pub show_direct: bool,
    /// Include blocked/blackhole traffic.
    pub show_blocked: bool,
    /// Include proxy traffic.
    pub show_proxy: bool,
}

/// Traffic classification assigned to one parsed Xray access-log entry.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum XrayLogEvent {
    /// Direct/freedom outbound.
    #[default]
    Direct,
    /// Blocked/blackhole outbound.
    Blocked,
    /// Proxied outbound.
    Proxied,
    /// A newer panel returned an unknown numeric classification.
    Unknown,
}

impl From<i32> for XrayLogEvent {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Direct,
            1 => Self::Blocked,
            2 => Self::Proxied,
            _ => Self::Unknown,
        }
    }
}

impl From<XrayLogEvent> for i32 {
    fn from(value: XrayLogEvent) -> Self {
        match value {
            XrayLogEvent::Direct => 0,
            XrayLogEvent::Blocked => 1,
            XrayLogEvent::Proxied => 2,
            XrayLogEvent::Unknown => -1,
        }
    }
}

impl<'de> Deserialize<'de> for XrayLogEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        i32::deserialize(deserializer).map(Self::from)
    }
}

impl Serialize for XrayLogEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        i32::from(*self).serialize(serializer)
    }
}

/// One structured entry parsed from the Xray access log.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct XrayLogEntry {
    /// UTC timestamp in RFC 3339 form.
    pub date_time: String,
    /// Client source address.
    pub from_address: String,
    /// Requested destination address.
    pub to_address: String,
    /// Inbound tag.
    pub inbound: String,
    /// Outbound tag.
    pub outbound: String,
    /// Client email/name when present.
    pub email: String,
    /// Direct, blocked, or proxied classification.
    pub event: XrayLogEvent,
}

/// Binary database backup or migration downloaded from the panel.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct DatabaseFile {
    /// Suggested attachment filename, when supplied by the panel.
    pub filename: Option<String>,
    /// Response content type, when supplied by the panel.
    pub content_type: Option<String>,
    /// Complete file contents.
    pub bytes: Vec<u8>,
}

impl fmt::Debug for DatabaseFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DatabaseFile")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field(
                "bytes",
                &format_args!("[REDACTED; {} bytes]", self.bytes.len()),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_variants_match_the_source_allowlists() {
        let buckets = [
            HistoryBucket::Minutes2,
            HistoryBucket::Minutes30,
            HistoryBucket::Hour1,
            HistoryBucket::Hours3,
            HistoryBucket::Hours6,
            HistoryBucket::Hours12,
            HistoryBucket::Day1,
            HistoryBucket::Days2,
            HistoryBucket::Days7,
        ]
        .map(HistoryBucket::seconds);
        assert_eq!(buckets, [2, 30, 60, 180, 360, 720, 1_440, 2_880, 10_080]);

        let metrics = [
            SystemMetric::Cpu,
            SystemMetric::Memory,
            SystemMetric::Swap,
            SystemMetric::NetworkUp,
            SystemMetric::NetworkDown,
            SystemMetric::PacketUp,
            SystemMetric::PacketDown,
            SystemMetric::DiskRead,
            SystemMetric::DiskWrite,
            SystemMetric::DiskUsage,
            SystemMetric::TcpCount,
            SystemMetric::UdpCount,
            SystemMetric::Online,
            SystemMetric::Load1,
            SystemMetric::Load5,
            SystemMetric::Load15,
        ]
        .map(SystemMetric::as_str);
        assert_eq!(
            metrics,
            [
                "cpu",
                "mem",
                "swap",
                "netUp",
                "netDown",
                "pktUp",
                "pktDown",
                "diskRead",
                "diskWrite",
                "diskUsage",
                "tcpCount",
                "udpCount",
                "online",
                "load1",
                "load5",
                "load15",
            ]
        );
    }
}
