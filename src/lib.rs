//! A typed, asynchronous SDK for the 3x-ui panel API.
//!
//! API tokens are the preferred authentication mechanism for automation.
//! Cookie sessions remain available for username/password and 2FA login flows.

mod auth;
mod client;
mod clients;
mod error;
mod hosts;
mod inbounds;
mod response;
mod server;
mod settings;

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
pub use error::{Error, Result};
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
