#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeMap;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::models::{
    ApiTokenMetadata, BalancerStatus, CreatedApiToken, EffectiveDefaults, FactoryDefaults,
    MoveDirection, OutboundDocuments, OutboundSubscription, OutboundSubscriptionInput,
    OutboundTestMode, OutboundTestResult, OutboundTraffic, PanelSettingsUpdate, PanelSettingsView,
    RouteTestRequest, RouteTestResult, SensitivePayload, SmtpTestResult, UserCredentialsUpdate,
    WarpRegistration, XraySettingsSnapshot,
};
use crate::{
    Client, Error, Result, XrayConfig, client::AuthenticationScope, response::ApiResponse,
};

const SETTINGS_ROOT: &str = "panel/api/setting";
const XRAY_ROOT: &str = "panel/api/xray";

/// Panel settings, credentials, notification tests, and API-token operations.
#[derive(Clone, Copy, Debug)]
pub struct SettingsApi<'client> {
    client: &'client Client,
}

impl<'client> SettingsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Returns the complete browser-safe panel settings view.
    pub async fn all(self) -> Result<PanelSettingsView> {
        self.post_object("all", None::<&()>).await
    }

    /// Returns host-derived effective defaults.
    pub async fn defaults(self) -> Result<EffectiveDefaults> {
        self.post_object("defaultSettings", None::<&()>).await
    }

    /// Returns the source defaults as setting-name/value pairs.
    ///
    /// This v3.6.0 source route is missing from upstream `OpenAPI`.
    pub async fn factory_defaults(self) -> Result<FactoryDefaults> {
        self.post_object("factoryDefaults", None::<&()>).await
    }

    /// Replaces all persisted panel settings.
    pub async fn update(self, update: &PanelSettingsUpdate) -> Result<()> {
        self.post_empty("update", Some(update)).await
    }

    /// Validates a Go regular expression with the panel runtime.
    ///
    /// This v3.6.0 source route is missing from upstream `OpenAPI`.
    pub async fn validate_regex(self, regex: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Body<'a> {
            regex: &'a str,
        }
        self.post_empty("validateRegex", Some(&Body { regex }))
            .await
    }

    /// Replaces the currently authenticated panel user's credentials.
    pub async fn update_user(self, update: &UserCredentialsUpdate) -> Result<()> {
        self.post_empty("updateUser", Some(update)).await
    }

    /// Schedules a panel process restart after the upstream three-second delay.
    pub async fn restart_panel(self) -> Result<()> {
        self.post_empty("restartPanel", None::<&()>).await
    }

    /// Returns the panel's default Xray template.
    pub async fn default_xray_config(self) -> Result<XrayConfig> {
        self.get_object("getDefaultJsonConfig").await
    }

    /// Lists API-token metadata. Plaintext tokens are never included.
    pub async fn api_tokens(self) -> Result<Vec<ApiTokenMetadata>> {
        self.get_object("apiTokens").await
    }

    /// Creates an API token and returns its plaintext exactly once.
    pub async fn create_api_token(self, name: &str) -> Result<CreatedApiToken> {
        #[derive(Serialize)]
        struct Form<'a> {
            name: &'a str,
        }
        self.post_form_object("apiTokens/create", &Form { name })
            .await
    }

    /// Permanently deletes an API token.
    pub async fn delete_api_token(self, id: i64) -> Result<()> {
        self.post_empty::<()>(&format!("apiTokens/delete/{id}"), None)
            .await
    }

    /// Enables or disables an API token.
    pub async fn set_api_token_enabled(self, id: i64, enabled: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            enabled: bool,
        }
        self.post_empty(
            &format!("apiTokens/setEnabled/{id}"),
            Some(&Body { enabled }),
        )
        .await
    }

    /// Tests the currently saved SMTP configuration.
    ///
    /// Unlike normal panel envelopes, an SMTP connectivity failure is a typed
    /// successful HTTP response with `success: false` and a diagnostic stage.
    pub async fn test_smtp(self) -> Result<SmtpTestResult> {
        let path = format!("{SETTINGS_ROOT}/testSmtp");
        let result = self
            .client
            .execute_response::<SmtpTestResult, ()>(
                Method::POST,
                &path,
                None,
                AuthenticationScope::PanelApi,
            )
            .await?;
        if !result.success && result.stage.is_empty() {
            return Err(Error::Api {
                method: Method::POST,
                url: Box::new(self.client.endpoint(&path)?),
                message: if result.message.is_empty() {
                    "SMTP test is unavailable".to_owned()
                } else {
                    result.message
                },
            });
        }
        Ok(result)
    }

    /// Sends a Telegram test notification using the saved bot configuration.
    pub async fn test_telegram(self) -> Result<()> {
        self.post_empty("testTgBot", None::<&()>).await
    }

    async fn get_object<T>(self, suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = format!("{SETTINGS_ROOT}/{suffix}");
        let envelope = self
            .client
            .execute::<T, ()>(Method::GET, &path, None, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, Method::GET, &path, envelope)
    }

    async fn post_object<T, B>(self, suffix: &str, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{SETTINGS_ROOT}/{suffix}");
        let envelope = self
            .client
            .execute(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, Method::POST, &path, envelope)
    }

    async fn post_form_object<T, B>(self, suffix: &str, form: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{SETTINGS_ROOT}/{suffix}");
        let envelope = self
            .client
            .execute_form(Method::POST, &path, form, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, Method::POST, &path, envelope)
    }

    async fn post_empty<B>(self, suffix: &str, body: Option<&B>) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("{SETTINGS_ROOT}/{suffix}");
        self.client
            .execute::<Value, B>(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }
}

/// Xray template, integration, routing-test, and outbound-subscription operations.
#[derive(Clone, Copy, Debug)]
pub struct XraySettingsApi<'client> {
    client: &'client Client,
}

impl<'client> XraySettingsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Returns the default Xray template.
    pub async fn default_config(self) -> Result<XrayConfig> {
        self.get_object("getDefaultJsonConfig").await
    }

    /// Returns cumulative traffic for every outbound.
    pub async fn outbounds_traffic(self) -> Result<Vec<OutboundTraffic>> {
        self.get_object("getOutboundsTraffic").await
    }

    /// Returns the most recent Xray startup or reload result.
    pub async fn xray_result(self) -> Result<String> {
        self.get_object("getXrayResult").await
    }

    /// Returns and decodes the panel's nested-string Xray settings response.
    pub async fn settings(self) -> Result<XraySettingsSnapshot> {
        let path = format!("{XRAY_ROOT}/");
        let envelope = self
            .client
            .execute::<String, ()>(Method::POST, &path, None, AuthenticationScope::PanelApi)
            .await?;
        let raw = required_object(self.client, Method::POST, &path, envelope)?;
        let url = self.client.endpoint(&path)?;
        serde_json::from_str(&raw).map_err(|source| Error::Decode {
            method: Method::POST,
            url: Box::new(url),
            source,
        })
    }

    /// Replaces the Xray template and outbound-test URL, serializing JSON automatically.
    pub async fn update(self, config: &XrayConfig, outbound_test_url: &str) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            xray_setting: String,
            outbound_test_url: &'a str,
        }
        let xray_setting = serde_json::to_string(config).map_err(|source| Error::Encode {
            operation: "update Xray settings",
            source,
        })?;
        self.post_form_empty(
            "update",
            &Form {
                xray_setting,
                outbound_test_url,
            },
        )
        .await
    }

    /// Returns stored WARP account data.
    pub async fn warp_data(self) -> Result<SensitivePayload> {
        self.integration("warp", "data", None::<&()>).await
    }
    /// Deletes stored WARP account data.
    pub async fn delete_warp(self) -> Result<()> {
        self.integration_empty("warp", "del", None::<&()>).await
    }
    /// Returns generated WARP Xray configuration.
    pub async fn warp_config(self) -> Result<SensitivePayload> {
        self.integration("warp", "config", None::<&()>).await
    }
    /// Registers WARP using optional existing key material.
    pub async fn register_warp(self, registration: &WarpRegistration) -> Result<SensitivePayload> {
        self.integration("warp", "reg", Some(registration)).await
    }
    /// Rotates the WARP address immediately.
    pub async fn change_warp_ip(self) -> Result<SensitivePayload> {
        self.integration("warp", "changeIp", None::<&()>).await
    }
    /// Applies a WARP license string.
    pub async fn set_warp_license(self, license: &str) -> Result<SensitivePayload> {
        #[derive(Serialize)]
        struct Form<'a> {
            license: &'a str,
        }
        self.integration("warp", "license", Some(&Form { license }))
            .await
    }
    /// Sets the automatic WARP rotation interval.
    pub async fn set_warp_update_interval(self, interval: u32) -> Result<()> {
        #[derive(Serialize)]
        struct Form {
            interval: u32,
        }
        self.integration_empty("warp", "interval", Some(&Form { interval }))
            .await
    }

    /// Returns `NordVPN` countries.
    pub async fn nord_countries(self) -> Result<SensitivePayload> {
        self.integration("nord", "countries", None::<&()>).await
    }
    /// Returns `NordVPN` servers for a country identifier.
    pub async fn nord_servers(self, country_id: &str) -> Result<SensitivePayload> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            country_id: &'a str,
        }
        self.integration("nord", "servers", Some(&Form { country_id }))
            .await
    }
    /// Exchanges a `NordVPN` access token for credentials.
    pub async fn register_nord(self, token: &str) -> Result<SensitivePayload> {
        #[derive(Serialize)]
        struct Form<'a> {
            token: &'a str,
        }
        self.integration("nord", "reg", Some(&Form { token })).await
    }
    /// Stores a `NordVPN` private key.
    pub async fn set_nord_key(self, key: &str) -> Result<SensitivePayload> {
        #[derive(Serialize)]
        struct Form<'a> {
            key: &'a str,
        }
        self.integration("nord", "setKey", Some(&Form { key }))
            .await
    }
    /// Returns stored `NordVPN` data.
    pub async fn nord_data(self) -> Result<SensitivePayload> {
        self.integration("nord", "data", None::<&()>).await
    }
    /// Deletes stored `NordVPN` data.
    pub async fn delete_nord(self) -> Result<()> {
        self.integration_empty("nord", "del", None::<&()>).await
    }

    /// Resets traffic counters for one outbound tag.
    pub async fn reset_outbound_traffic(self, tag: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Form<'a> {
            tag: &'a str,
        }
        self.post_form_empty("resetOutboundsTraffic", &Form { tag })
            .await
    }

    /// Tests one outbound, automatically encoding nested Xray JSON.
    pub async fn test_outbound(
        self,
        outbound: &Value,
        all_outbounds: Option<&[Value]>,
        mode: OutboundTestMode,
    ) -> Result<OutboundTestResult> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            outbound: &'a str,
            all_outbounds: &'a str,
            mode: &'a str,
        }

        let outbound = encode_json("test outbound", outbound)?;
        let all_outbounds = encode_optional_json("test outbound dependencies", all_outbounds)?;
        self.post_form_object(
            "testOutbound",
            &Form {
                outbound: &outbound,
                all_outbounds: &all_outbounds,
                mode: mode.as_str(),
            },
        )
        .await
    }

    /// Tests up to 50 outbounds in one shared Xray instance.
    pub async fn test_outbounds(
        self,
        outbounds: &[Value],
        all_outbounds: Option<&[Value]>,
        mode: OutboundTestMode,
    ) -> Result<Vec<OutboundTestResult>> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            outbounds: &'a str,
            all_outbounds: &'a str,
            mode: &'a str,
        }

        let outbounds = encode_json("test outbounds", outbounds)?;
        let all_outbounds = encode_optional_json("test outbound dependencies", all_outbounds)?;
        self.post_form_object(
            "testOutbounds",
            &Form {
                outbounds: &outbounds,
                all_outbounds: &all_outbounds,
                mode: mode.as_str(),
            },
        )
        .await
    }

    /// Returns live status for comma-separated balancer tags.
    pub async fn balancer_status(
        self,
        tags: &[impl AsRef<str>],
    ) -> Result<BTreeMap<String, BalancerStatus>> {
        #[derive(Serialize)]
        struct Form {
            tags: String,
        }
        let tags = tags.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(",");
        self.post_form_object("balancerStatus", &Form { tags })
            .await
    }

    /// Forces a balancer target, or clears the override when `target` is empty.
    pub async fn set_balancer_override(self, tag: &str, target: &str) -> Result<()> {
        #[derive(Serialize)]
        struct Form<'a> {
            tag: &'a str,
            target: &'a str,
        }
        self.post_form_empty("balancerOverride", &Form { tag, target })
            .await
    }

    /// Asks the running Xray router how it would route a synthetic connection.
    pub async fn test_route(self, request: &RouteTestRequest) -> Result<RouteTestResult> {
        self.post_form_object("routeTest", request).await
    }

    /// Lists stored remote outbound subscriptions.
    pub async fn outbound_subscriptions(self) -> Result<Vec<OutboundSubscription>> {
        self.get_object("outbound-subs").await
    }
    /// Creates a remote outbound subscription.
    pub async fn create_outbound_subscription(
        self,
        input: &OutboundSubscriptionInput,
    ) -> Result<OutboundSubscription> {
        self.post_form_object("outbound-subs", input).await
    }
    /// Refreshes and returns parsed outbounds for a subscription.
    pub async fn refresh_outbound_subscription(self, id: i64) -> Result<OutboundDocuments> {
        self.post_form_object(&format!("outbound-subs/{id}/refresh"), &())
            .await
    }
    /// Moves a subscription up or down in merge order.
    pub async fn move_outbound_subscription(self, id: i64, direction: MoveDirection) -> Result<()> {
        #[derive(Serialize)]
        struct Form<'a> {
            dir: &'a str,
        }
        self.post_form_empty(
            &format!("outbound-subs/{id}/move"),
            &Form {
                dir: direction.as_str(),
            },
        )
        .await
    }
    /// Replaces a stored outbound subscription.
    pub async fn update_outbound_subscription(
        self,
        id: i64,
        input: &OutboundSubscriptionInput,
    ) -> Result<()> {
        self.post_form_empty(&format!("outbound-subs/{id}"), input)
            .await
    }
    /// Deletes a subscription with HTTP DELETE.
    pub async fn delete_outbound_subscription(self, id: i64) -> Result<()> {
        let path = format!("{XRAY_ROOT}/outbound-subs/{id}");
        self.client
            .execute::<Value, ()>(Method::DELETE, &path, None, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }
    /// Deletes a subscription through the POST compatibility alias.
    pub async fn delete_outbound_subscription_via_post(self, id: i64) -> Result<()> {
        self.post_form_empty(&format!("outbound-subs/{id}/del"), &())
            .await
    }
    /// Fetches and parses a remote subscription without retaining it.
    pub async fn parse_outbound_subscription(
        self,
        url: &str,
        allow_private: bool,
        allow_insecure: bool,
    ) -> Result<OutboundDocuments> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            url: &'a str,
            allow_private: bool,
            allow_insecure: bool,
        }
        self.post_form_object(
            "outbound-subs/parse",
            &Form {
                url,
                allow_private,
                allow_insecure,
            },
        )
        .await
    }

    async fn integration<T, B>(self, family: &str, action: &str, form: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{XRAY_ROOT}/{family}/{}", segment(action));
        let envelope = match form {
            Some(form) => {
                self.client
                    .execute_form(Method::POST, &path, form, AuthenticationScope::PanelApi)
                    .await?
            }
            None => {
                self.client
                    .execute::<T, ()>(Method::POST, &path, None, AuthenticationScope::PanelApi)
                    .await?
            }
        };
        required_object(self.client, Method::POST, &path, envelope)
    }

    async fn integration_empty<B>(self, family: &str, action: &str, form: Option<&B>) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let _: SensitivePayload = self.integration(family, action, form).await?;
        Ok(())
    }

    async fn get_object<T>(self, suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = format!("{XRAY_ROOT}/{suffix}");
        let envelope = self
            .client
            .execute::<T, ()>(Method::GET, &path, None, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, Method::GET, &path, envelope)
    }

    async fn post_form_object<T, B>(self, suffix: &str, form: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{XRAY_ROOT}/{suffix}");
        let envelope = self
            .client
            .execute_form(Method::POST, &path, form, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, Method::POST, &path, envelope)
    }

    async fn post_form_empty<B>(self, suffix: &str, form: &B) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("{XRAY_ROOT}/{suffix}");
        self.client
            .execute_form::<Value, _>(Method::POST, &path, form, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }
}

fn required_object<T>(
    client: &Client,
    method: Method,
    path: &str,
    envelope: ApiResponse<T>,
) -> Result<T> {
    let url = client.endpoint(path)?;
    envelope.obj.ok_or_else(|| Error::MissingObject {
        method,
        url: Box::new(url),
    })
}

fn encode_json<T: Serialize + ?Sized>(operation: &'static str, value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|source| Error::Encode { operation, source })
}

fn encode_optional_json<T: Serialize + ?Sized>(
    operation: &'static str,
    value: Option<&T>,
) -> Result<String> {
    value.map_or_else(|| Ok(String::new()), |value| encode_json(operation, value))
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}
