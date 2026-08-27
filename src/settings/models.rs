#![allow(clippy::struct_excessive_bools)]

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Panel listener, session, and outbound settings.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WebSettings {
    /// Listener IP, or an empty string for all interfaces.
    pub web_listen: String,
    /// Public panel domain.
    pub web_domain: String,
    /// Panel TCP port.
    pub web_port: u16,
    /// TLS certificate path.
    pub web_cert_file: String,
    /// TLS private-key path.
    pub web_key_file: String,
    /// Secret panel base path.
    pub web_base_path: String,
    /// Session lifetime in minutes.
    pub session_max_age: u32,
    /// Comma-separated trusted reverse-proxy CIDRs.
    #[serde(rename = "trustedProxyCIDRs")]
    pub trusted_proxy_cidrs: String,
    /// Xray outbound tag used by panel-originated traffic.
    pub panel_outbound: String,
}

impl fmt::Debug for WebSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebSettings")
            .field("web_listen", &self.web_listen)
            .field("web_domain", &self.web_domain)
            .field("web_port", &self.web_port)
            .field("web_cert_file", &self.web_cert_file)
            .field("web_key_file", &"[REDACTED]")
            .field("web_base_path", &"[REDACTED]")
            .field("session_max_age", &self.session_max_age)
            .field("trusted_proxy_cidrs", &self.trusted_proxy_cidrs)
            .field("panel_outbound", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Panel display and warning thresholds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplaySettings {
    /// Rows per page.
    pub page_size: u32,
    /// Expiration warning window.
    pub expire_diff: u32,
    /// Traffic warning percentage.
    pub traffic_diff: u8,
    /// Inbound remark template.
    pub remark_template: String,
    /// Shows client identity on every generated link.
    pub sub_show_identity_on_all_links: bool,
    /// Date picker calendar identifier.
    pub datepicker: String,
}

/// Telegram notification settings.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TelegramSettings {
    /// Enables the Telegram bot.
    pub tg_bot_enable: bool,
    /// Telegram bot token. A blank update preserves the stored token.
    pub tg_bot_token: String,
    /// Optional bot proxy URL.
    pub tg_bot_proxy: String,
    /// Optional Telegram-compatible API server.
    #[serde(rename = "tgBotAPIServer")]
    pub tg_bot_api_server: String,
    /// Target chat identifier.
    pub tg_bot_chat_id: String,
    /// Scheduled report time.
    pub tg_run_time: String,
    /// Enables database backups through Telegram.
    pub tg_bot_backup: bool,
    /// CPU warning percentage.
    pub tg_cpu: u8,
    /// Memory warning percentage.
    pub tg_memory: u8,
    /// Notification language.
    pub tg_lang: String,
    /// Comma-separated enabled event identifiers.
    pub tg_enabled_events: String,
}

impl fmt::Debug for TelegramSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TelegramSettings")
            .field("tg_bot_enable", &self.tg_bot_enable)
            .field("tg_bot_token", &"[REDACTED]")
            .field("tg_bot_proxy", &"[REDACTED]")
            .field("tg_bot_api_server", &self.tg_bot_api_server)
            .field("tg_bot_chat_id", &self.tg_bot_chat_id)
            .field("tg_run_time", &self.tg_run_time)
            .field("tg_bot_backup", &self.tg_bot_backup)
            .field("tg_cpu", &self.tg_cpu)
            .field("tg_memory", &self.tg_memory)
            .field("tg_lang", &self.tg_lang)
            .field("tg_enabled_events", &self.tg_enabled_events)
            .finish()
    }
}

/// SMTP notification settings.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SmtpSettings {
    /// Enables SMTP notifications.
    pub smtp_enable: bool,
    /// SMTP server host.
    pub smtp_host: String,
    /// SMTP server port.
    pub smtp_port: u16,
    /// SMTP username.
    pub smtp_username: String,
    /// SMTP password. A blank update preserves the stored password.
    pub smtp_password: String,
    /// Envelope sender address.
    pub smtp_from: String,
    /// Display name for the sender.
    pub smtp_from_name: String,
    /// Recipient address list.
    pub smtp_to: String,
    /// Encryption mode understood by 3x-ui.
    pub smtp_encryption_type: String,
    /// Comma-separated enabled event identifiers.
    pub smtp_enabled_events: String,
    /// CPU warning percentage.
    pub smtp_cpu: u8,
    /// Memory warning percentage.
    pub smtp_memory: u8,
}

impl fmt::Debug for SmtpSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SmtpSettings")
            .field("smtp_enable", &self.smtp_enable)
            .field("smtp_host", &self.smtp_host)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_username", &self.smtp_username)
            .field("smtp_password", &"[REDACTED]")
            .field("smtp_from", &self.smtp_from)
            .field("smtp_from_name", &self.smtp_from_name)
            .field("smtp_to", &self.smtp_to)
            .field("smtp_encryption_type", &self.smtp_encryption_type)
            .field("smtp_enabled_events", &self.smtp_enabled_events)
            .field("smtp_cpu", &self.smtp_cpu)
            .field("smtp_memory", &self.smtp_memory)
            .finish()
    }
}

/// Security, time-zone, and integration health settings.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SecuritySettings {
    /// Percentage below which an outbound is treated as down.
    pub outbound_down_threshold: u8,
    /// IANA time-zone name.
    pub time_location: String,
    /// Enables two-factor authentication.
    pub two_factor_enable: bool,
    /// Two-factor seed. It is redacted on reads.
    pub two_factor_token: String,
    /// Automatic WARP IP update interval.
    pub warp_update_interval: u32,
}

impl fmt::Debug for SecuritySettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecuritySettings")
            .field("outbound_down_threshold", &self.outbound_down_threshold)
            .field("time_location", &self.time_location)
            .field("two_factor_enable", &self.two_factor_enable)
            .field("two_factor_token", &"[REDACTED]")
            .field("warp_update_interval", &self.warp_update_interval)
            .finish()
    }
}

/// Subscription server and generated-format settings.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionSettings {
    /// Enables the subscription server.
    pub sub_enable: bool,
    /// Enables JSON subscriptions.
    pub sub_json_enable: bool,
    /// Auto-detects JSON subscription clients.
    pub sub_json_auto_detect: bool,
    /// Always emits a JSON array.
    pub sub_json_always_array: bool,
    /// User-agent regex for JSON subscriptions.
    pub sub_json_user_agent_regex: String,
    /// Auto-detects Clash clients.
    pub sub_clash_auto_detect: bool,
    /// User-agent regex for Clash subscriptions.
    pub sub_clash_user_agent_regex: String,
    /// Subscription title.
    pub sub_title: String,
    /// Support URL.
    pub sub_support_url: String,
    /// Profile URL.
    pub sub_profile_url: String,
    /// Announcement text.
    pub sub_announce: String,
    /// Enables base subscription routing rules.
    pub sub_enable_routing: bool,
    /// Base subscription routing rules.
    pub sub_routing_rules: String,
    /// Enables Incy routing rules.
    pub sub_incy_enable_routing: bool,
    /// Incy routing rules.
    pub sub_incy_routing_rules: String,
    /// Subscription listener IP.
    pub sub_listen: String,
    /// Subscription server port.
    pub sub_port: u16,
    /// Base subscription path.
    pub sub_path: String,
    /// Subscription domain.
    pub sub_domain: String,
    /// Subscription TLS certificate path.
    pub sub_cert_file: String,
    /// Subscription TLS private-key path.
    pub sub_key_file: String,
    /// Subscription update interval.
    pub sub_updates: u32,
    /// Enables external traffic reporting.
    pub external_traffic_inform_enable: bool,
    /// External traffic reporting URI.
    #[serde(rename = "externalTrafficInformURI")]
    pub external_traffic_inform_uri: String,
    /// Restarts Xray when a client is disabled.
    pub restart_xray_on_client_disable: bool,
    /// Encrypts subscription identifiers.
    pub sub_encrypt: bool,
    /// Public base subscription URI.
    #[serde(rename = "subURI")]
    pub sub_uri: String,
    /// JSON subscription path.
    pub sub_json_path: String,
    /// Public JSON subscription URI.
    #[serde(rename = "subJsonURI")]
    pub sub_json_uri: String,
    /// Enables Clash subscriptions.
    pub sub_clash_enable: bool,
    /// Clash subscription path.
    pub sub_clash_path: String,
    /// Public Clash subscription URI.
    #[serde(rename = "subClashURI")]
    pub sub_clash_uri: String,
    /// Enables Clash routing.
    pub sub_clash_enable_routing: bool,
    /// Clash routing rules.
    pub sub_clash_rules: String,
    /// JSON mux template.
    pub sub_json_mux: String,
    /// JSON routing rules.
    pub sub_json_rules: String,
    /// JSON final-mask template.
    pub sub_json_final_mask: String,
    /// Subscription theme directory.
    pub sub_theme_dir: String,
    /// Hides settings from subscription output.
    pub sub_hide_settings: bool,
}

impl fmt::Debug for SubscriptionSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionSettings")
            .field("sub_enable", &self.sub_enable)
            .field("sub_json_enable", &self.sub_json_enable)
            .field("sub_json_auto_detect", &self.sub_json_auto_detect)
            .field("sub_json_always_array", &self.sub_json_always_array)
            .field("sub_clash_auto_detect", &self.sub_clash_auto_detect)
            .field("sub_title", &self.sub_title)
            .field("sub_listen", &self.sub_listen)
            .field("sub_port", &self.sub_port)
            .field("sub_domain", &self.sub_domain)
            .field("sub_updates", &self.sub_updates)
            .field(
                "external_traffic_inform_enable",
                &self.external_traffic_inform_enable,
            )
            .field(
                "restart_xray_on_client_disable",
                &self.restart_xray_on_client_disable,
            )
            .field("sub_encrypt", &self.sub_encrypt)
            .field("sub_clash_enable", &self.sub_clash_enable)
            .field("sub_clash_enable_routing", &self.sub_clash_enable_routing)
            .field("sub_hide_settings", &self.sub_hide_settings)
            .field("paths_uris_rules_and_key_material", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// LDAP synchronization and client defaults.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LdapSettings {
    /// Enables LDAP synchronization.
    pub ldap_enable: bool,
    /// LDAP host.
    pub ldap_host: String,
    /// LDAP port.
    pub ldap_port: u16,
    /// Enables LDAP TLS.
    #[serde(rename = "ldapUseTLS")]
    pub ldap_use_tls: bool,
    /// Disables LDAP certificate verification.
    pub ldap_insecure_skip_verify: bool,
    /// Bind distinguished name.
    #[serde(rename = "ldapBindDN")]
    pub ldap_bind_dn: String,
    /// Bind password. A blank update preserves the stored password.
    pub ldap_password: String,
    /// Search base distinguished name.
    #[serde(rename = "ldapBaseDN")]
    pub ldap_base_dn: String,
    /// LDAP user filter.
    pub ldap_user_filter: String,
    /// LDAP username attribute.
    pub ldap_user_attr: String,
    /// LDAP field used for VLESS identity.
    pub ldap_vless_field: String,
    /// Synchronization cron expression.
    pub ldap_sync_cron: String,
    /// LDAP enabled/disabled flag field.
    pub ldap_flag_field: String,
    /// Values treated as true for the flag field.
    pub ldap_truthy_values: String,
    /// Inverts the LDAP flag.
    pub ldap_invert_flag: bool,
    /// Inbound tags assigned to synchronized clients.
    pub ldap_inbound_tags: String,
    /// Creates missing panel clients.
    pub ldap_auto_create: bool,
    /// Deletes panel clients absent from LDAP.
    pub ldap_auto_delete: bool,
    /// Default traffic quota in gigabytes.
    #[serde(rename = "ldapDefaultTotalGB")]
    pub ldap_default_total_gb: u64,
    /// Default expiry in days.
    pub ldap_default_expiry_days: u32,
    /// Default IP limit.
    #[serde(rename = "ldapDefaultLimitIP")]
    pub ldap_default_limit_ip: u32,
}

impl fmt::Debug for LdapSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LdapSettings")
            .field("ldap_enable", &self.ldap_enable)
            .field("ldap_host", &self.ldap_host)
            .field("ldap_port", &self.ldap_port)
            .field("ldap_use_tls", &self.ldap_use_tls)
            .field("ldap_insecure_skip_verify", &self.ldap_insecure_skip_verify)
            .field("ldap_bind_dn", &self.ldap_bind_dn)
            .field("ldap_password", &"[REDACTED]")
            .field("ldap_base_dn", &self.ldap_base_dn)
            .field("ldap_user_filter", &self.ldap_user_filter)
            .field("ldap_user_attr", &self.ldap_user_attr)
            .field("ldap_vless_field", &self.ldap_vless_field)
            .field("ldap_sync_cron", &self.ldap_sync_cron)
            .field("ldap_flag_field", &self.ldap_flag_field)
            .field("ldap_truthy_values", &self.ldap_truthy_values)
            .field("ldap_invert_flag", &self.ldap_invert_flag)
            .field("ldap_inbound_tags", &self.ldap_inbound_tags)
            .field("ldap_auto_create", &self.ldap_auto_create)
            .field("ldap_auto_delete", &self.ldap_auto_delete)
            .field("ldap_default_total_gb", &self.ldap_default_total_gb)
            .field("ldap_default_expiry_days", &self.ldap_default_expiry_days)
            .field("ldap_default_limit_ip", &self.ldap_default_limit_ip)
            .finish()
    }
}

/// Complete persisted panel settings, grouped ergonomically but flattened on the wire.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct PanelSettings {
    /// Web listener and session settings.
    #[serde(flatten)]
    pub web: WebSettings,
    /// Display settings.
    #[serde(flatten)]
    pub display: DisplaySettings,
    /// Telegram settings.
    #[serde(flatten)]
    pub telegram: TelegramSettings,
    /// SMTP settings.
    #[serde(flatten)]
    pub smtp: SmtpSettings,
    /// Security and integration settings.
    #[serde(flatten)]
    pub security: SecuritySettings,
    /// Subscription server settings.
    #[serde(flatten)]
    pub subscriptions: SubscriptionSettings,
    /// LDAP settings.
    #[serde(flatten)]
    pub ldap: LdapSettings,
}

/// Browser-safe settings view with secret-presence indicators.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PanelSettingsView {
    /// Redacted settings.
    #[serde(flatten)]
    pub settings: PanelSettings,
    /// A Telegram token is stored.
    pub has_tg_bot_token: bool,
    /// A two-factor seed is stored.
    pub has_two_factor_token: bool,
    /// An LDAP password is stored.
    pub has_ldap_password: bool,
    /// At least one API token exists.
    pub has_api_token: bool,
    /// WARP credentials are stored.
    pub has_warp_secret: bool,
    /// `NordVPN` credentials are stored.
    pub has_nord_secret: bool,
    /// An SMTP password is stored.
    pub has_smtp_password: bool,
}

/// Full-replacement settings update plus request-only secret controls.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelSettingsUpdate {
    /// Complete settings replacement.
    #[serde(flatten)]
    pub settings: PanelSettings,
    /// Current 2FA code, required when disabling 2FA.
    pub two_factor_code: String,
    /// Explicitly clears the stored Telegram token.
    pub clear_tg_bot_token: bool,
    /// Explicitly clears the stored LDAP password.
    pub clear_ldap_password: bool,
    /// Explicitly clears the stored SMTP password.
    pub clear_smtp_password: bool,
}

impl PanelSettingsUpdate {
    /// Creates an update that preserves redacted blank secrets.
    pub fn new(settings: PanelSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }
}

/// Host-derived defaults used by the panel UI.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EffectiveDefaults {
    /// Expiration warning window.
    pub expire_diff: i64,
    /// Traffic warning percentage.
    pub traffic_diff: i64,
    /// Page size.
    pub page_size: i64,
    /// Default certificate path.
    pub default_cert: String,
    /// Default key path.
    pub default_key: String,
    /// Whether Telegram is enabled.
    pub tg_bot_enable: bool,
    /// Subscription theme directory.
    pub sub_theme_dir: String,
    /// Whether the subscription server is enabled.
    pub sub_enable: bool,
    /// Whether JSON subscriptions are enabled.
    pub sub_json_enable: bool,
    /// Whether Clash subscriptions are enabled.
    pub sub_clash_enable: bool,
    /// Subscription title.
    pub sub_title: String,
    /// Base subscription URI.
    #[serde(rename = "subURI")]
    pub sub_uri: String,
    /// JSON subscription URI.
    #[serde(rename = "subJsonURI")]
    pub sub_json_uri: String,
    /// Clash subscription URI.
    #[serde(rename = "subClashURI")]
    pub sub_clash_uri: String,
    /// Calendar type.
    pub datepicker: String,
    /// Whether IP limits are active.
    pub ip_limit_enable: bool,
    /// Whether access logging is active.
    pub access_log_enable: bool,
    /// Web domain.
    pub web_domain: String,
    /// Subscription domain.
    pub sub_domain: String,
    /// Whether the development update channel is enabled.
    pub dev_channel_enable: bool,
    /// Whether this panel is a development build.
    pub is_dev_build: bool,
}

/// Factory defaults keyed by their 3x-ui setting names.
pub type FactoryDefaults = BTreeMap<String, String>;

/// Current and replacement panel credentials.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCredentialsUpdate {
    /// Current username.
    pub old_username: String,
    /// Current password.
    pub old_password: String,
    /// Replacement username.
    pub new_username: String,
    /// Replacement password.
    pub new_password: String,
    /// Current 2FA code when enabled.
    pub two_factor_code: String,
}

impl fmt::Debug for UserCredentialsUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UserCredentialsUpdate")
            .field("old_username", &self.old_username)
            .field("old_password", &"[REDACTED]")
            .field("new_username", &self.new_username)
            .field("new_password", &"[REDACTED]")
            .field("two_factor_code", &"[REDACTED]")
            .finish()
    }
}

/// Metadata for a panel API token. The plaintext token is never returned by list calls.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTokenMetadata {
    /// Database identifier.
    pub id: i64,
    /// User-provided token name.
    pub name: String,
    /// Whether authentication with this token is enabled.
    pub enabled: bool,
    /// Unix creation timestamp in seconds.
    pub created_at: i64,
}

/// Newly created API token, whose plaintext value is shown exactly once.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedApiToken {
    /// Database identifier.
    pub id: i64,
    /// User-provided token name.
    pub name: String,
    /// Plaintext bearer token.
    pub token: String,
    /// Whether authentication with this token is enabled.
    pub enabled: bool,
    /// Unix creation timestamp in seconds.
    pub created_at: i64,
}

impl fmt::Debug for CreatedApiToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedApiToken")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("token", &"[REDACTED]")
            .field("enabled", &self.enabled)
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Structured result of the panel's SMTP connection test.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SmtpTestResult {
    /// Whether every test stage succeeded.
    pub success: bool,
    /// Stage that succeeded or failed.
    #[serde(default)]
    pub stage: String,
    /// Human-readable result message.
    #[serde(default, rename = "msg")]
    pub message: String,
}

/// Open-ended Xray JSON returned by an integration action.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SensitivePayload(String);

impl SensitivePayload {
    /// Borrows the original response string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses the response as JSON when the action returned JSON.
    ///
    /// # Errors
    ///
    /// Returns a JSON decoding error when the response is empty or not JSON.
    pub fn json(&self) -> serde_json::Result<Value> {
        serde_json::from_str(&self.0)
    }
}

impl fmt::Debug for SensitivePayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SensitivePayload")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// WARP key material used for registration.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpRegistration {
    /// Existing WARP private key, or an empty string to generate one.
    pub private_key: String,
    /// Existing WARP public key, or an empty string to generate one.
    pub public_key: String,
}

impl fmt::Debug for WarpRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WarpRegistration")
            .field("private_key", &"[REDACTED]")
            .field("public_key", &self.public_key)
            .finish()
    }
}

/// Decoded Xray settings response, including runtime-only subscription outbounds.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct XraySettingsSnapshot {
    /// Editable Xray template.
    pub xray_setting: crate::XrayConfig,
    /// Known inbound tags.
    pub inbound_tags: Vec<String>,
    /// Client reverse-proxy tags.
    pub client_reverse_tags: Vec<String>,
    /// URL used for outbound tests.
    pub outbound_test_url: String,
    /// Runtime-injected subscription outbounds.
    pub subscription_outbounds: Vec<Value>,
    /// Tags of runtime-injected subscription outbounds.
    pub subscription_outbound_tags: Vec<String>,
}

impl Default for XraySettingsSnapshot {
    fn default() -> Self {
        Self {
            xray_setting: crate::XrayConfig::from(Value::Object(serde_json::Map::new())),
            inbound_tags: Vec::new(),
            client_reverse_tags: Vec::new(),
            outbound_test_url: String::new(),
            subscription_outbounds: Vec::new(),
            subscription_outbound_tags: Vec::new(),
        }
    }
}

impl fmt::Debug for XraySettingsSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("XraySettingsSnapshot")
            .field("xray_setting", &"[REDACTED]")
            .field("inbound_tags", &self.inbound_tags)
            .field("client_reverse_tags", &self.client_reverse_tags)
            .field("outbound_test_url", &self.outbound_test_url)
            .field("subscription_outbounds", &"[REDACTED]")
            .field(
                "subscription_outbound_tags",
                &self.subscription_outbound_tags,
            )
            .finish()
    }
}

/// Cumulative traffic counters for one Xray outbound.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct OutboundTraffic {
    /// Database identifier, when present.
    pub id: i64,
    /// Outbound tag.
    pub tag: String,
    /// Uploaded bytes.
    pub up: i64,
    /// Downloaded bytes.
    pub down: i64,
    /// Total byte allowance or counter.
    pub total: i64,
}

/// Probe mode for outbound tests.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum OutboundTestMode {
    /// Full HTTP probe through a temporary Xray instance.
    #[default]
    Http,
    /// Fast TCP dial-only probe.
    Tcp,
    /// Reports the cold full-request delay.
    Real,
}

impl OutboundTestMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "",
            Self::Tcp => "tcp",
            Self::Real => "real",
        }
    }
}

/// Result for one endpoint in a TCP-mode outbound test.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct TestEndpointResult {
    /// Endpoint address.
    pub address: String,
    /// Whether dialing succeeded.
    pub success: bool,
    /// Dial delay in milliseconds.
    pub delay: i64,
    /// Error message.
    pub error: String,
}

/// Public egress identity observed by an HTTP-mode test.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct TestEgressResult {
    /// Observed IPv4 address.
    pub ipv4: String,
    /// Observed IPv6 address.
    pub ipv6: String,
    /// Observed country code.
    pub country: String,
    /// Cloudflare WARP status.
    pub warp: String,
}

/// Result of an outbound connectivity test.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OutboundTestResult {
    /// Tested outbound tag.
    pub tag: String,
    /// Whether the probe succeeded.
    pub success: bool,
    /// End-to-end delay in milliseconds.
    pub delay: i64,
    /// Failure message.
    pub error: String,
    /// Effective probe mode.
    pub mode: String,
    /// HTTP status returned by the test URL.
    pub http_status: i32,
    /// Local test-inbound connection time in milliseconds.
    pub connect_ms: i64,
    /// Outbound-chain and target TLS time in milliseconds.
    #[serde(rename = "tlsMs")]
    pub tls_ms: i64,
    /// Time to first byte in milliseconds.
    #[serde(rename = "ttfbMs")]
    pub ttfb_ms: i64,
    /// Per-endpoint TCP results.
    pub endpoints: Vec<TestEndpointResult>,
    /// Observed egress identity.
    pub egress: Option<TestEgressResult>,
}

/// Current live status of one Xray balancer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct BalancerStatus {
    /// Balancer tag.
    pub tag: String,
    /// Whether the balancer exists in the running core.
    pub running: bool,
    /// Forced outbound target, if any.
    #[serde(rename = "override")]
    pub override_target: String,
    /// Targets selected by the strategy.
    pub selected: Vec<String>,
}

/// Synthetic connection used to test Xray routing.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteTestRequest {
    /// Simulated inbound tag.
    pub inbound_tag: String,
    /// Destination domain.
    pub domain: String,
    /// Destination IP.
    pub ip: String,
    /// Destination port.
    pub port: u16,
    /// Network, usually `tcp` or `udp`.
    pub network: String,
    /// Optional sniffed protocol.
    pub protocol: String,
    /// Optional user email.
    pub email: String,
}

/// Routing decision returned by Xray.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RouteTestResult {
    /// Whether a routing rule matched.
    pub matched: bool,
    /// Selected outbound tag.
    pub outbound_tag: String,
    /// Balancer chain traversed by the decision.
    pub group_tags: Vec<String>,
}

/// Stored remote outbound subscription.
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OutboundSubscription {
    /// Database identifier.
    pub id: i64,
    /// User-visible remark.
    pub remark: String,
    /// Remote subscription URL.
    pub url: String,
    /// Whether it contributes outbounds.
    pub enabled: bool,
    /// Whether private destination addresses are allowed.
    pub allow_private: bool,
    /// Whether invalid TLS certificates are accepted.
    pub allow_insecure: bool,
    /// Prefix applied to generated outbound tags.
    pub tag_prefix: String,
    /// Refresh interval in seconds.
    pub update_interval: i32,
    /// Merge priority.
    pub priority: i32,
    /// Whether remote outbounds precede manual outbounds.
    pub prepend: bool,
    /// Last successful refresh timestamp.
    pub last_updated: i64,
    /// Last refresh error.
    pub last_error: String,
    /// Cached raw outbounds, normally redacted by list operations.
    pub last_fetched_outbounds: String,
    /// Unix creation timestamp in milliseconds.
    pub created_at: i64,
    /// Unix update timestamp in milliseconds.
    pub updated_at: i64,
    /// Parsed outbound count.
    pub outbound_count: i32,
}

impl fmt::Debug for OutboundSubscription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutboundSubscription")
            .field("id", &self.id)
            .field("remark", &self.remark)
            .field("url", &"[REDACTED]")
            .field("enabled", &self.enabled)
            .field("allow_private", &self.allow_private)
            .field("allow_insecure", &self.allow_insecure)
            .field("tag_prefix", &self.tag_prefix)
            .field("update_interval", &self.update_interval)
            .field("priority", &self.priority)
            .field("prepend", &self.prepend)
            .field("last_updated", &self.last_updated)
            .field("last_error", &self.last_error)
            .field("last_fetched_outbounds", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("outbound_count", &self.outbound_count)
            .finish()
    }
}

/// Full replacement payload for a remote outbound subscription.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundSubscriptionInput {
    /// User-visible remark.
    pub remark: String,
    /// Remote subscription URL.
    pub url: String,
    /// Prefix applied to generated tags.
    pub tag_prefix: String,
    /// Whether it contributes outbounds.
    pub enabled: bool,
    /// Whether private destination addresses are allowed.
    pub allow_private: bool,
    /// Whether invalid TLS certificates are accepted.
    pub allow_insecure: bool,
    /// Whether remote outbounds precede manual outbounds.
    pub prepend: bool,
    /// Refresh interval in seconds.
    pub update_interval: i32,
}

impl OutboundSubscriptionInput {
    /// Creates enabled subscription input with the upstream 600-second interval.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            remark: String::new(),
            url: url.into(),
            tag_prefix: String::new(),
            enabled: true,
            allow_private: false,
            allow_insecure: false,
            prepend: false,
            update_interval: 600,
        }
    }
}

impl fmt::Debug for OutboundSubscriptionInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutboundSubscriptionInput")
            .field("remark", &self.remark)
            .field("url", &"[REDACTED]")
            .field("tag_prefix", &self.tag_prefix)
            .field("enabled", &self.enabled)
            .field("allow_private", &self.allow_private)
            .field("allow_insecure", &self.allow_insecure)
            .field("prepend", &self.prepend)
            .field("update_interval", &self.update_interval)
            .finish()
    }
}

/// Direction used to reorder an outbound subscription.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MoveDirection {
    /// Move toward lower priority numbers.
    Up,
    /// Move toward higher priority numbers.
    Down,
}

impl MoveDirection {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

/// Secret-bearing list of parsed Xray outbounds.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct OutboundDocuments(Vec<Value>);

impl OutboundDocuments {
    /// Borrows parsed outbound JSON documents.
    pub fn as_slice(&self) -> &[Value] {
        &self.0
    }
    /// Consumes the wrapper.
    pub fn into_inner(self) -> Vec<Value> {
        self.0
    }
}

impl fmt::Debug for OutboundDocuments {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OutboundDocuments")
            .field(&"[REDACTED]")
            .finish()
    }
}
