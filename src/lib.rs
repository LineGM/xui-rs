//! A typed, asynchronous SDK for the complete 3x-ui v3.6.0 API.
//!
//! API tokens are the preferred authentication mechanism for automation.
//! Cookie sessions remain available for username/password and 2FA login flows,
//! and are required by the authenticated [`events`] stream.
//!
//! # Quick start
//!
//! ```no_run
//! use xui_rs::{Client, LoginRequest};
//!
//! # async fn run() -> xui_rs::Result<()> {
//! let client = Client::new("https://panel.example.com/secret/")?;
//! client
//!     .auth()
//!     .login(LoginRequest::new("admin", "password"))
//!     .await?;
//!
//! let status = client.server().status().await?;
//! println!("CPU: {:.1}%; Xray: {:?}", status.cpu, status.xray.state);
//!
//! client.auth().logout().await?;
//! # Ok(())
//! # }
//! ```
//!
//! Types are available both from the crate root for concise imports and from
//! domain modules such as [`inbounds`], [`clients`], [`server`], [`settings`],
//! [`hosts`], [`nodes`], [`panel`], [`events`], and [`subscription`] for
//! discoverable API documentation.

pub mod auth;
pub mod client;
pub mod clients;
pub mod error;
pub mod events;
pub mod hosts;
pub mod inbounds;
pub mod nodes;
pub mod panel;
pub mod proxy;
mod response;

#[cfg(test)]
mod remaining_http_contract_tests;

pub mod server;
pub mod settings;
pub mod subscription;

pub use auth::{AuthApi, CsrfToken, LoginRequest};
pub use client::{AuthenticationKind, Client, ClientBuilder};
pub use clients::{
    ActiveInboundsByGuid, AffectedCount, BulkAdjustRequest, BulkAdjustResult, BulkAttachResult,
    BulkClientIssue, BulkCreateResult, BulkDeleteResult, BulkDetachResult, BulkFlowAdjustment,
    BulkSetEnabledResult, ClientConfig, ClientCreateRequest, ClientDetails, ClientExternalLink,
    ClientExternalLinkInput, ClientExternalLinkKind, ClientIpEntry, ClientIpInfo, ClientIpsByGuid,
    ClientMutationStatus, ClientPage, ClientPageRequest, ClientRecord, ClientReverse, ClientSlim,
    ClientSort, ClientStatusFilter, ClientSummary, ClientWithAttachments, ClientsApi,
    ClientsByGuid, DeletedCount, GroupName, GroupSummary, LastOnlineByEmail, SortOrder,
};
pub use error::{Error, ErrorKind, Result};
pub use events::{
    ClientStatsUpdate, EventMessageType, EventStream, EventsApi, InboundTrafficSummary,
    Invalidation, NotificationLevel, PanelEvent, PanelEventKind, PanelNotification, TrafficDelta,
    TrafficUpdate, WebSocketClose, XrayStateChange,
};
pub use hosts::{
    HostGroup, HostJsonOverride, HostOptions, HostRow, HostSecurity, HostsApi, MihomoIpVersion,
    SubscriptionFormat, VlessRoute,
};
pub use inbounds::{
    BulkDeleteClientsResult, BulkDeleteInboundsResult, ClientTraffic, ClientTrafficUsage,
    FallbackInput, FallbackParent, Inbound, InboundConfig, InboundFallback, InboundOption,
    InboundProtocol, InboundsApi, ShareAddressStrategy, SkippedClient, SkippedInbound,
    TrafficPushRequest, TrafficReset,
};
pub use nodes::{
    NodeInboundSyncMode, NodeMetric, NodeMtlsCa, NodeProbeResult, NodeRequest, NodeScheme,
    NodeStatus, NodeTlsVerifyMode, NodeUpdateChannel, NodeUpdateResult, NodeView, NodesApi,
    RemoteInboundOption, RemoteInboundProtocol,
};
pub use panel::{OpenApiDocument, PanelApi};
pub use proxy::{ProxyConfig, ProxyError, ProxyScheme};
pub use server::{
    AppStats, ClientIpObservation, ClientIpRecord, DatabaseFile, DiskIo, EchKeyPair,
    Fail2banStatus, HistoryBucket, LegacyCpuPoint, LogLevel, MetricPoint, Mldsa65KeyPair,
    Mlkem768KeyPair, NetworkIo, NetworkTraffic, NodeSummary, PanelLogRequest, PanelUpdateInfo,
    PanelUpdateRun, PanelUpdateState, PanelUpdateStatus, ProcessState, PublicAddresses,
    RealityScanRequest, RealityScanResult, ResourceUsage, ServerApi, ServerStatus, SystemMetric,
    VlessEncryptionAuth, VlessEncryptionOptions, WebCertificateFiles, X25519KeyPair, XrayConfig,
    XrayLogEntry, XrayLogEvent, XrayLogRequest, XrayMetric, XrayMetricsState, XrayObservatoryEntry,
    XrayStatus,
};
pub use settings::{
    ApiTokenMetadata, BalancerStatus, CreatedApiToken, DisplaySettings, EffectiveDefaults,
    FactoryDefaults, LdapSettings, MoveDirection, OutboundDocuments, OutboundSubscription,
    OutboundSubscriptionInput, OutboundTestMode, OutboundTestResult, OutboundTraffic,
    PanelSettings, PanelSettingsUpdate, PanelSettingsView, RouteTestRequest, RouteTestResult,
    SecuritySettings, SensitivePayload, SettingsApi, SmtpSettings, SmtpTestResult,
    SubscriptionSettings, TelegramSettings, TestEgressResult, TestEndpointResult,
    UserCredentialsUpdate, WarpRegistration, WebSettings, XraySettingsApi, XraySettingsSnapshot,
};
pub use subscription::{
    SubscriptionClient, SubscriptionClientBuilder, SubscriptionDecodeError, SubscriptionDocument,
    SubscriptionInfo, SubscriptionJson, SubscriptionMetadata, SubscriptionResponse,
    SubscriptionTraffic,
};
