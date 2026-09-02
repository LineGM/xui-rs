//! Panel and Xray settings APIs.

mod api;
mod models;

#[cfg(test)]
mod contract_tests;

pub use api::{SettingsApi, XraySettingsApi};
pub use models::{
    ApiTokenCreateRequest, ApiTokenMetadata, ApiTokenScope, BalancerStatus, CreatedApiToken,
    DisplaySettings, EffectiveDefaults, FactoryDefaults, GeoCategory, GeoCategoryPage, GeoEntry,
    GeoEntryPage, GeoFile, GeodataTokenIssue, LdapSettings, MoveDirection, OutboundDocuments,
    OutboundSubscription, OutboundSubscriptionInput, OutboundTestMode, OutboundTestResult,
    OutboundTraffic, PanelSettings, PanelSettingsUpdate, PanelSettingsView, PiaAccount, PiaCountry,
    PiaKey, PiaRegion, PiaServer, PiaServers, RouteTestRequest, RouteTestResult, SecuritySettings,
    SensitivePayload, SmtpSettings, SmtpTestResult, SubscriptionSettings, TelegramSettings,
    TestEgressResult, TestEndpointResult, UserCredentialsUpdate, WarpRegistration, WebSettings,
    XraySettingsSnapshot,
};
