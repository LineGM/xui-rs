use reqwest::Method;

use super::OpenApiDocument;
use crate::{Client, Result, client::AuthenticationScope};

/// Panel-wide operations that do not belong to a domain controller.
#[derive(Clone, Copy, Debug)]
pub struct PanelApi<'client> {
    client: &'client Client,
}

impl<'client> PanelApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Downloads the runtime `OpenAPI` document used by the panel's API page.
    ///
    /// Unlike normal panel endpoints, this route returns the document directly
    /// rather than wrapping it in a 3x-ui response envelope. Its `servers`
    /// entry reflects the connected panel's configured base path.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or JSON decoding fails.
    pub async fn openapi(self) -> Result<OpenApiDocument> {
        self.client
            .execute_response::<OpenApiDocument, ()>(
                Method::GET,
                "panel/api/openapi.json",
                None,
                AuthenticationScope::PanelApi,
            )
            .await
    }

    /// Sends a fresh database backup to configured Telegram administrators.
    ///
    /// The v3.6.0 handler returns an empty HTTP 200 response and does not report
    /// per-recipient delivery results.
    ///
    /// # Errors
    ///
    /// Returns an error when authentication, CSRF handling, transport, or the
    /// HTTP status fails.
    pub async fn backup_to_telegram(self) -> Result<()> {
        self.client
            .execute_empty::<()>(
                Method::POST,
                "panel/api/backuptotgbot",
                None,
                AuthenticationScope::PanelApi,
            )
            .await
    }
}
