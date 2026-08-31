//! Panel and Xray settings APIs.

mod api;
mod models;

#[cfg(test)]
mod contract_tests;

pub use api::{SettingsApi, XraySettingsApi};
pub use models::{
    ApiTokenMetadata, BalancerStatus, CreatedApiToken, DisplaySettings, EffectiveDefaults,
    FactoryDefaults, LdapSettings, MoveDirection, OutboundDocuments, OutboundSubscription,
    OutboundSubscriptionInput, OutboundTestMode, OutboundTestResult, OutboundTraffic,
    PanelSettings, PanelSettingsUpdate, PanelSettingsView, RouteTestRequest, RouteTestResult,
    SecuritySettings, SensitivePayload, SmtpSettings, SmtpTestResult, SubscriptionSettings,
    TelegramSettings, TestEgressResult, TestEndpointResult, UserCredentialsUpdate,
    WarpRegistration, WebSettings, XraySettingsSnapshot,
};
