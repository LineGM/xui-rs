#![allow(missing_docs)]

use std::{fmt::Debug, time::Duration};

use reqwest::{Method, StatusCode};
use url::Url;
use xui_rs::{
    Client, ClientBuilder, ClientConfig, Error, ErrorKind, EventStream, InboundConfig, NodeRequest,
    PanelEvent, PanelSettingsUpdate, ProxyConfig, ProxyError, ProxyScheme, Result, ServerStatus,
    SubscriptionClient, SubscriptionClientBuilder, WebSocketClose,
    auth::{AuthApi, LoginRequest},
    clients::{ClientPageRequest, ClientsApi},
    events::{EventMessageType, EventsApi},
    hosts::{HostGroup, HostsApi},
    inbounds::{InboundProtocol, InboundsApi},
    nodes::{NodeStatus, NodesApi},
    panel::{OpenApiDocument, PanelApi},
    server::{HistoryBucket, ServerApi},
    settings::{SettingsApi, XraySettingsApi},
    subscription::{SubscriptionInfo, SubscriptionMetadata},
};

fn assert_send_sync<T: Send + Sync>() {}
fn assert_send_unpin<T: Send + Unpin>() {}
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

    assert_debug::<LoginRequest>();
    assert_debug::<PanelEvent>();
    assert_debug::<WebSocketClose>();
    assert_debug::<SubscriptionMetadata>();
    assert_debug::<ProxyConfig>();
    assert_copy::<ProxyScheme>();
}

#[test]
fn domain_modules_and_concise_root_reexports_name_the_same_types() {
    let client_config = xui_rs::clients::ClientConfig::new("alice@example.com");
    let _: ClientConfig = client_config;

    let inbound = xui_rs::inbounds::InboundConfig::new(InboundProtocol::Vless, 443);
    let _: InboundConfig = inbound;

    let node = xui_rs::nodes::NodeRequest::new("edge", "edge.example.com", 2053);
    let _: NodeRequest = node;

    let _: Option<ServerStatus> = None::<xui_rs::server::ServerStatus>;
    let _: Option<PanelSettingsUpdate> = None::<xui_rs::settings::PanelSettingsUpdate>;
    let _: Option<PanelEvent> = None::<xui_rs::events::PanelEvent>;
    let _: Option<SubscriptionInfo> = None::<xui_rs::subscription::SubscriptionInfo>;

    let _: Result<()> = Ok(());
    let _: ErrorKind = xui_rs::error::ErrorKind::Configuration;

    // Representative names from every public domain remain discoverable.
    let _ = ClientPageRequest::default();
    let _ = HostGroup::new(Vec::new(), "example");
    let _ = NodeStatus::Unknown;
    let _ = HistoryBucket::Hour1;
    let _ = EventMessageType::Status;
    let _: Option<OpenApiDocument> = None;
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

    let timeout = Error::WebSocketConnectTimeout {
        url: Box::new(url),
        timeout: Duration::from_secs(10),
    };
    assert_eq!(timeout.kind(), ErrorKind::WebSocketConnectTimeout);
    assert_eq!(timeout.status(), None);
    assert!(timeout.is_timeout());
}
