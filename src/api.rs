use regex::Regex;
use reqwest::Client;
use reqwest::header::COOKIE;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

use crate::errors::MyError;

pub struct XUiClient {
    client: Client,
    panel_base_url: Url,
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
    /// * `panel_url` - A string slice representing the base URL for the X-UI panel.
    ///
    /// # Returns
    ///
    /// A `Result` containing an instance of `XUiClient` if successful, or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let client = XUiClient::new("https://your-xui-panel.com")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(panel_url: &str) -> Result<Self, MyError> {
        // Create a new instance of the client with the given base URL.
        // A new HTTP client is created and the session cookie is initially set to None.
        let url = match Url::parse(panel_url) {
            Ok(url) => url,
            Err(e) => return Err(MyError::UrlParseError(e)),
        };

        let new_client = match Client::builder().build() {
            Ok(new_client) => new_client,
            Err(e) => return Err(MyError::ReqwestError(e)),
        };

        Ok(Self {
            client: new_client,
            panel_base_url: url,
            session_cookie: None,
            cookie_expiry: None,
            username: None,
            password: None,
        })
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
        }
    }

    /// Logs in to the 3X-UI panel using the provided username and password.
    ///
    /// This function sends a POST request to the login endpoint with the given username and password
    /// in JSON format. If the response is successful, it extracts the session cookie from the
    /// "set-cookie" header and stores it for future authenticated requests.
    ///
    /// # Arguments
    ///
    /// * `username` - A string slice representing the username for login.
    /// * `password` - A string slice representing the password for login.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the login operation. If successful, it returns
    /// `Ok(())`. If the login fails, it returns a `MyError` with details.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), MyError> {
        let login_endpoint = match self.panel_base_url.join("login") {
            Ok(login_endpoint) => login_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);

        let response = self
            .client
            .post(login_endpoint)
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

                // Store credentials for potential re-login
                self.username = Some(username.to_owned());
                self.password = Some(password.to_owned());
            }
            Ok(())
        } else {
            // If the response is not successful, return an error with the status code.
            Err(MyError::CustomError(format!(
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
                return Err(MyError::CustomError(
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
            Err(MyError::CustomError(
                "No session cookie available".to_string(),
            ))
        }
    }

    async fn api_get_request(&mut self, endpoint: Url) -> Result<serde_json::Value, MyError> {
        let response = self
            .with_cookie(self.client.get(endpoint))
            .await?
            .send()
            .await?;

        let response_as_json = response.json().await?;

        Ok(response_as_json)
    }

    /// Retrieves a list of all inbound configurations from the 3X-UI panel.
    ///
    /// This function sends a GET request to the inbounds list endpoint and returns
    /// the JSON response containing all configured inbounds.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the inbounds data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let inbounds = client.get_inbounds().await?;
    ///     println!("Inbounds: {}", inbounds);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_inbounds(&mut self) -> Result<serde_json::Value, MyError> {
        let inbounds_list_endpoint = match self.panel_base_url.join("panel/api/inbounds/list") {
            Ok(inbounds_list_endpoint) => inbounds_list_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_get_request(inbounds_list_endpoint).await
    }

    /// Retrieves the configuration for a specific inbound by its ID.
    ///
    /// This function sends a GET request to fetch details about a specific inbound
    /// configuration identified by the provided ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - A 32-bit unsigned integer representing the ID of the inbound to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the inbound data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let inbound = client.get_inbound(1).await?;
    ///     println!("Inbound details: {}", inbound);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_inbound(&mut self, inbound_id: u32) -> Result<serde_json::Value, MyError> {
        let inbound_get_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/get/{}", inbound_id))
        {
            Ok(inbound_get_endpoint) => inbound_get_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_get_request(inbound_get_endpoint).await
    }

    /// Retrieves traffic information for a client identified by their email address.
    ///
    /// This function sends a GET request to fetch traffic statistics for a specific client
    /// identified by their email address.
    ///
    /// # Arguments
    ///
    /// * `client_email` - A string slice representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the client's traffic data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let traffic = client.get_client_traffic_by_email("user@example.com").await?;
    ///     println!("Traffic data: {}", traffic);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_client_traffic_by_email(
        &mut self,
        client_email: &str,
    ) -> Result<serde_json::Value, MyError> {
        let traffic_by_email_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/getClientTraffics/{}",
            client_email
        )) {
            Ok(traffic_by_email_endpoint) => traffic_by_email_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_get_request(traffic_by_email_endpoint).await
    }

    /// Retrieves traffic information for a client identified by their UUID.
    ///
    /// This function sends a GET request to fetch traffic statistics for a specific client
    /// identified by their UUID.
    ///
    /// # Arguments
    ///
    /// * `uuid` - A string slice representing the UUID of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the client's traffic data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let traffic = client.get_client_traffic_by_uuid("d7c06399-a3e3-4007-9109-19012597dd01").await?;
    ///     println!("Traffic data: {}", traffic);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_client_traffic_by_uuid(
        &mut self,
        uuid: &str,
    ) -> Result<serde_json::Value, MyError> {
        let traffic_by_uuid_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/getClientTrafficsById/{}",
            uuid
        )) {
            Ok(traffic_by_uuid_endpoint) => traffic_by_uuid_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_get_request(traffic_by_uuid_endpoint).await
    }

    /// Creates a backup of the 3X-UI panel configuration.
    ///
    /// This function sends a GET request to trigger the panel's backup creation mechanism.
    /// It returns the HTTP status code of the response.
    ///
    /// # Returns
    ///
    /// A `Result` containing the HTTP status code (u16) if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let status = client.get_backup().await?;
    ///     println!("Backup creation status: {}", status);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backup(&mut self) -> Result<u16, MyError> {
        let create_backup_endpoint =
            match self.panel_base_url.join("panel/api/inbounds/createbackup") {
                Ok(create_backup_endpoint) => create_backup_endpoint,
                Err(err) => return Err(MyError::UrlParseError(err)),
            };

        let response = self
            .with_cookie(self.client.get(create_backup_endpoint))
            .await?
            .send()
            .await?;

        Ok(response.status().as_u16())
    }
}
