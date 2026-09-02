#![allow(missing_docs)]

use std::{fmt::Debug, time::Duration};

use reqwest::{Method, StatusCode};
use url::Url;
use xui_rs::{
    Client, ClientBuilder, Error, ErrorKind, EventStream, PanelEvent, ProxyConfig, ProxyError,
    ProxyScheme, SubscriptionClient, SubscriptionClientBuilder, WebSocketClose,
    auth::{AuthApi, LoginRequest},
    clients::ClientsApi,
    events::EventsApi,
    hosts::HostsApi,
    inbounds::InboundsApi,
    nodes::NodesApi,
    panel::PanelApi,
    server::ServerApi,
    settings::{SettingsApi, XraySettingsApi},
    subscription::SubscriptionMetadata,
    subscription_balancers::SubscriptionBalancersApi,
};

macro_rules! assert_root_reexports {
    ($module:ident => $($name:ident),+ $(,)?) => {
        $(let _: Option<xui_rs::$name> = None::<xui_rs::$module::$name>;)+
    };
}

fn assert_send_sync<T: Send + Sync>() {}
fn assert_send_unpin<T: Send + Unpin>() {}
fn assert_send_value<T: Send>(_: T) {}
fn assert_copy<T: Copy>() {}
fn assert_error<T: std::error::Error + Send + Sync + 'static>() {}
fn assert_debug<T: Debug>() {}

#[test]
fn core_and_accessor_types_keep_their_async_trait_contracts() {
    assert_send_sync::<Client>();
    assert_send_sync::<ClientBuilder>();
    assert_send_sync::<SubscriptionClient>();
    assert_send_sync::<SubscriptionClientBuilder>();
    assert_send_sync::<ProxyConfig>();
    assert_send_unpin::<EventStream>();
    assert_error::<Error>();
    assert_error::<ProxyError>();

    assert_copy::<AuthApi<'static>>();
    assert_copy::<InboundsApi<'static>>();
    assert_copy::<ClientsApi<'static>>();
    assert_copy::<ServerApi<'static>>();
    assert_copy::<SettingsApi<'static>>();
    assert_copy::<XraySettingsApi<'static>>();
    assert_copy::<HostsApi<'static>>();
    assert_copy::<NodesApi<'static>>();
    assert_copy::<PanelApi<'static>>();
    assert_copy::<EventsApi<'static>>();
    assert_copy::<SubscriptionBalancersApi<'static>>();

    assert_debug::<LoginRequest>();
    assert_debug::<PanelEvent>();
    assert_debug::<WebSocketClose>();
    assert_debug::<SubscriptionMetadata>();
    assert_debug::<ProxyConfig>();
    assert_copy::<ProxyScheme>();
}

#[test]
fn every_concise_root_reexport_names_the_same_domain_type() {
    assert_root_reexports!(auth => CsrfToken, LoginRequest);
    assert_root_reexports!(client => AuthenticationKind, Client, ClientBuilder);
    assert_root_reexports!(clients =>
        ActiveInboundsByGuid, AffectedCount, BulkAdjustRequest, BulkAdjustResult,
        BulkAttachResult, BulkClientIssue, BulkCreateResult, BulkDeleteResult,
        BulkDetachResult, BulkFlowAdjustment, BulkSetEnabledResult, ClientConfig,
        ClientCreateRequest, ClientDetails, ClientExternalLink, ClientExternalLinkInput,
        ClientExternalLinkKind, ClientIpEntry, ClientIpInfo, ClientIpsByGuid,
        ClientHwidDevice, ClientMutationStatus, ClientPage, ClientPageRequest, ClientRecord, ClientReverse,
        ClientSlim, ClientSort, ClientStatusFilter, ClientSummary, ClientWithAttachments,
        ClientsByGuid, DeletedCount, GroupName, GroupSummary, LastOnlineByEmail, SortOrder,
    );
    assert_root_reexports!(error => Error, ErrorKind);
    assert_root_reexports!(events =>
        ClientStatsUpdate, EventMessageType, EventStream, InboundTrafficSummary, Invalidation,
        NotificationLevel, PanelEvent, PanelEventKind, PanelNotification, TrafficDelta,
        TrafficUpdate, WebSocketClose, XrayStateChange,
    );
    assert_root_reexports!(hosts =>
        HostGroup, HostJsonOverride, HostOptions, HostRow, HostSecurity, MihomoIpVersion,
        SubscriptionFormat, VlessRoute,
    );
    assert_root_reexports!(inbounds =>
        AmneziaWgServerSettings, BulkDeleteClientsResult, BulkDeleteInboundsResult, ClientTraffic, ClientTrafficUsage,
        FallbackInput, FallbackParent, Inbound, InboundConfig, InboundFallback, InboundOption,
        InboundProtocol, ShareAddressStrategy, SkippedClient, SkippedInbound, TrafficPushRequest,
        TrafficReset,
    );
    assert_root_reexports!(nodes =>
        NodeInboundSyncMode, NodeMetric, NodeMtlsCa, NodeProbeResult, NodeRequest, NodeScheme,
        NodeStatus, NodeTlsVerifyMode, NodeUpdateChannel, NodeUpdateResult, NodeView,
        RemoteInboundOption, RemoteInboundProtocol,
    );
    assert_root_reexports!(panel => OpenApiDocument);
    assert_root_reexports!(proxy => ProxyConfig, ProxyError, ProxyScheme);
    assert_root_reexports!(server =>
        AmneziaWgLogs, AmneziaWgPeerActivity, AmneziaWgStatus, AppStats, ClientIpObservation, ClientIpRecord, DatabaseFile, DiskIo, EchKeyPair,
        Fail2banStatus, HistoryBucket, LegacyCpuPoint, LogLevel, MetricPoint, Mldsa65KeyPair,
        Mlkem768KeyPair, NetworkIo, NetworkTraffic, NodeSummary, PanelLogRequest,
        PanelUpdateInfo, PanelUpdateRun, PanelUpdateState, PanelUpdateStatus, ProcessState,
        PublicAddresses, RealityScanRequest, RealityScanResult, ResourceUsage, ServerStatus,
        SystemMetric, VlessEncryptionAuth, VlessEncryptionOptions, WebCertificateFiles,
        X25519KeyPair, XrayConfig, XrayLogEntry, XrayLogEvent, XrayLogRequest, XrayMetric,
        XrayMetricsState, XrayObservatoryEntry, XrayStatus,
    );
    assert_root_reexports!(settings =>
        ApiTokenCreateRequest, ApiTokenMetadata, ApiTokenScope, BalancerStatus, CreatedApiToken, DisplaySettings, EffectiveDefaults,
        FactoryDefaults, GeoCategory, GeoCategoryPage, GeoEntry, GeoEntryPage, GeoFile,
        GeodataTokenIssue, LdapSettings, MoveDirection, OutboundDocuments, OutboundSubscription,
        OutboundSubscriptionInput, OutboundTestMode, OutboundTestResult, OutboundTraffic,
        PanelSettings, PanelSettingsUpdate, PanelSettingsView, PiaAccount, PiaCountry, PiaKey,
        PiaRegion, PiaServer, PiaServers, RouteTestRequest, RouteTestResult,
        SecuritySettings, SensitivePayload, SmtpSettings, SmtpTestResult, SubscriptionSettings,
        TelegramSettings, TestEgressResult, TestEndpointResult, UserCredentialsUpdate,
        WarpRegistration, WebSettings, XraySettingsSnapshot,
    );
    assert_root_reexports!(subscription =>
        SubscriptionClient, SubscriptionClientBuilder, SubscriptionDecodeError,
        SubscriptionDevice, SubscriptionDocument, SubscriptionInfo, SubscriptionJson, SubscriptionMetadata,
        SubscriptionTraffic,
    );
    assert_root_reexports!(subscription_balancers =>
        SubscriptionBalancer, SubscriptionBalancerInput, SubscriptionBalancerStrategy,
    );

    let _: Option<xui_rs::AuthApi<'static>> = None::<xui_rs::auth::AuthApi<'static>>;
    let _: Option<xui_rs::ClientsApi<'static>> = None::<xui_rs::clients::ClientsApi<'static>>;
    let _: Option<xui_rs::EventsApi<'static>> = None::<xui_rs::events::EventsApi<'static>>;
    let _: Option<xui_rs::HostsApi<'static>> = None::<xui_rs::hosts::HostsApi<'static>>;
    let _: Option<xui_rs::InboundsApi<'static>> = None::<xui_rs::inbounds::InboundsApi<'static>>;
    let _: Option<xui_rs::NodesApi<'static>> = None::<xui_rs::nodes::NodesApi<'static>>;
    let _: Option<xui_rs::PanelApi<'static>> = None::<xui_rs::panel::PanelApi<'static>>;
    let _: Option<xui_rs::ServerApi<'static>> = None::<xui_rs::server::ServerApi<'static>>;
    let _: Option<xui_rs::SettingsApi<'static>> = None::<xui_rs::settings::SettingsApi<'static>>;
    let _: Option<xui_rs::XraySettingsApi<'static>> =
        None::<xui_rs::settings::XraySettingsApi<'static>>;
    let _: Option<xui_rs::SubscriptionBalancersApi<'static>> =
        None::<xui_rs::subscription_balancers::SubscriptionBalancersApi<'static>>;
    let _: Option<xui_rs::SubscriptionResponse<()>> =
        None::<xui_rs::subscription::SubscriptionResponse<()>>;
    let _: xui_rs::Result<()> = Ok(());

    assert_eq!(
        xui_rs::DEFAULT_API_RESPONSE_BODY_LIMIT,
        xui_rs::client::DEFAULT_API_RESPONSE_BODY_LIMIT
    );
    assert_eq!(
        xui_rs::DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT,
        xui_rs::client::DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT
    );
    assert_eq!(
        xui_rs::DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT,
        xui_rs::subscription::DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT
    );
}

#[test]
fn representative_endpoint_futures_remain_send_for_multithreaded_runtimes() {
    let client = Client::new("https://panel.example.com/private/").unwrap();
    assert_send_value(client.auth().csrf_token());
    assert_send_value(client.inbounds().list());
    assert_send_value(client.clients().list());
    assert_send_value(client.server().status());
    assert_send_value(client.settings().all());
    assert_send_value(client.xray_settings().settings());
    assert_send_value(client.hosts().list());
    assert_send_value(client.nodes().list());
    assert_send_value(client.panel().openapi());
    assert_send_value(client.events().connect());
    assert_send_value(client.subscription_balancers().list());

    let subscriptions = SubscriptionClient::new("https://panel.example.com").unwrap();
    assert_send_value(subscriptions.raw("subscription-id"));
}

#[test]
fn error_introspection_does_not_require_destructuring_variants() {
    let url = Url::parse("https://panel.example.com/panel/api/server/status").unwrap();
    let unauthorized = Error::Unauthorized {
        method: Method::GET,
        url: Box::new(url.clone()),
    };
    assert_eq!(unauthorized.kind(), ErrorKind::Unauthorized);
    assert_eq!(unauthorized.kind().as_str(), "unauthorized");
    assert_eq!(unauthorized.kind().to_string(), "unauthorized");
    assert_eq!(unauthorized.status(), Some(StatusCode::UNAUTHORIZED));
    assert_eq!(unauthorized.method(), Some(&Method::GET));
    assert_eq!(unauthorized.url(), Some(&url));
    assert!(unauthorized.is_unauthorized());
    assert!(!unauthorized.is_forbidden());
    assert!(!unauthorized.is_timeout());

    let rate_limited = Error::HttpStatus {
        method: Method::POST,
        url: Box::new(url.clone()),
        status: StatusCode::TOO_MANY_REQUESTS,
    };
    assert!(rate_limited.is_rate_limited());
    assert!(!rate_limited.is_server_error());

    let unavailable = Error::HttpStatus {
        method: Method::GET,
        url: Box::new(url.clone()),
        status: StatusCode::SERVICE_UNAVAILABLE,
    };
    assert!(unavailable.is_server_error());

    let oversized = Error::ResponseTooLarge {
        method: Method::GET,
        url: Box::new(url.clone()),
        limit: 1024,
        content_length: Some(2048),
    };
    assert_eq!(oversized.kind(), ErrorKind::ResponseTooLarge);
    assert_eq!(oversized.kind().as_str(), "response_too_large");
    assert!(oversized.is_response_too_large());
    assert_eq!(oversized.response_body_limit(), Some(1024));
    assert_eq!(oversized.advertised_content_length(), Some(2048));
    assert_eq!(oversized.method(), Some(&Method::GET));
    assert_eq!(oversized.url(), Some(&url));

    let timeout = Error::WebSocketConnectTimeout {
        url: Box::new(url),
        timeout: Duration::from_secs(10),
    };
    assert_eq!(timeout.kind(), ErrorKind::WebSocketConnectTimeout);
    assert_eq!(timeout.status(), None);
    assert!(timeout.is_timeout());
}
