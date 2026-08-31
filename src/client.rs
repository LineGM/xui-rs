//! Main panel client, authentication selection, and transport configuration.

use std::{sync::Arc, time::Duration};

use reqwest::{
    Method, RequestBuilder, StatusCode,
    cookie::{CookieStore, Jar},
    header,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use url::Url;

use crate::{
    AuthApi, Error, ProxyConfig, Result,
    response::ApiResponse,
    transport::{self, ErrorUrlPolicy},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_USER_AGENT: &str = concat!("xui-rs/", env!("CARGO_PKG_VERSION"));

/// Default maximum size of an ordinary panel API response body: 64 MiB.
pub const DEFAULT_API_RESPONSE_BODY_LIMIT: usize = 64 * 1024 * 1024;

/// Default maximum size of an explicit database or migration download: 512 MiB.
pub const DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT: usize = 512 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum AuthenticationScope {
    Session,
    #[allow(dead_code)]
    PanelApi,
}

/// Authentication mechanism used for protected panel endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthenticationKind {
    /// Signed session cookies obtained through `/login`.
    Session,
    /// An API token sent in the `Authorization` header.
    BearerToken,
}

/// Builder for [`Client`].
#[must_use]
pub struct ClientBuilder {
    base_url: Url,
    bearer_token: Option<SecretString>,
    timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
    accept_invalid_certs: bool,
    proxy: Option<ProxyConfig>,
    response_body_limit: usize,
    download_body_limit: usize,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClientBuilder")
            .field("server_origin", &origin(&self.base_url))
            .field(
                "authentication",
                &self
                    .bearer_token
                    .as_ref()
                    .map(|_| AuthenticationKind::BearerToken),
            )
            .field("timeout", &self.timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("user_agent", &self.user_agent)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .field("proxy", &self.proxy.as_ref().map(ProxyConfig::scheme))
            .field("response_body_limit", &self.response_body_limit)
            .field("download_body_limit", &self.download_body_limit)
            .finish()
    }
}

impl ClientBuilder {
    pub(crate) fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            base_url: normalize_base_url(base_url.as_ref())?,
            bearer_token: None,
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            accept_invalid_certs: false,
            proxy: None,
            response_body_limit: DEFAULT_API_RESPONSE_BODY_LIMIT,
            download_body_limit: DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT,
        })
    }

    /// Uses a 3x-ui API token for protected API endpoints.
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(SecretString::from(token.into()));
        self
    }

    /// Sets the total timeout for one HTTP request.
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the timeout for establishing a connection.
    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Overrides the user-agent sent by the SDK.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Accepts invalid TLS certificates.
    ///
    /// Use this only for an explicitly trusted self-signed deployment. It
    /// weakens transport security for every request made by this client.
    pub const fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    /// Routes panel HTTP requests and WebSocket connections through an
    /// explicit proxy.
    ///
    /// Environment proxy variables are never consulted. Use
    /// [`ProxyScheme::Socks5h`](crate::ProxyScheme::Socks5h) when target DNS
    /// resolution must happen on the proxy.
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Parses and configures a credential-free proxy URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] for an invalid proxy URL. Configure
    /// credentials with [`ProxyConfig::with_basic_auth`] and [`Self::proxy`].
    pub fn proxy_url(self, proxy_url: impl AsRef<str>) -> Result<Self> {
        Ok(self.proxy(ProxyConfig::new(proxy_url)?))
    }

    /// Removes a previously configured explicit proxy.
    pub fn no_proxy(mut self) -> Self {
        self.proxy = None;
        self
    }

    /// Sets the maximum in-memory size of an ordinary panel API response.
    ///
    /// Both declared `Content-Length` and actually received chunked or
    /// decompressed bytes are checked. Set a deliberately larger value for
    /// deployments whose client lists, logs, or runtime `OpenAPI` exceed the
    /// safe 64 MiB default.
    pub const fn response_body_limit(mut self, limit: usize) -> Self {
        self.response_body_limit = limit;
        self
    }

    /// Sets the maximum in-memory size of explicit database and migration
    /// downloads.
    ///
    /// This is separate from [`Self::response_body_limit`] because backups are
    /// expected to be substantially larger than structured API responses.
    pub const fn download_body_limit(mut self, limit: usize) -> Self {
        self.download_body_limit = limit;
        self
    }

    /// Builds the configured client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] when the HTTP client configuration is
    /// invalid.
    pub fn build(self) -> Result<Client> {
        if self
            .bearer_token
            .as_ref()
            .is_some_and(|token| token.expose_secret().trim().is_empty())
        {
            return Err(Error::Configuration(
                "bearer token must not be empty".to_owned(),
            ));
        }
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-requested-with",
            header::HeaderValue::from_static("XMLHttpRequest"),
        );

        let cookie_jar = Arc::new(Jar::default());
        let mut http_builder = reqwest::Client::builder()
            .no_proxy()
            .cookie_provider(Arc::clone(&cookie_jar))
            .default_headers(headers)
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .user_agent(self.user_agent.clone())
            .danger_accept_invalid_certs(self.accept_invalid_certs);
        if let Some(proxy) = &self.proxy {
            http_builder = http_builder.proxy(proxy.reqwest_proxy()?);
        }
        let http = http_builder
            .build()
            .map_err(|error| Error::Configuration(error.to_string()))?;

        Ok(Client {
            inner: Arc::new(Inner {
                http,
                base_url: self.base_url,
                bearer_token: self.bearer_token,
                cookie_jar,
                csrf_token: Mutex::new(None),
                connect_timeout: self.connect_timeout,
                user_agent: self.user_agent,
                accept_invalid_certs: self.accept_invalid_certs,
                proxy: self.proxy,
                response_body_limit: self.response_body_limit,
                download_body_limit: self.download_body_limit,
            }),
        })
    }
}

/// A cheap-to-clone, concurrency-safe client for a 3x-ui panel.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Client")
            .field("server_origin", &origin(&self.inner.base_url))
            .field("authentication", &self.authentication_kind())
            .field("proxy", &self.inner.proxy.as_ref().map(ProxyConfig::scheme))
            .field("response_body_limit", &self.inner.response_body_limit)
            .field("download_body_limit", &self.inner.download_body_limit)
            .finish_non_exhaustive()
    }
}

struct Inner {
    http: reqwest::Client,
    base_url: Url,
    bearer_token: Option<SecretString>,
    cookie_jar: Arc<Jar>,
    csrf_token: Mutex<Option<SecretString>>,
    connect_timeout: Duration,
    user_agent: String,
    accept_invalid_certs: bool,
    proxy: Option<ProxyConfig>,
    response_body_limit: usize,
    download_body_limit: usize,
}

impl Client {
    /// Creates a cookie-session client with default settings.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] when the base URL or HTTP client
    /// configuration is invalid.
    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        Self::builder(base_url)?.build()
    }

    /// Starts configuring a client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] when `base_url` is not an HTTP(S) URL.
    pub fn builder(base_url: impl AsRef<str>) -> Result<ClientBuilder> {
        ClientBuilder::new(base_url)
    }

    /// Returns the normalized panel base URL.
    pub fn base_url(&self) -> &Url {
        &self.inner.base_url
    }

    /// Returns the authentication mechanism for protected panel endpoints.
    pub fn authentication_kind(&self) -> AuthenticationKind {
        if self.inner.bearer_token.is_some() {
            AuthenticationKind::BearerToken
        } else {
            AuthenticationKind::Session
        }
    }

    /// Returns the configured maximum ordinary API response body size.
    pub fn response_body_limit(&self) -> usize {
        self.inner.response_body_limit
    }

    /// Returns the configured maximum database/migration download size.
    pub fn download_body_limit(&self) -> usize {
        self.inner.download_body_limit
    }

    /// Accesses login, logout, 2FA, and CSRF operations.
    pub const fn auth(&self) -> AuthApi<'_> {
        AuthApi::new(self)
    }

    /// Accesses inbound management operations.
    pub const fn inbounds(&self) -> crate::InboundsApi<'_> {
        crate::InboundsApi::new(self)
    }

    /// Accesses client and client-group management operations.
    pub const fn clients(&self) -> crate::ClientsApi<'_> {
        crate::ClientsApi::new(self)
    }

    /// Accesses server status, Xray lifecycle, maintenance, and utility operations.
    pub const fn server(&self) -> crate::ServerApi<'_> {
        crate::ServerApi::new(self)
    }

    /// Accesses panel settings, credentials, notifications, and API tokens.
    pub const fn settings(&self) -> crate::SettingsApi<'_> {
        crate::SettingsApi::new(self)
    }

    /// Accesses Xray settings, integrations, tests, and outbound subscriptions.
    pub const fn xray_settings(&self) -> crate::XraySettingsApi<'_> {
        crate::XraySettingsApi::new(self)
    }

    /// Accesses per-inbound subscription host override operations.
    pub const fn hosts(&self) -> crate::HostsApi<'_> {
        crate::HostsApi::new(self)
    }

    /// Accesses remote node registration, health, discovery, and mTLS operations.
    pub const fn nodes(&self) -> crate::NodesApi<'_> {
        crate::NodesApi::new(self)
    }

    /// Accesses panel-wide metadata and backup operations.
    pub const fn panel(&self) -> crate::PanelApi<'_> {
        crate::PanelApi::new(self)
    }

    /// Accesses the authenticated real-time event stream.
    pub const fn events(&self) -> crate::EventsApi<'_> {
        crate::EventsApi::new(self)
    }

    pub(crate) fn endpoint(&self, path: &str) -> Result<Url> {
        self.inner
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| Error::Configuration(format!("invalid endpoint path: {error}")))
    }

    pub(crate) fn cookie_header(&self, url: &Url) -> Option<header::HeaderValue> {
        self.inner.cookie_jar.cookies(url)
    }

    pub(crate) fn connect_timeout(&self) -> Duration {
        self.inner.connect_timeout
    }

    pub(crate) fn user_agent(&self) -> &str {
        &self.inner.user_agent
    }

    pub(crate) fn accepts_invalid_certs(&self) -> bool {
        self.inner.accept_invalid_certs
    }

    pub(crate) fn proxy(&self) -> Option<&ProxyConfig> {
        self.inner.proxy.as_ref()
    }

    pub(crate) async fn execute<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        scope: AuthenticationScope,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.execute_with(method, path, scope, |request| match body {
            Some(body) => request.json(body),
            None => request,
        })
        .await
    }

    pub(crate) async fn execute_form<T, B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        scope: AuthenticationScope,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.execute_with(method, path, scope, |request| request.form(body))
            .await
    }

    pub(crate) async fn execute_response<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        scope: AuthenticationScope,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.execute_response_with(method, path, scope, |request| match body {
            Some(body) => request.json(body),
            None => request,
        })
        .await
    }

    pub(crate) async fn execute_empty<B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        scope: AuthenticationScope,
    ) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        self.execute_configured(method, path, scope, |request| match body {
            Some(body) => request.json(body),
            None => request,
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn execute_query<T, Q>(
        &self,
        method: Method,
        path: &str,
        query: &Q,
        scope: AuthenticationScope,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        self.execute_with(method, path, scope, |request| request.query(query))
            .await
    }

    pub(crate) async fn execute_query_json<T, Q, B>(
        &self,
        method: Method,
        path: &str,
        query: &Q,
        body: &B,
        scope: AuthenticationScope,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
    {
        self.execute_with(method, path, scope, |request| {
            request.query(query).json(body)
        })
        .await
    }

    pub(crate) async fn execute_multipart<T>(
        &self,
        method: Method,
        path: &str,
        field: &'static str,
        filename: &str,
        bytes: &[u8],
        scope: AuthenticationScope,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
    {
        self.execute_with(method, path, scope, |request| {
            let part = reqwest::multipart::Part::bytes(bytes.to_vec())
                .file_name(filename.to_owned())
                .mime_str("application/octet-stream")
                .expect("static MIME type is valid");
            request.multipart(reqwest::multipart::Form::new().part(field, part))
        })
        .await
    }

    pub(crate) async fn execute_bytes(
        &self,
        method: Method,
        path: &str,
        scope: AuthenticationScope,
    ) -> Result<(header::HeaderMap, Vec<u8>)> {
        let url = self.endpoint(path)?;
        let response = self
            .execute_raw_once(method.clone(), url.clone(), scope, false, |request| request)
            .await?;
        let headers = response.headers().clone();
        let bytes = transport::read_response_body(
            response,
            &method,
            &url,
            self.inner.download_body_limit,
            ErrorUrlPolicy::Preserve,
        )
        .await?;
        Ok((headers, bytes))
    }

    async fn execute_with<T, F>(
        &self,
        method: Method,
        path: &str,
        scope: AuthenticationScope,
        configure: F,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        let (method, url, bytes) = self
            .execute_configured(method, path, scope, configure)
            .await?;
        let envelope: ApiResponse<T> =
            serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
                method: method.clone(),
                url: Box::new(url.clone()),
                source,
            })?;
        if !envelope.success {
            return Err(Error::Api {
                method,
                url: Box::new(url),
                message: if envelope.msg.is_empty() {
                    "unknown panel error".to_owned()
                } else {
                    envelope.msg
                },
            });
        }
        Ok(envelope)
    }

    async fn execute_response_with<T, F>(
        &self,
        method: Method,
        path: &str,
        scope: AuthenticationScope,
        configure: F,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        let (method, url, bytes) = self
            .execute_configured(method, path, scope, configure)
            .await?;
        serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
            method,
            url: Box::new(url),
            source,
        })
    }

    async fn execute_configured<F>(
        &self,
        method: Method,
        path: &str,
        scope: AuthenticationScope,
        configure: F,
    ) -> Result<(Method, Url, Vec<u8>)>
    where
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        let url = self.endpoint(path)?;
        let is_unsafe = !matches!(
            method,
            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
        );
        let unsafe_session_request = is_unsafe
            && match scope {
                AuthenticationScope::Session => true,
                AuthenticationScope::PanelApi => self.inner.bearer_token.is_none(),
            };
        let first = self
            .execute_raw_once(
                method.clone(),
                url.clone(),
                scope,
                unsafe_session_request,
                &configure,
            )
            .await;
        let response = if unsafe_session_request && matches!(first, Err(Error::Forbidden { .. })) {
            self.clear_csrf_token().await;
            self.execute_raw_once(
                method.clone(),
                url.clone(),
                scope,
                unsafe_session_request,
                &configure,
            )
            .await?
        } else {
            first?
        };
        let bytes = transport::read_response_body(
            response,
            &method,
            &url,
            self.inner.response_body_limit,
            ErrorUrlPolicy::Preserve,
        )
        .await?;
        Ok((method, url, bytes))
    }

    async fn execute_raw_once<F>(
        &self,
        method: Method,
        url: Url,
        scope: AuthenticationScope,
        csrf_required: bool,
        configure: F,
    ) -> Result<reqwest::Response>
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        let mut request = self.inner.http.request(method.clone(), url.clone());
        if matches!(scope, AuthenticationScope::PanelApi) {
            request = self.authenticate(request);
        }
        if csrf_required {
            let token = self.ensure_csrf_token().await?;
            request = request.header("x-csrf-token", token.expose_secret());
        }
        request = configure(request);

        let response =
            transport::send_request(request, &method, &url, ErrorUrlPolicy::Preserve).await?;
        let status = response.status();
        match status {
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized {
                method,
                url: Box::new(url),
            }),
            StatusCode::FORBIDDEN => Err(Error::Forbidden {
                method,
                url: Box::new(url),
            }),
            _ if !status.is_success() => Err(Error::HttpStatus {
                method,
                url: Box::new(url),
                status,
            }),
            _ => Ok(response),
        }
    }

    fn authenticate(&self, request: RequestBuilder) -> RequestBuilder {
        match self.inner.bearer_token.as_ref() {
            Some(token) => request.bearer_auth(token.expose_secret()),
            None => request,
        }
    }

    pub(crate) async fn ensure_csrf_token(&self) -> Result<SecretString> {
        let mut current = self.inner.csrf_token.lock().await;
        if let Some(token) = current.as_ref() {
            return Ok(token.clone());
        }

        let method = Method::GET;
        let url = self.endpoint("csrf-token")?;
        let response = transport::send_request(
            self.inner.http.get(url.clone()),
            &method,
            &url,
            ErrorUrlPolicy::Preserve,
        )
        .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                method,
                url: Box::new(url),
                status,
            });
        }
        let bytes = transport::read_response_body(
            response,
            &method,
            &url,
            self.inner.response_body_limit,
            ErrorUrlPolicy::Preserve,
        )
        .await?;
        let envelope: ApiResponse<String> =
            serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
                method: method.clone(),
                url: Box::new(url.clone()),
                source,
            })?;
        if !envelope.success {
            return Err(Error::Api {
                method,
                url: Box::new(url),
                message: envelope.msg,
            });
        }
        let token = SecretString::from(envelope.obj.ok_or_else(|| Error::MissingObject {
            method,
            url: Box::new(url),
        })?);
        *current = Some(token.clone());
        Ok(token)
    }

    pub(crate) async fn clear_csrf_token(&self) {
        *self.inner.csrf_token.lock().await = None;
    }
}

fn normalize_base_url(value: &str) -> Result<Url> {
    let mut url = Url::parse(value)
        .map_err(|error| Error::Configuration(format!("invalid panel URL: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Error::Configuration(
            "panel URL scheme must be http or https".to_owned(),
        ));
    }
    if url.host_str().is_none() {
        return Err(Error::Configuration(
            "panel URL must contain a host".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Configuration(
            "panel URL must not contain credentials".to_owned(),
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn origin(url: &Url) -> String {
    let mut value = url.clone();
    value.set_path("");
    value.set_query(None);
    value.set_fragment(None);
    value.to_string().trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    #[test]
    fn normalizes_base_url_without_losing_path() {
        let client = Client::new("https://panel.example.com/secret?ignored=1#fragment").unwrap();
        assert_eq!(
            client.base_url().as_str(),
            "https://panel.example.com/secret/"
        );
        assert_eq!(
            client.endpoint("panel/api/server/status").unwrap().as_str(),
            "https://panel.example.com/secret/panel/api/server/status"
        );
    }

    #[test]
    fn rejects_non_http_urls() {
        assert!(matches!(
            Client::new("ftp://panel.example.com").unwrap_err(),
            Error::Configuration(_)
        ));
    }

    #[test]
    fn rejects_url_credentials_and_empty_bearer_tokens() {
        assert!(matches!(
            Client::new("https://admin:password@panel.example.com").unwrap_err(),
            Error::Configuration(_)
        ));
        assert!(matches!(
            Client::builder("https://panel.example.com")
                .unwrap()
                .bearer_token("  ")
                .build()
                .unwrap_err(),
            Error::Configuration(_)
        ));
    }

    #[test]
    fn debug_output_never_contains_bearer_token_or_secret_base_path() {
        let builder = Client::builder("https://panel.example.com/private-panel-path")
            .unwrap()
            .bearer_token("super-secret-token");
        let builder_debug = format!("{builder:?}");
        assert!(!builder_debug.contains("super-secret-token"));
        assert!(!builder_debug.contains("private-panel-path"));
        assert!(builder_debug.contains("panel.example.com"));
        assert!(builder_debug.contains("BearerToken"));

        let client = builder.build().unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret-token"));
        assert!(!debug.contains("private-panel-path"));
        assert!(debug.contains("panel.example.com"));
        assert!(debug.contains("BearerToken"));
    }

    #[tokio::test]
    async fn bearer_api_request_skips_csrf() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/panel/api/example"))
            .and(matchers::header("authorization", "Bearer api-secret"))
            .and(matchers::header("x-requested-with", "XMLHttpRequest"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "success": true, "obj": 42 })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::builder(server.uri())
            .unwrap()
            .bearer_token("api-secret")
            .build()
            .unwrap();

        let response = client
            .execute::<u32, ()>(
                Method::GET,
                "panel/api/example",
                None,
                AuthenticationScope::PanelApi,
            )
            .await
            .unwrap();

        assert_eq!(response.obj, Some(42));
    }
}
