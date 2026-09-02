use std::time::Duration;

use reqwest::{Method, header};
use serde_json::Value;
use url::Url;

use super::{
    SubscriptionDevice, SubscriptionDocument, SubscriptionInfo, SubscriptionJson,
    SubscriptionMetadata, SubscriptionResponse,
};
use crate::{
    Error, ProxyConfig, Result, SubscriptionSettings,
    transport::{self, ErrorUrlPolicy},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_USER_AGENT: &str = concat!("xui-rs/", env!("CARGO_PKG_VERSION"));

/// Default maximum public subscription response body size: 64 MiB.
pub const DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT: usize = 64 * 1024 * 1024;

/// Builder for a public standalone [`SubscriptionClient`].
#[must_use]
pub struct SubscriptionClientBuilder {
    base_url: Url,
    raw_path: String,
    json_path: String,
    clash_path: String,
    timeout: Duration,
    connect_timeout: Duration,
    accept_invalid_certs: bool,
    proxy: Option<ProxyConfig>,
    response_body_limit: usize,
    device: Option<SubscriptionDevice>,
}

impl SubscriptionClientBuilder {
    fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            base_url: normalize_server_url(base_url.as_ref())?,
            raw_path: "sub/".to_owned(),
            json_path: "json/".to_owned(),
            clash_path: "clash/".to_owned(),
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            accept_invalid_certs: false,
            proxy: None,
            response_body_limit: DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT,
            device: None,
        })
    }

    /// Sets the configured raw subscription path (default `/sub/`).
    pub fn raw_path(mut self, path: impl Into<String>) -> Self {
        self.raw_path = path.into();
        self
    }

    /// Sets the configured JSON subscription path (default `/json/`).
    pub fn json_path(mut self, path: impl Into<String>) -> Self {
        self.json_path = path.into();
        self
    }

    /// Sets the configured Clash subscription path (default `/clash/`).
    pub fn clash_path(mut self, path: impl Into<String>) -> Self {
        self.clash_path = path.into();
        self
    }

    /// Sets the total timeout for one subscription request.
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the timeout for establishing a connection.
    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Accepts invalid TLS certificates for this subscription server.
    ///
    /// This weakens transport security for credential-bearing documents and
    /// should only be used with an explicitly trusted self-signed deployment.
    pub const fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    /// Routes subscription HTTP requests through an explicit proxy.
    ///
    /// Environment proxy variables are never consulted.
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Parses and configures a credential-free proxy URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] for an invalid proxy URL.
    pub fn proxy_url(self, proxy_url: impl AsRef<str>) -> Result<Self> {
        Ok(self.proxy(ProxyConfig::new(proxy_url)?))
    }

    /// Removes a previously configured explicit proxy.
    pub fn no_proxy(mut self) -> Self {
        self.proxy = None;
        self
    }

    /// Sets the maximum in-memory size of a subscription response body.
    ///
    /// Both declared `Content-Length` and actually received chunked or
    /// decompressed bytes are checked.
    pub const fn response_body_limit(mut self, limit: usize) -> Self {
        self.response_body_limit = limit;
        self
    }

    /// Sends subscription-device headers used by 3x-ui's optional HWID gate.
    pub fn device(mut self, device: SubscriptionDevice) -> Self {
        self.device = Some(device);
        self
    }

    /// Removes previously configured subscription-device metadata.
    pub fn no_device(mut self) -> Self {
        self.device = None;
        self
    }

    /// Builds the standalone client.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured path or HTTP-client option is
    /// invalid.
    pub fn build(self) -> Result<SubscriptionClient> {
        let raw_prefix = prefix_from_path(&self.base_url, &self.raw_path, "raw")?;
        let json_prefix = prefix_from_path(&self.base_url, &self.json_path, "JSON")?;
        let clash_prefix = prefix_from_path(&self.base_url, &self.clash_path, "Clash")?;
        SubscriptionClient::from_parts(
            raw_prefix,
            json_prefix,
            clash_prefix,
            SubscriptionTransportConfig {
                timeout: self.timeout,
                connect_timeout: self.connect_timeout,
                accept_invalid_certs: self.accept_invalid_certs,
                proxy: self.proxy.as_ref(),
                response_body_limit: self.response_body_limit,
                device: self.device.as_ref(),
            },
        )
    }
}

impl std::fmt::Debug for SubscriptionClientBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionClientBuilder")
            .field("server_origin", &origin(&self.base_url))
            .field("raw_path", &"[REDACTED]")
            .field("json_path", &"[REDACTED]")
            .field("clash_path", &"[REDACTED]")
            .field("timeout", &self.timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .field("proxy", &self.proxy.as_ref().map(ProxyConfig::scheme))
            .field("response_body_limit", &self.response_body_limit)
            .field("device", &self.device)
            .finish()
    }
}

/// Public client for the separate 3x-ui subscription HTTP/HTTPS server.
///
/// This client intentionally has no panel authentication or cookie state. The
/// subscription identifier embedded into every request path is redacted from
/// SDK errors.
#[derive(Clone)]
pub struct SubscriptionClient {
    http: reqwest::Client,
    raw_prefix: Url,
    json_prefix: Url,
    clash_prefix: Url,
    response_body_limit: usize,
}

impl SubscriptionClient {
    /// Creates a client using the default `/sub/`, `/json/`, and `/clash/`
    /// paths below `server_url`.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL or HTTP-client configuration is invalid.
    pub fn new(server_url: impl AsRef<str>) -> Result<Self> {
        Self::builder(server_url)?.build()
    }

    /// Starts configuring a standalone subscription client.
    ///
    /// # Errors
    ///
    /// Returns an error when `server_url` is not an absolute HTTP(S) URL.
    pub fn builder(server_url: impl AsRef<str>) -> Result<SubscriptionClientBuilder> {
        SubscriptionClientBuilder::new(server_url)
    }

    /// Creates a client from the absolute public URIs and fallback paths in a
    /// panel settings snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when `subURI` is not an absolute HTTP(S) prefix or any
    /// configured URI/HTTP option is invalid.
    pub fn from_settings(settings: &SubscriptionSettings) -> Result<Self> {
        let raw_prefix = normalize_prefix_uri(&settings.sub_uri, "subURI")?;
        let server_url = origin_url(&raw_prefix);
        let json_prefix = if settings.sub_json_uri.trim().is_empty() {
            prefix_from_path(&server_url, &settings.sub_json_path, "JSON")?
        } else {
            normalize_prefix_uri(&settings.sub_json_uri, "subJsonURI")?
        };
        let clash_prefix = if settings.sub_clash_uri.trim().is_empty() {
            prefix_from_path(&server_url, &settings.sub_clash_path, "Clash")?
        } else {
            normalize_prefix_uri(&settings.sub_clash_uri, "subClashURI")?
        };
        Self::from_parts(
            raw_prefix,
            json_prefix,
            clash_prefix,
            SubscriptionTransportConfig::default(),
        )
    }

    fn from_parts(
        raw_prefix: Url,
        json_prefix: Url,
        clash_prefix: Url,
        transport: SubscriptionTransportConfig<'_>,
    ) -> Result<Self> {
        let mut default_headers = header::HeaderMap::new();
        let user_agent = if let Some(device) = transport.device {
            if device.hwid().len() < 6 {
                return Err(Error::Configuration(
                    "subscription device HWID must contain at least 6 bytes".to_owned(),
                ));
            }
            insert_device_header(&mut default_headers, "x-hwid", device.hwid(), "HWID")?;
            insert_device_header(
                &mut default_headers,
                "x-device-os",
                &device.device_os,
                "device OS",
            )?;
            insert_device_header(
                &mut default_headers,
                "x-ver-os",
                &device.os_version,
                "OS version",
            )?;
            insert_device_header(
                &mut default_headers,
                "x-device-model",
                &device.device_model,
                "device model",
            )?;
            if device.user_agent.is_empty() {
                DEFAULT_USER_AGENT
            } else {
                device.user_agent.as_str()
            }
        } else {
            DEFAULT_USER_AGENT
        };
        let mut http_builder = reqwest::Client::builder()
            .no_proxy()
            .default_headers(default_headers)
            .timeout(transport.timeout)
            .connect_timeout(transport.connect_timeout)
            .user_agent(user_agent)
            .danger_accept_invalid_certs(transport.accept_invalid_certs);
        if let Some(proxy) = transport.proxy {
            http_builder = http_builder.proxy(proxy.reqwest_proxy()?);
        }
        let http = http_builder
            .build()
            .map_err(|error| Error::Configuration(error.to_string()))?;
        Ok(Self {
            http,
            raw_prefix,
            json_prefix,
            clash_prefix,
            response_body_limit: transport.response_body_limit,
        })
    }

    /// Returns the configured maximum subscription response body size.
    pub const fn response_body_limit(&self) -> usize {
        self.response_body_limit
    }

    /// Fetches the raw subscription body.
    ///
    /// The body is base64 when the panel's `subEncrypt` setting is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error for transport or unsuccessful HTTP status.
    pub async fn raw(
        &self,
        subscription_id: &str,
    ) -> Result<SubscriptionResponse<SubscriptionDocument>> {
        self.text(
            Method::GET,
            Format::Raw,
            subscription_id,
            None,
            "text/plain",
        )
        .await
    }

    /// Fetches the browser HTML information page.
    ///
    /// # Errors
    ///
    /// Returns an error for transport, unsuccessful HTTP status, or invalid
    /// UTF-8.
    pub async fn html(
        &self,
        subscription_id: &str,
    ) -> Result<SubscriptionResponse<SubscriptionDocument>> {
        self.text(Method::GET, Format::Raw, subscription_id, None, "text/html")
            .await
    }

    /// Fetches the typed `format=info` status view without imported link data.
    ///
    /// # Errors
    ///
    /// Returns an error for transport, unsuccessful HTTP status, or invalid
    /// JSON.
    pub async fn info(&self, subscription_id: &str) -> Result<SubscriptionInfo> {
        Ok(self.info_with_metadata(subscription_id).await?.content)
    }

    /// Fetches the typed information view together with common and HWID headers.
    ///
    /// Prefer this method when device-limit state is relevant to the caller.
    ///
    /// # Errors
    ///
    /// Returns an error for transport, unsuccessful HTTP status, or invalid JSON.
    pub async fn info_with_metadata(
        &self,
        subscription_id: &str,
    ) -> Result<SubscriptionResponse<SubscriptionInfo>> {
        let (headers, bytes, method, redacted_url) = self
            .send(
                Method::GET,
                Format::Raw,
                subscription_id,
                Some(("format", "info")),
                "application/json",
            )
            .await?;
        let content = serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
            method,
            url: Box::new(redacted_url),
            source,
        })?;
        Ok(SubscriptionResponse {
            content,
            metadata: SubscriptionMetadata::from_headers(&headers),
        })
    }

    /// Fetches and parses the Xray JSON subscription document.
    ///
    /// The raw-download view is requested to avoid browser content negotiation.
    ///
    /// # Errors
    ///
    /// Returns an error for transport, unsuccessful HTTP status, or invalid
    /// JSON.
    pub async fn json(
        &self,
        subscription_id: &str,
    ) -> Result<SubscriptionResponse<SubscriptionJson>> {
        let (headers, bytes, method, redacted_url) = self
            .send(
                Method::GET,
                Format::Json,
                subscription_id,
                Some(("view", "raw")),
                "application/json",
            )
            .await?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
            method,
            url: Box::new(redacted_url),
            source,
        })?;
        Ok(SubscriptionResponse {
            content: SubscriptionJson::new(value),
            metadata: SubscriptionMetadata::from_headers(&headers),
        })
    }

    /// Fetches the Clash/Mihomo YAML subscription document.
    ///
    /// The raw-download view is requested to avoid browser content negotiation.
    ///
    /// # Errors
    ///
    /// Returns an error for transport, unsuccessful HTTP status, or invalid
    /// UTF-8.
    pub async fn clash(
        &self,
        subscription_id: &str,
    ) -> Result<SubscriptionResponse<SubscriptionDocument>> {
        self.text(
            Method::GET,
            Format::Clash,
            subscription_id,
            Some(("view", "raw")),
            "application/yaml",
        )
        .await
    }

    /// Executes `HEAD` against the raw subscription route.
    ///
    /// # Errors
    ///
    /// Returns an error for transport or unsuccessful HTTP status.
    pub async fn raw_metadata(&self, subscription_id: &str) -> Result<SubscriptionMetadata> {
        self.head(Format::Raw, subscription_id).await
    }

    /// Executes `HEAD` against the JSON subscription route.
    ///
    /// # Errors
    ///
    /// Returns an error for transport or unsuccessful HTTP status.
    pub async fn json_metadata(&self, subscription_id: &str) -> Result<SubscriptionMetadata> {
        self.head(Format::Json, subscription_id).await
    }

    /// Executes `HEAD` against the Clash subscription route.
    ///
    /// # Errors
    ///
    /// Returns an error for transport or unsuccessful HTTP status.
    pub async fn clash_metadata(&self, subscription_id: &str) -> Result<SubscriptionMetadata> {
        self.head(Format::Clash, subscription_id).await
    }

    async fn head(&self, format: Format, subscription_id: &str) -> Result<SubscriptionMetadata> {
        let (headers, _, _, _) = self
            .send(Method::HEAD, format, subscription_id, None, "*/*")
            .await?;
        Ok(SubscriptionMetadata::from_headers(&headers))
    }

    async fn text(
        &self,
        method: Method,
        format: Format,
        subscription_id: &str,
        query: Option<(&str, &str)>,
        accept: &str,
    ) -> Result<SubscriptionResponse<SubscriptionDocument>> {
        let (headers, bytes, method, redacted_url) = self
            .send(method, format, subscription_id, query, accept)
            .await?;
        let text = String::from_utf8(bytes).map_err(|source| Error::Utf8 {
            method,
            url: Box::new(redacted_url),
            source,
        })?;
        Ok(SubscriptionResponse {
            content: SubscriptionDocument::new(text),
            metadata: SubscriptionMetadata::from_headers(&headers),
        })
    }

    async fn send(
        &self,
        method: Method,
        format: Format,
        subscription_id: &str,
        query: Option<(&str, &str)>,
        accept: &str,
    ) -> Result<(header::HeaderMap, Vec<u8>, Method, Url)> {
        let prefix = match format {
            Format::Raw => &self.raw_prefix,
            Format::Json => &self.json_prefix,
            Format::Clash => &self.clash_prefix,
        };
        let (mut url, mut redacted_url) = endpoint(prefix, subscription_id)?;
        if let Some((key, value)) = query {
            url.query_pairs_mut().append_pair(key, value);
            redacted_url.query_pairs_mut().append_pair(key, value);
        }
        let response = transport::send_request(
            self.http
                .request(method.clone(), url)
                .header(header::ACCEPT, accept),
            &method,
            &redacted_url,
            ErrorUrlPolicy::Redact,
        )
        .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                method,
                url: Box::new(redacted_url),
                status,
            });
        }
        let headers = response.headers().clone();
        let bytes = transport::read_response_body(
            response,
            &method,
            &redacted_url,
            self.response_body_limit,
            ErrorUrlPolicy::Redact,
        )
        .await?;
        Ok((headers, bytes, method, redacted_url))
    }
}

#[derive(Clone, Copy)]
struct SubscriptionTransportConfig<'proxy> {
    timeout: Duration,
    connect_timeout: Duration,
    accept_invalid_certs: bool,
    proxy: Option<&'proxy ProxyConfig>,
    response_body_limit: usize,
    device: Option<&'proxy SubscriptionDevice>,
}

impl Default for SubscriptionTransportConfig<'_> {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            accept_invalid_certs: false,
            proxy: None,
            response_body_limit: DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT,
            device: None,
        }
    }
}

fn insert_device_header(
    headers: &mut header::HeaderMap,
    name: &'static str,
    value: &str,
    label: &str,
) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let value = header::HeaderValue::from_str(value)
        .map_err(|error| Error::Configuration(format!("invalid {label} header: {error}")))?;
    headers.insert(header::HeaderName::from_static(name), value);
    Ok(())
}

impl std::fmt::Debug for SubscriptionClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionClient")
            .field("server_origin", &origin(&self.raw_prefix))
            .field("paths", &"[REDACTED]")
            .field("response_body_limit", &self.response_body_limit)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
enum Format {
    Raw,
    Json,
    Clash,
}

fn normalize_server_url(value: &str) -> Result<Url> {
    let mut url = parse_http_url(value, "subscription server URL")?;
    url.set_query(None);
    url.set_fragment(None);
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn normalize_prefix_uri(value: &str, field: &str) -> Result<Url> {
    if value.trim().is_empty() {
        return Err(Error::Configuration(format!("{field} must not be empty")));
    }
    let mut url = parse_http_url(value, field)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(Error::Configuration(format!(
            "{field} must not contain a query or fragment"
        )));
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn parse_http_url(value: &str, field: &str) -> Result<Url> {
    let url = Url::parse(value)
        .map_err(|error| Error::Configuration(format!("invalid {field}: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(Error::Configuration(format!(
            "{field} must be an absolute HTTP(S) URL"
        )));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Configuration(format!(
            "{field} must not contain credentials"
        )));
    }
    Ok(url)
}

fn prefix_from_path(base_url: &Url, path: &str, label: &str) -> Result<Url> {
    let path = path.trim();
    if path.is_empty() {
        return Err(Error::Configuration(format!(
            "{label} subscription path must not be empty"
        )));
    }
    if path.contains('?') || path.contains('#') {
        return Err(Error::Configuration(format!(
            "{label} subscription path must not contain a query or fragment"
        )));
    }
    let mut url = base_url
        .join(path.trim_start_matches('/'))
        .map_err(|error| Error::Configuration(format!("invalid {label} path: {error}")))?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn endpoint(prefix: &Url, subscription_id: &str) -> Result<(Url, Url)> {
    if subscription_id.is_empty() {
        return Err(Error::Configuration(
            "subscription ID must not be empty".to_owned(),
        ));
    }
    let mut url = prefix.clone();
    url.path_segments_mut()
        .map_err(|()| Error::Configuration("subscription URL cannot be a base".to_owned()))?
        .pop_if_empty()
        .push(subscription_id);
    let mut redacted = prefix.clone();
    redacted
        .path_segments_mut()
        .map_err(|()| Error::Configuration("subscription URL cannot be a base".to_owned()))?
        .pop_if_empty()
        .push("[REDACTED]");
    Ok((url, redacted))
}

fn origin_url(url: &Url) -> Url {
    let mut origin = url.clone();
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    origin
}

fn origin(url: &Url) -> String {
    let mut value = origin_url(url);
    value.set_path("");
    value.to_string().trim_end_matches('/').to_owned()
}
