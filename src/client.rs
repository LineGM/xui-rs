use std::{sync::Arc, time::Duration};

use reqwest::{Method, RequestBuilder, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use tracing::debug;
use url::Url;

use crate::{AuthApi, Error, Result, response::ApiResponse};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_USER_AGENT: &str = concat!("xui-rs/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy)]
pub(crate) enum AuthenticationScope {
    Session,
    #[allow(dead_code)]
    PanelApi,
}

/// Authentication mechanism used for protected panel endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticationKind {
    /// Signed session cookies obtained through `/login`.
    Session,
    /// An API token sent in the `Authorization` header.
    BearerToken,
}

/// Builder for [`Client`].
#[derive(Debug)]
#[must_use]
pub struct ClientBuilder {
    base_url: Url,
    bearer_token: Option<SecretString>,
    timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
    accept_invalid_certs: bool,
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

        let http = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .user_agent(self.user_agent)
            .danger_accept_invalid_certs(self.accept_invalid_certs)
            .build()
            .map_err(|error| Error::Configuration(error.to_string()))?;

        Ok(Client {
            inner: Arc::new(Inner {
                http,
                base_url: self.base_url,
                bearer_token: self.bearer_token,
                csrf_token: Mutex::new(None),
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
            .field("base_url", &self.inner.base_url)
            .field("authentication", &self.authentication_kind())
            .finish_non_exhaustive()
    }
}

struct Inner {
    http: reqwest::Client,
    base_url: Url,
    bearer_token: Option<SecretString>,
    csrf_token: Mutex<Option<SecretString>>,
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

    pub(crate) fn endpoint(&self, path: &str) -> Result<Url> {
        self.inner
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| Error::Configuration(format!("invalid endpoint path: {error}")))
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
        let bytes = response.bytes().await.map_err(|source| Error::Transport {
            method,
            url: Box::new(url),
            source,
        })?;
        Ok((headers, bytes.to_vec()))
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
            .execute_once(
                method.clone(),
                url.clone(),
                scope,
                unsafe_session_request,
                &configure,
            )
            .await;
        if unsafe_session_request && matches!(first, Err(Error::Forbidden { .. })) {
            self.clear_csrf_token().await;
            return self
                .execute_once(method, url, scope, unsafe_session_request, &configure)
                .await;
        }
        first
    }

    async fn execute_once<T, F>(
        &self,
        method: Method,
        url: Url,
        scope: AuthenticationScope,
        csrf_required: bool,
        configure: &F,
    ) -> Result<ApiResponse<T>>
    where
        T: DeserializeOwned,
        F: Fn(RequestBuilder) -> RequestBuilder,
    {
        let response = self
            .execute_raw_once(method.clone(), url.clone(), scope, csrf_required, configure)
            .await?;
        let bytes = response.bytes().await.map_err(|source| Error::Transport {
            method: method.clone(),
            url: Box::new(url.clone()),
            source,
        })?;

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

        debug!(%method, %url, "sending 3x-ui request");
        let response = request.send().await.map_err(|source| Error::Transport {
            method: method.clone(),
            url: Box::new(url.clone()),
            source,
        })?;
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
        let response = self
            .inner
            .http
            .get(url.clone())
            .send()
            .await
            .map_err(|source| Error::Transport {
                method: method.clone(),
                url: Box::new(url.clone()),
                source,
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                method,
                url: Box::new(url),
                status,
            });
        }
        let bytes = response.bytes().await.map_err(|source| Error::Transport {
            method: method.clone(),
            url: Box::new(url.clone()),
            source,
        })?;
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
    fn debug_output_never_contains_bearer_token() {
        let client = Client::builder("https://panel.example.com")
            .unwrap()
            .bearer_token("super-secret-token")
            .build()
            .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret-token"));
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
