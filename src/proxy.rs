//! Explicit outbound proxy configuration shared by every SDK transport.

use std::{fmt, io, str::FromStr};

use reqwest::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use url::Url;

use crate::{Error, Result};

/// Proxy protocol and DNS behavior.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProxyScheme {
    /// Plain HTTP proxy. TLS targets use an HTTP CONNECT tunnel.
    Http,
    /// TLS-protected HTTP proxy. TLS targets use CONNECT inside proxy TLS.
    Https,
    /// SOCKS5 with target hostnames resolved locally.
    Socks5,
    /// SOCKS5 with target hostnames resolved by the proxy.
    Socks5h,
}

impl ProxyScheme {
    /// Returns the exact URL scheme.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
            Self::Socks5h => "socks5h",
        }
    }

    pub(crate) const fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
            Self::Socks5 | Self::Socks5h => 1080,
        }
    }

    pub(crate) const fn resolves_remotely(self) -> bool {
        matches!(self, Self::Socks5h)
    }
}

impl fmt::Display for ProxyScheme {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Explicit proxy used by panel HTTP, WebSocket, and subscription requests.
///
/// Proxy URLs must not contain credentials. Add username/password separately
/// with [`ProxyConfig::with_basic_auth`] so ordinary URL and `Debug` handling
/// cannot expose them.
#[derive(Clone)]
#[must_use]
pub struct ProxyConfig {
    url: Url,
    scheme: ProxyScheme,
    credentials: Option<ProxyCredentials>,
}

#[derive(Clone)]
struct ProxyCredentials {
    username: SecretString,
    password: SecretString,
}

impl ProxyConfig {
    /// Parses an HTTP, HTTPS, SOCKS5, or `SOCKS5h` proxy URL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] for an unsupported/relative URL,
    /// embedded credentials, or a path, query, or fragment.
    pub fn new(url: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(url.as_ref())
            .map_err(|error| Error::Configuration(format!("invalid proxy URL: {error}")))?;
        let scheme = match url.scheme() {
            "http" => ProxyScheme::Http,
            "https" => ProxyScheme::Https,
            "socks5" => ProxyScheme::Socks5,
            "socks5h" => ProxyScheme::Socks5h,
            _ => {
                return Err(Error::Configuration(
                    "proxy URL scheme must be http, https, socks5, or socks5h".to_owned(),
                ));
            }
        };
        if url.host_str().is_none() {
            return Err(Error::Configuration(
                "proxy URL must contain a host".to_owned(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(Error::Configuration(
                "proxy URL must not contain credentials; use with_basic_auth".to_owned(),
            ));
        }
        if !matches!(url.path(), "" | "/") || url.query().is_some() || url.fragment().is_some() {
            return Err(Error::Configuration(
                "proxy URL must not contain a path, query, or fragment".to_owned(),
            ));
        }
        Ok(Self {
            url,
            scheme,
            credentials: None,
        })
    }

    /// Adds HTTP Basic or SOCKS5 username/password authentication.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] when the username is empty/contains a
    /// colon or SOCKS5's one-byte username/password length limit is exceeded.
    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self> {
        let username = username.into();
        let password = password.into();
        if username.is_empty() {
            return Err(Error::Configuration(
                "proxy username must not be empty".to_owned(),
            ));
        }
        if username.contains(':') {
            return Err(Error::Configuration(
                "proxy username must not contain ':'".to_owned(),
            ));
        }
        if matches!(self.scheme, ProxyScheme::Socks5 | ProxyScheme::Socks5h)
            && (username.len() > usize::from(u8::MAX) || password.len() > usize::from(u8::MAX))
        {
            return Err(Error::Configuration(
                "SOCKS5 username and password must each be at most 255 bytes".to_owned(),
            ));
        }
        self.credentials = Some(ProxyCredentials {
            username: SecretString::from(username),
            password: SecretString::from(password),
        });
        Ok(self)
    }

    /// Returns the selected proxy protocol and DNS behavior.
    pub const fn scheme(&self) -> ProxyScheme {
        self.scheme
    }

    /// Returns the credential-free proxy endpoint URL.
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// Returns whether proxy authentication is configured.
    pub const fn has_basic_auth(&self) -> bool {
        self.credentials.is_some()
    }

    pub(crate) fn host(&self) -> &str {
        self.url
            .host_str()
            .expect("ProxyConfig construction requires a host")
    }

    pub(crate) fn port(&self) -> u16 {
        self.url
            .port()
            .unwrap_or_else(|| self.scheme.default_port())
    }

    pub(crate) fn credentials(&self) -> Option<(&str, &str)> {
        self.credentials.as_ref().map(|credentials| {
            (
                credentials.username.expose_secret(),
                credentials.password.expose_secret(),
            )
        })
    }

    pub(crate) fn reqwest_proxy(&self) -> Result<reqwest::Proxy> {
        let proxy = reqwest::Proxy::all(self.url.as_str())
            .map_err(|error| Error::Configuration(format!("invalid proxy: {error}")))?;
        Ok(match self.credentials() {
            Some((username, password)) => proxy.basic_auth(username, password),
            None => proxy,
        })
    }
}

impl fmt::Debug for ProxyConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProxyConfig")
            .field("scheme", &self.scheme)
            .field("endpoint", &"[REDACTED]")
            .field("authenticated", &self.has_basic_auth())
            .finish_non_exhaustive()
    }
}

impl FromStr for ProxyConfig {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ProxyConfig {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl TryFrom<String> for ProxyConfig {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

/// Failure while opening a WebSocket tunnel through an explicit proxy.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ProxyError {
    /// Proxy TCP or TLS transport failed.
    #[error("proxy transport failed: {0}")]
    Io(#[source] io::Error),
    /// SOCKS5 negotiation or target connection failed.
    #[error("SOCKS5 negotiation failed: {0}")]
    Socks5(#[source] Box<tokio_socks::Error>),
    /// HTTP proxy rejected CONNECT.
    #[error("HTTP CONNECT returned {0}")]
    HttpStatus(StatusCode),
    /// HTTP proxy returned a malformed CONNECT response.
    #[error("HTTP proxy returned an invalid CONNECT response")]
    InvalidHttpResponse,
    /// HTTP proxy headers exceeded the SDK's bounded parser limit.
    #[error("HTTP proxy CONNECT response exceeded {limit} bytes")]
    HttpResponseTooLarge {
        /// Maximum accepted CONNECT response header size.
        limit: usize,
    },
    /// Native TLS trust roots could not be initialized.
    #[error("proxy TLS configuration failed: {0}")]
    TlsConfiguration(String),
}

impl ProxyError {
    pub(crate) fn is_timeout(&self) -> bool {
        match self {
            Self::Io(error) => error.kind() == io::ErrorKind::TimedOut,
            Self::Socks5(error) => {
                matches!(error.as_ref(), tokio_socks::Error::Io(error) if error.kind() == io::ErrorKind::TimedOut)
            }
            _ => false,
        }
    }
}
