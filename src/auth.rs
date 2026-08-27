use std::fmt;

use reqwest::Method;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::{Client, Error, Result, client::AuthenticationScope};

/// Credentials accepted by the 3x-ui login endpoint.
#[derive(Clone)]
#[must_use]
pub struct LoginRequest {
    username: String,
    password: SecretString,
    two_factor_code: Option<SecretString>,
}

impl LoginRequest {
    /// Creates credentials for a regular username/password login.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: SecretString::from(password.into()),
            two_factor_code: None,
        }
    }

    /// Adds the one-time code required by panels with 2FA enabled.
    pub fn with_two_factor_code(mut self, code: impl Into<String>) -> Self {
        self.two_factor_code = Some(SecretString::from(code.into()));
        self
    }

    /// Returns the username associated with the request.
    pub fn username(&self) -> &str {
        &self.username
    }
}

impl fmt::Debug for LoginRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginRequest")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field(
                "two_factor_code",
                &self.two_factor_code.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginPayload<'a> {
    username: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    two_factor_code: Option<&'a str>,
}

/// A CSRF token bound to a cookie session.
#[derive(Clone)]
pub struct CsrfToken(SecretString);

impl CsrfToken {
    /// Exposes the token for integrations that need to build a raw request.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CsrfToken([REDACTED])")
    }
}

/// Authentication endpoints for a [`Client`].
#[derive(Clone, Copy, Debug)]
pub struct AuthApi<'client> {
    client: &'client Client,
}

impl<'client> AuthApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Returns the session's CSRF token, creating a pre-auth session if needed.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot be reached or returns an invalid
    /// CSRF response.
    pub async fn csrf_token(self) -> Result<CsrfToken> {
        self.client.ensure_csrf_token().await.map(CsrfToken)
    }

    /// Reports whether two-factor authentication is enabled on the panel.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response omits its
    /// documented boolean value.
    pub async fn is_two_factor_enabled(self) -> Result<bool> {
        let envelope = self
            .client
            .execute::<bool, ()>(
                Method::POST,
                "getTwoFactorEnable",
                None,
                AuthenticationScope::Session,
            )
            .await?;
        let url = self.client.endpoint("getTwoFactorEnable")?;
        envelope.obj.ok_or_else(|| Error::MissingObject {
            method: Method::POST,
            url: Box::new(url),
        })
    }

    /// Authenticates a cookie session.
    ///
    /// The SDK obtains a CSRF token before login and lets the HTTP cookie jar
    /// manage the signed `3x-ui` cookie. Credentials are never retained, and
    /// expired sessions are not silently re-authenticated.
    ///
    /// # Errors
    ///
    /// Returns an error when CSRF initialization, transport, authentication,
    /// or response decoding fails.
    pub async fn login(self, credentials: LoginRequest) -> Result<()> {
        let payload = LoginPayload {
            username: credentials.username(),
            password: credentials.password.expose_secret(),
            two_factor_code: credentials
                .two_factor_code
                .as_ref()
                .map(ExposeSecret::expose_secret),
        };
        self.client
            .execute::<serde_json::Value, _>(
                Method::POST,
                "login",
                Some(&payload),
                AuthenticationScope::Session,
            )
            .await?;
        Ok(())
    }

    /// Clears the current cookie session.
    ///
    /// This does not revoke or remove a bearer token configured on the client.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot clear the session.
    pub async fn logout(self) -> Result<()> {
        self.client
            .execute::<serde_json::Value, ()>(
                Method::POST,
                "logout",
                None,
                AuthenticationScope::Session,
            )
            .await?;
        self.client.clear_csrf_token().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_debug_redacts_all_secrets() {
        let request =
            LoginRequest::new("admin", "password-secret").with_two_factor_code("otp-secret");
        let debug = format!("{request:?}");
        assert!(debug.contains("admin"));
        assert!(!debug.contains("password-secret"));
        assert!(!debug.contains("otp-secret"));
    }

    #[test]
    fn csrf_debug_is_redacted() {
        let token = CsrfToken(SecretString::from("csrf-secret".to_owned()));
        assert!(!format!("{token:?}").contains("csrf-secret"));
        assert_eq!(token.expose(), "csrf-secret");
    }
}
