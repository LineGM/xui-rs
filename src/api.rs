use regex::Regex;
use reqwest::Client;
use reqwest::header::COOKIE;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::errors::MyError;

pub struct XUiClient {
    client: Client,
    panel_base_url: String,
    session_cookie: Option<String>,
    cookie_expiry: Option<Instant>,
    username: Option<String>,
    password: Option<String>,
}

impl XUiClient {
    /// Creates a new instance of `XUiClient`.
    ///
    /// Initializes a `XUiClient` with the provided panel base URL, a new HTTP client,
    /// and no session cookie.
    ///
    /// # Arguments
    ///
    /// * `panel_base_url` - A string slice representing the base URL for the client.
    ///
    /// # Returns
    ///
    /// An instance of `XUiClient`.
    pub fn new(panel_url: &str) -> Self {
        // Create a new instance of the client with the given base URL.
        // A new HTTP client is created and the session cookie is initially set to None.
        Self {
            client: Client::new(),
            panel_base_url: panel_url.to_owned(),
            session_cookie: None,
            cookie_expiry: None,
            username: None,
            password: None,
        }
    }

    /// Extracts Max-Age value from cookie string
    fn extract_max_age(&mut self) -> Option<u64> {
        let re = Regex::new(r"Max-Age=(\d+)").ok()?;
        if let Some(ref cookie_str) = self.session_cookie {
            re.captures(cookie_str)?
                .get(1)?
                .as_str()
                .parse::<u64>()
                .ok()
        } else {
            None
        }
    }

    /// Extracts cookie expiry time from cookie string
    fn extract_cookie_expiry(&mut self) {
        // Try to extract Max-Age first
        if let Some(max_age) = self.extract_max_age() {
            self.cookie_expiry = Some(Instant::now() + Duration::from_secs(max_age));
            return;
        }
    }

    /// Logs in to the panel using the provided username and password.
    ///
    /// This function sends a POST request to "/login" with the given username and password
    /// in form data format. If the response is successful, it extracts the session cookie
    /// from the "set-cookie" header and stores it in the client's state. Otherwise, it
    /// returns an error with the status code.
    ///
    /// # Arguments
    ///
    /// * `username` - A string representing the username for login.
    /// * `password` - A string representing the password for login.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the login. If the login is successful, it returns
    /// `Ok(())`. If the login fails, it returns an error with the status code.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), MyError> {
        // Store credentials for potential re-login
        self.username = Some(username.to_owned());
        self.password = Some(password.to_owned());

        let login_endpoint = format!("{}/login", self.panel_base_url);

        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);

        let response = self
            .client
            .post(&login_endpoint)
            .json(&params)
            .send()
            .await?;

        // If the response is successful, extract the session cookie from the
        // "set-cookie" header and store it in the client's state.
        if response.status().is_success() {
            if let Some(cookie) = response.headers().get("set-cookie") {
                let cookie_str = cookie.to_str()?.to_string();
                // Parse expiry time from cookie
                self.session_cookie = Some(cookie_str);
                self.extract_cookie_expiry();
            }
            Ok(())
        } else {
            // If the response is not successful, return an error with the status code.
            Err(MyError::Custom(format!(
                "Login failed with status: {}",
                response.status()
            )))
        }
    }

    /// Checks if the stored session cookie is still valid
    fn is_cookie_valid(&self) -> bool {
        if self.session_cookie.is_none() {
            return false;
        }

        if let Some(expiry) = self.cookie_expiry {
            // Add a minute buffer to account for network delays
            return expiry > Instant::now() + Duration::from_secs(600);
        }

        // If we don't have expiry info, consider it valid if it exists
        self.session_cookie.is_some()
    }

    /// Re-authenticates if the session cookie is expired or missing
    async fn ensure_authenticated(&mut self) -> Result<(), MyError> {
        if !self.is_cookie_valid() {
            if let (Some(username), Some(password)) = (self.username.clone(), self.password.clone())
            {
                return self.login(&username, &password).await;
            } else {
                return Err(MyError::Custom(
                    "Session expired and no credentials available for re-login".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Attaches the session cookie to the request if available.
    async fn with_cookie(
        &mut self,
        req: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, MyError> {
        // Ensure we have a valid authentication cookie
        self.ensure_authenticated().await?;

        // Now attach the cookie to the request
        if let Some(ref cookie) = self.session_cookie {
            Ok(req.header(COOKIE, cookie))
        } else {
            // This should not happen due to ensure_authenticated, but just in case
            Err(MyError::Custom("No session cookie available".to_string()))
        }
    }

    async fn api_get_request(&mut self, endpoint: &str) -> Result<serde_json::Value, MyError> {
        let response = self
            .with_cookie(self.client.get(endpoint))
            .await?
            .send()
            .await?;

        let response_as_json = response.json().await?;

        Ok(response_as_json)
    }

    /// Retrieves a list of all inbound configurations from the panel.
    ///
    /// This function sends a GET request to "/panel/api/inbounds/list"
    /// and returns the JSON response containing a list of inbound configurations.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping a `serde_json::Value` containing the JSON response. If an error occurs
    /// during the request or response parsing, a `MyError` is returned.
    pub async fn get_inbounds(&mut self) -> Result<serde_json::Value, MyError> {
        let inbounds_list_endpoint = format!("{}/panel/api/inbounds/list", self.panel_base_url);
        self.api_get_request(&inbounds_list_endpoint).await
    }

    /// Retrieves the inbound configuration for a specified inbound ID.
    ///
    /// This function sends a GET request to "/panel/api/inbounds/get/<inbound_id>"
    /// and returns the JSON response containing the inbound configuration.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - A 32-bit unsigned integer representing the ID of the inbound.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping a `Value` containing the JSON response. If an error occurs
    /// during the request or response parsing, a `MyError` is returned.
    pub async fn get_inbound(&mut self, inbound_id: u32) -> Result<serde_json::Value, MyError> {
        let inbound_get_endpoint = format!(
            "{}/panel/api/inbounds/get/{}",
            self.panel_base_url, inbound_id
        );

        self.api_get_request(&inbound_get_endpoint).await
    }

    /// Retrieves traffic information for a client identified by email.
    ///
    /// This function sends a GET request to "/panel/api/inbounds/getClientTraffics/<client_email>"
    /// and returns the JSON response containing traffic statistics for the specified client.
    ///
    /// # Arguments
    ///
    /// * `client_email` - A string slice representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping a `serde_json::Value` containing the JSON response with traffic data.
    /// If an error occurs during the request or response parsing, a `MyError` is returned.
    pub async fn get_client_traffic_by_email(
        &mut self,
        client_email: &str,
    ) -> Result<serde_json::Value, MyError> {
        let client_traffic_endpoint = format!(
            "{}/panel/api/inbounds/getClientTraffics/{}",
            self.panel_base_url, client_email
        );

        self.api_get_request(&client_traffic_endpoint).await
    }

    pub async fn get_client_traffic_by_uuid(
        &mut self,
        uuid: &str,
    ) -> Result<serde_json::Value, MyError> {
        let client_traffic_endpoint = format!(
            "{}/panel/api/inbounds/getClientTrafficsById/{}",
            self.panel_base_url, uuid
        );

        self.api_get_request(&client_traffic_endpoint).await
    }

    /// Retrieves traffic information for a client identified by UUID.
    ///
    /// This function sends a GET request to "/panel/api/inbounds/getClientTrafficsById/<uuid>"
    /// and returns the JSON response containing traffic statistics for the specified client.
    ///
    /// # Arguments
    ///
    /// * `uuid` - A string slice representing the UUID of the client.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping a `serde_json::Value` containing the JSON response with traffic data.
    /// If an error occurs during the request or response parsing, a `MyError` is returned.
    pub async fn get_backup(&mut self) -> Result<(), MyError> {
        let client_traffic_endpoint =
            format!("{}/panel/api/inbounds/createbackup", self.panel_base_url);

        let _response = self
            .with_cookie(self.client.get(client_traffic_endpoint))
            .await?
            .send();

        Ok(())
    }
}
