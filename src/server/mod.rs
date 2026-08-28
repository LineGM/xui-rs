//! Server status, metrics, Xray lifecycle, maintenance, and utility APIs.

mod api;
mod models;

#[cfg(test)]
mod contract_tests;

pub use api::ServerApi;
pub use models::{
    AppStats, ClientIpObservation, ClientIpRecord, DatabaseFile, DiskIo, EchKeyPair,
    Fail2banStatus, HistoryBucket, LegacyCpuPoint, LogLevel, MetricPoint, Mldsa65KeyPair,
    Mlkem768KeyPair, NetworkIo, NetworkTraffic, NodeSummary, PanelLogRequest, PanelUpdateInfo,
    PanelUpdateRun, PanelUpdateState, PanelUpdateStatus, ProcessState, PublicAddresses,
    RealityScanRequest, RealityScanResult, ResourceUsage, ServerStatus, SystemMetric,
    VlessEncryptionAuth, VlessEncryptionOptions, WebCertificateFiles, X25519KeyPair, XrayConfig,
    XrayLogEntry, XrayLogEvent, XrayLogRequest, XrayMetric, XrayMetricsState, XrayObservatoryEntry,
    XrayStatus,
};
