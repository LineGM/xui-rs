use std::{fmt, string::FromUtf8Error};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::header::{self, HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Device identity sent to subscription endpoints when HWID limiting is enabled.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SubscriptionDevice {
    hwid: String,
    /// Subscription application's user agent.
    pub user_agent: String,
    /// Operating-system name sent as `X-Device-OS`.
    pub device_os: String,
    /// Operating-system version sent as `X-Ver-OS`.
    pub os_version: String,
    /// Device model sent as `X-Device-Model`.
    pub device_model: String,
}

impl SubscriptionDevice {
    /// Creates a device identity. 3x-ui requires an HWID of at least six bytes.
    pub fn new(hwid: impl Into<String>, user_agent: impl Into<String>) -> Self {
        Self {
            hwid: hwid.into(),
            user_agent: user_agent.into(),
            ..Self::default()
        }
    }

    /// Borrows the raw HWID. Avoid logging or persisting this value unnecessarily.
    pub fn hwid(&self) -> &str {
        &self.hwid
    }
}

impl fmt::Debug for SubscriptionDevice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionDevice")
            .field("hwid", &"[REDACTED]")
            .field("user_agent", &self.user_agent)
            .field("device_os", &self.device_os)
            .field("os_version", &self.os_version)
            .field("device_model", &self.device_model)
            .finish()
    }
}

/// Failure while decoding an encrypted raw subscription body.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SubscriptionDecodeError {
    /// The body is not valid standard base64.
    #[error("subscription body is not valid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    /// The decoded body is not valid UTF-8.
    #[error("decoded subscription body is not valid UTF-8: {0}")]
    Utf8(#[from] FromUtf8Error),
}

/// Secret-bearing text returned by raw, Clash, or HTML subscription views.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SubscriptionDocument(String);

impl SubscriptionDocument {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    /// Borrows the exact response body.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the exact response body.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Decodes a raw subscription body when `subEncrypt` is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error when the body is not standard base64 or its decoded
    /// bytes are not UTF-8.
    pub fn decode_base64(&self) -> Result<Self, SubscriptionDecodeError> {
        let bytes = STANDARD.decode(self.0.trim())?;
        String::from_utf8(bytes).map(Self).map_err(Into::into)
    }

    /// Iterates over non-empty lines in a plaintext or decoded document.
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.0
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
    }
}

impl fmt::Debug for SubscriptionDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SubscriptionDocument")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// Secret-bearing Xray JSON subscription document.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SubscriptionJson(Value);

impl SubscriptionJson {
    pub(crate) const fn new(value: Value) -> Self {
        Self(value)
    }

    /// Borrows the parsed JSON configuration.
    pub const fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consumes the wrapper and returns the parsed JSON configuration.
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl fmt::Debug for SubscriptionJson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SubscriptionJson")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// Parsed traffic and expiry values from `Subscription-Userinfo`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SubscriptionTraffic {
    /// Uploaded bytes.
    pub upload: i64,
    /// Downloaded bytes.
    pub download: i64,
    /// Total traffic allowance in bytes, or zero when unlimited.
    pub total: i64,
    /// Expiration Unix timestamp in seconds, or zero when unlimited.
    pub expire: i64,
}

/// Common response headers emitted by all three subscription formats.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SubscriptionMetadata {
    /// Parsed traffic/expiry header, when present and valid.
    pub traffic: Option<SubscriptionTraffic>,
    /// Decoded profile title.
    pub profile_title: Option<String>,
    /// Suggested polling interval in minutes.
    pub update_interval_minutes: Option<u32>,
    /// Configured support URL.
    pub support_url: Option<String>,
    /// Decoded announcement.
    pub announcement: Option<String>,
    /// Whether compatible clients should enable routing.
    pub routing_enabled: bool,
    /// Whether clients should hide editable settings.
    pub hide_settings: bool,
    /// Response MIME type.
    pub content_type: Option<String>,
    /// Download/profile filename header.
    pub content_disposition: Option<String>,
    /// Whether HWID enforcement is enabled for this subscription.
    pub hwid_active: bool,
    /// Whether the client omitted or sent an unsupported HWID.
    pub hwid_not_supported: bool,
    /// Whether all configured HWID slots are currently occupied.
    pub hwid_limit_reached: bool,
    /// Whether this request was rejected because no device slot was available.
    pub hwid_max_devices_reached: bool,
    profile_web_page_url: Option<String>,
    routing_rules: Option<String>,
}

impl SubscriptionMetadata {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            traffic: header_text(headers, "subscription-userinfo")
                .and_then(parse_subscription_userinfo),
            profile_title: header_text(headers, "profile-title").and_then(decode_text_header),
            update_interval_minutes: header_text(headers, "profile-update-interval")
                .and_then(|value| value.parse().ok()),
            support_url: header_text(headers, "support-url").map(str::to_owned),
            announcement: header_text(headers, "announce").and_then(decode_text_header),
            routing_enabled: header_text(headers, "routing-enable") == Some("true"),
            hide_settings: header_text(headers, "hide-settings") == Some("1"),
            content_type: headers
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            content_disposition: headers
                .get(header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            hwid_active: header_text(headers, "x-hwid-active") == Some("true"),
            hwid_not_supported: header_text(headers, "x-hwid-not-supported") == Some("true"),
            hwid_limit_reached: header_text(headers, "x-hwid-limit") == Some("true"),
            hwid_max_devices_reached: header_text(headers, "x-hwid-max-devices-reached")
                == Some("true"),
            profile_web_page_url: header_text(headers, "profile-web-page-url").map(str::to_owned),
            routing_rules: header_text(headers, "routing").map(str::to_owned),
        }
    }

    /// Returns the profile web-page URL, which normally embeds the secret
    /// subscription identifier.
    pub fn profile_web_page_url(&self) -> Option<&str> {
        self.profile_web_page_url.as_deref()
    }

    /// Returns routing rules supplied to compatible subscription clients.
    pub fn routing_rules(&self) -> Option<&str> {
        self.routing_rules.as_deref()
    }
}

impl fmt::Debug for SubscriptionMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionMetadata")
            .field("traffic", &self.traffic)
            .field("profile_title", &self.profile_title)
            .field("update_interval_minutes", &self.update_interval_minutes)
            .field("support_url", &self.support_url)
            .field("announcement", &self.announcement)
            .field("routing_enabled", &self.routing_enabled)
            .field("hide_settings", &self.hide_settings)
            .field("content_type", &self.content_type)
            .field("content_disposition", &self.content_disposition)
            .field("hwid_active", &self.hwid_active)
            .field("hwid_not_supported", &self.hwid_not_supported)
            .field("hwid_limit_reached", &self.hwid_limit_reached)
            .field("hwid_max_devices_reached", &self.hwid_max_devices_reached)
            .field("profile_web_page_url", &"[REDACTED]")
            .field("routing_rules", &"[REDACTED]")
            .finish()
    }
}

/// Body plus typed metadata returned by a subscription endpoint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubscriptionResponse<T> {
    /// Secret-bearing body or parsed document.
    pub content: T,
    /// Traffic, profile, routing, and content headers.
    pub metadata: SubscriptionMetadata,
}

/// Typed `format=info` subscription status view.
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionInfo {
    /// Secret subscription identifier.
    #[serde(rename = "sId")]
    pub subscription_id: String,
    /// Whether the shared traffic record is enabled.
    pub enabled: bool,
    /// Whether any attached client is currently online.
    pub is_online: bool,
    /// Human-formatted downloaded traffic.
    pub download: String,
    /// Human-formatted uploaded traffic.
    pub upload: String,
    /// Human-formatted total allowance.
    pub total: String,
    /// Human-formatted used traffic.
    pub used: String,
    /// Human-formatted remaining traffic.
    pub remained: String,
    /// Expiration Unix timestamp in seconds.
    pub expire: i64,
    /// Last-online timestamp in milliseconds.
    pub last_online: i64,
    /// Downloaded bytes.
    pub download_byte: i64,
    /// Uploaded bytes.
    pub upload_byte: i64,
    /// Total allowance in bytes.
    pub total_byte: i64,
    /// Raw subscription URL.
    pub sub_url: String,
    /// JSON subscription URL, or empty when disabled.
    pub sub_json_url: String,
    /// Clash subscription URL, or empty when disabled.
    pub sub_clash_url: String,
    /// Profile title.
    pub sub_title: String,
    /// Support URL.
    pub sub_support_url: String,
    /// Distinct client emails represented by the subscription.
    pub emails: Vec<String>,
    /// Calendar identifier used by the information page.
    pub datepicker: String,
    /// Subscription announcement.
    pub announce: String,
}

impl fmt::Debug for SubscriptionInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionInfo")
            .field("subscription_id", &"[REDACTED]")
            .field("enabled", &self.enabled)
            .field("is_online", &self.is_online)
            .field("download", &self.download)
            .field("upload", &self.upload)
            .field("total", &self.total)
            .field("used", &self.used)
            .field("remained", &self.remained)
            .field("expire", &self.expire)
            .field("last_online", &self.last_online)
            .field("download_byte", &self.download_byte)
            .field("upload_byte", &self.upload_byte)
            .field("total_byte", &self.total_byte)
            .field("sub_url", &"[REDACTED]")
            .field("sub_json_url", &"[REDACTED]")
            .field("sub_clash_url", &"[REDACTED]")
            .field("sub_title", &self.sub_title)
            .field("sub_support_url", &self.sub_support_url)
            .field("emails", &"[REDACTED]")
            .field("datepicker", &self.datepicker)
            .field("announce", &self.announce)
            .finish()
    }
}

fn header_text<'headers>(headers: &'headers HeaderMap, name: &str) -> Option<&'headers str> {
    headers.get(name)?.to_str().ok()
}

fn parse_subscription_userinfo(value: &str) -> Option<SubscriptionTraffic> {
    let mut upload = None;
    let mut download = None;
    let mut total = None;
    let mut expire = None;
    for item in value.split(';') {
        let (key, value) = item.trim().split_once('=')?;
        let value = value.trim().parse().ok()?;
        match key.trim() {
            "upload" => upload = Some(value),
            "download" => download = Some(value),
            "total" => total = Some(value),
            "expire" => expire = Some(value),
            _ => {}
        }
    }
    Some(SubscriptionTraffic {
        upload: upload?,
        download: download?,
        total: total?,
        expire: expire?,
    })
}

fn decode_text_header(value: &str) -> Option<String> {
    let encoded = value.strip_prefix("base64:")?;
    let bytes = STANDARD.decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}
