//! A typed, asynchronous SDK for the 3x-ui panel API.
//!
//! API tokens are the preferred authentication mechanism for automation.
//! Cookie sessions remain available for username/password and 2FA login flows.

mod auth;
mod client;
mod error;
mod inbounds;
mod response;

pub use auth::{AuthApi, CsrfToken, LoginRequest};
pub use client::{AuthenticationKind, Client, ClientBuilder};
pub use error::{Error, Result};
pub use inbounds::{
    BulkDeleteClientsResult, BulkDeleteInboundsResult, ClientTraffic, ClientTrafficUsage,
    FallbackInput, FallbackParent, Inbound, InboundConfig, InboundFallback, InboundOption,
    InboundProtocol, InboundsApi, ShareAddressStrategy, SkippedClient, SkippedInbound,
    TrafficPushRequest, TrafficReset,
};
