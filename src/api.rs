use regex::Regex;
use reqwest::header::COOKIE;
use reqwest::{Client, IntoUrl};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::errors::MyError;

pub struct XUiClient {
    client: Client,
    panel_base_url: url::Url,
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
    /// * `panel_url` - A string slice or any type that can be converted into a URL representing the base URL for the X-UI panel.
    ///
    /// # Returns
    ///
    /// A `Result` containing an instance of `XUiClient` if successful, or a `MyError` if an error occurred.
    ///
    /// # Notes
    ///
    /// - A trailing slash is significant.
    ///   Without it, the last path component is considered to be a "file" name
    ///   to be removed to get at the "directory" that is used as the base.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(panel_url: impl IntoUrl) -> Result<Self, MyError> {
        // Create a new instance of the client with the given base URL.
        // A new HTTP client is created and the session cookie is initially set to None.
        let url = match panel_url.into_url() {
            Ok(url) => url,
            Err(e) => return Err(MyError::ReqwestError(e)),
        };

        let reqwest_client = match Client::builder().build() {
            Ok(reqwest_client) => reqwest_client,
            Err(e) => return Err(MyError::ReqwestError(e)),
        };

        Ok(Self {
            client: reqwest_client,
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
    /// * `username` - Any type that can be converted into a String representing the username for login.
    /// * `password` - Any type that can be converted into a String representing the password for login.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the login operation. If successful, it returns
    /// `Ok(())`. If the login fails, it returns a `MyError` with details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn login(
        &mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<(), MyError> {
        let login_endpoint = match self.panel_base_url.join("login/") {
            Ok(login_endpoint) => login_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        let username_str: String = username.into();
        let password_str: String = password.into();

        let mut params = HashMap::new();
        params.insert("username", &username_str);
        params.insert("password", &password_str);

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
                self.username = Some(username_str);
                self.password = Some(password_str);
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

    /// Sends a GET request to the specified endpoint and returns the JSON response.
    async fn api_get_request(
        &mut self,
        endpoint: impl IntoUrl,
    ) -> Result<serde_json::Value, MyError> {
        let endpoint_url = match endpoint.into_url() {
            Ok(endpoint_url) => endpoint_url,
            Err(e) => return Err(MyError::ReqwestError(e)),
        };

        let response = self
            .with_cookie(self.client.get(endpoint_url))
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
    /// ```rust
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
        let inbounds_list_endpoint = match self.panel_base_url.join("panel/api/inbounds/list/") {
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
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the inbound data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let inbound = client.get_inbound(1_u64).await?;
    ///     println!("Inbound details: {}", inbound);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_inbound(
        &mut self,
        inbound_id: impl Into<u64>,
    ) -> Result<serde_json::Value, MyError> {
        let inbound: u64 = inbound_id.into();

        let inbound_get_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/get/{}/", inbound))
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
    /// * `client_email` - Any type that can be converted into a String representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the client's traffic data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
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
        client_email: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let traffic_by_email_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/getClientTraffics/{}/",
            client_email.into()
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
    /// * `uuid` - Any type that can be converted into a String representing the UUID of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the client's traffic data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
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
        uuid: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let traffic_by_uuid_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/getClientTrafficsById/{}/",
            uuid.into()
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
    /// ```rust
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
            match self.panel_base_url.join("panel/api/inbounds/createbackup/") {
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

    /// Sends a POST request to the specified endpoint with an optional JSON body and returns the JSON response.
    async fn api_post_request(
        &mut self,
        endpoint: impl IntoUrl,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, MyError> {
        let endpoint_url = match endpoint.into_url() {
            Ok(endpoint_url) => endpoint_url,
            Err(e) => return Err(MyError::ReqwestError(e)),
        };

        let mut req_builder = self.with_cookie(self.client.post(endpoint_url)).await?;

        if let Some(json_body) = body {
            req_builder = req_builder.json(json_body);
        }

        let response = req_builder.send().await?;
        let response_as_json = response.json().await?;

        Ok(response_as_json)
    }

    /// Retrieves IP records for a client identified by their email address.
    ///
    /// This function sends a POST request to fetch IP records for a specific client
    /// identified by their email address.
    ///
    /// # Arguments
    ///
    /// * `client_email` - Any type that can be converted into a String representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the client's IP records if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///     let ip_records = client.get_client_ips("user@example.com").await?;
    ///     println!("IP records: {}", ip_records);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_client_ips(
        &mut self,
        client_email: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let client_ips_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/clientIps/{}/",
            client_email.into()
        )) {
            Ok(client_ips_endpoint) => client_ips_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_post_request(client_ips_endpoint, None).await
    }

    /// Adds a new inbound configuration to the 3X-UI panel.
    ///
    /// This function sends a POST request with a JSON body containing the inbound configuration
    /// parameters to add a new inbound.
    ///
    /// # Arguments
    ///
    /// * `inbound_config` - A serde_json::Value containing the inbound configuration parameters.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    /// use serde_json::json;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     let inbound_config = json!({
    ///         "up": 0,
    ///         "down": 0,
    ///         "total": 0,
    ///         "remark": "New Inbound",
    ///         "enable": true,
    ///         "expiryTime": 0,
    ///         "listen": "",
    ///         "port": 10000,
    ///         "protocol": "vmess",
    ///         "settings": "{\"clients\":[{\"id\":\"b831381d-6324-4d53-ad4f-8cda48b30811\",\"alterId\":0,\"email\":\"example@example.com\",\"limitIp\":0,\"totalGB\":0,\"expiryTime\":0,\"enable\":true,\"tgId\":\"\",\"subId\":\"\"}],\"disableInsecureEncryption\":false}",
    ///         "streamSettings": "{\"network\":\"tcp\",\"security\":\"none\",\"tcpSettings\":{\"acceptProxyProtocol\":false,\"header\":{\"type\":\"none\"}}}",
    ///         "sniffing": "{\"enabled\":true,\"destOverride\":[\"http\",\"tls\"]}"
    ///     });
    ///
    ///     let response = client.add_inbound(inbound_config).await?;
    ///     println!("Add inbound response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn add_inbound(
        &mut self,
        inbound_config: serde_json::Value,
    ) -> Result<serde_json::Value, MyError> {
        let add_inbound_endpoint = match self.panel_base_url.join("panel/api/inbounds/add/") {
            Ok(add_inbound_endpoint) => add_inbound_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_post_request(add_inbound_endpoint, Some(&inbound_config))
            .await
    }

    /// Adds a new client to a specific inbound in the 3X-UI panel.
    ///
    /// This function sends a POST request with a JSON body containing the client configuration
    /// to add a new client to an existing inbound. The client JSON is wrapped in a "clients" array
    /// within the settings string.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound to add the client to.
    /// * `client` - A serde_json::Value representing the client object to add.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    /// use serde_json::json;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     let client_config = json!({
    ///         "id": "bbfad557-28f2-47e5-9f3d-e3c7f532fbda",
    ///         "flow": "",
    ///         "email": "new_client@example.com",
    ///         "limitIp": 0,
    ///         "totalGB": 0,
    ///         "expiryTime": 0,
    ///         "enable": true,
    ///         "tgId": "",
    ///         "subId": "sub_id_here",
    ///         "reset": 0
    ///     });
    ///
    ///     let response = client.add_client(5_u64, client_config).await?;
    ///     println!("Add client response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn add_client(
        &mut self,
        inbound_id: impl Into<u64>,
        client: serde_json::Value,
    ) -> Result<serde_json::Value, MyError> {
        let add_client_endpoint = match self.panel_base_url.join("panel/api/inbounds/addClient/") {
            Ok(add_client_endpoint) => add_client_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // Create the settings string with the client in a "clients" array
        let settings_obj_str = serde_json::json!({
            "clients": [client]
        })
        .to_string();

        // Create the request body
        let request_body = serde_json::json!({
            "id": inbound_id.into(),
            "settings": settings_obj_str
        });

        self.api_post_request(add_client_endpoint, Some(&request_body))
            .await
    }

    /// Updates an existing inbound configuration in the 3X-UI panel.
    ///
    /// This function sends a POST request with a JSON body containing the updated inbound configuration
    /// parameters to modify an existing inbound identified by its ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound to update.
    /// * `inbound_config` - A serde_json::Value containing the updated inbound configuration parameters.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    /// use serde_json::json;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // First get the current inbound configuration
    ///     let current_inbound = client.get_inbound(4_u64).await?;
    ///
    ///     // Modify the configuration as needed
    ///     let mut updated_config = current_inbound["obj"].clone();
    ///     updated_config["port"] = json!(44360);
    ///     updated_config["protocol"] = json!("vless");
    ///
    ///     // Update the inbound with the modified configuration
    ///     let response = client.update_inbound(4_u64, updated_config).await?;
    ///     println!("Update inbound response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_inbound(
        &mut self,
        inbound_id: impl Into<u64>,
        inbound_config: serde_json::Value,
    ) -> Result<serde_json::Value, MyError> {
        let id = inbound_id.into();
        let update_inbound_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/update/{}/", id))
        {
            Ok(update_inbound_endpoint) => update_inbound_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        self.api_post_request(update_inbound_endpoint, Some(&inbound_config))
            .await
    }

    /// Updates an existing client in the 3X-UI panel.
    ///
    /// This function sends a POST request with a JSON body containing the updated client configuration
    /// to modify an existing client identified by its UUID.
    ///
    /// # Arguments
    ///
    /// * `uuid` - Any type that can be converted into a String representing the UUID of the client to update.
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound containing the client.
    /// * `client` - A serde_json::Value representing the updated client object.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    /// use serde_json::json;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Define the updated client configuration
    ///     let updated_client = json!({
    ///         "id": "95e4e7bb-7796-47e7-e8a7-f4055194f776",
    ///         "flow": "",
    ///         "email": "updated_client@example.com",
    ///         "limitIp": 2,
    ///         "totalGB": 42949672960_u64,  // 40 GB in bytes, 0 - unlimited
    ///         "expiryTime": 1682864675944_u64,  // Unix timestamp in milliseconds, 0 - unlimited
    ///         "enable": true,
    ///         "tgId": "",
    ///         "subId": "sub_id_here",
    ///         "reset": 0
    ///     });
    ///
    ///     // Update the client
    ///     let response = client.update_client(
    ///         "95e4e7bb-7796-47e7-e8a7-f4055194f776",  // UUID of the client to update
    ///         3_u64,  // Inbound ID
    ///         updated_client
    ///     ).await?;
    ///
    ///     println!("Update client response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_client(
        &mut self,
        uuid: impl Into<String>,
        inbound_id: impl Into<u64>,
        client: serde_json::Value,
    ) -> Result<serde_json::Value, MyError> {
        let client_uuid = uuid.into();
        let update_client_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/updateClient/{}/", client_uuid))
        {
            Ok(update_client_endpoint) => update_client_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // Create the settings string with the client in a "clients" array
        let settings_obj_str = serde_json::json!({
            "clients": [client]
        })
        .to_string();

        // Create the request body
        let request_body = serde_json::json!({
            "id": inbound_id.into(),
            "settings": settings_obj_str
        });

        self.api_post_request(update_client_endpoint, Some(&request_body))
            .await
    }

    /// Clears IP records for a client identified by their email address.
    ///
    /// This function sends a POST request to reset or clear all IP records associated
    /// with a specific client identified by their email address.
    ///
    /// # Arguments
    ///
    /// * `client_email` - Any type that can be converted into a String representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Clear IP records for a client
    ///     let response = client.clear_client_ips("user@example.com").await?;
    ///     println!("Clear IP records response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn clear_client_ips(
        &mut self,
        client_email: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let email = client_email.into();
        let clear_ips_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/clearClientIps/{}/", email))
        {
            Ok(clear_ips_endpoint) => clear_ips_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(clear_ips_endpoint, None).await
    }

    /// Resets traffic statistics for all inbounds in the system.
    ///
    /// This function sends a POST request to reset the traffic statistics for all inbounds
    /// configured in the 3X-UI panel.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Reset traffic statistics for all inbounds
    ///     let response = client.reset_all_traffics().await?;
    ///     println!("Reset all traffics response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_all_traffics(&mut self) -> Result<serde_json::Value, MyError> {
        let reset_all_traffics_endpoint = match self
            .panel_base_url
            .join("panel/api/inbounds/resetAllTraffics/")
        {
            Ok(reset_all_traffics_endpoint) => reset_all_traffics_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(reset_all_traffics_endpoint, None)
            .await
    }

    /// Resets traffic statistics for all clients in a specific inbound.
    ///
    /// This function sends a POST request to reset the traffic statistics for all clients
    /// associated with a specific inbound identified by its ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Reset traffic statistics for all clients in inbound with ID 3
    ///     let response = client.reset_all_client_traffics(3_u64).await?;
    ///     println!("Reset all client traffics response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_all_client_traffics(
        &mut self,
        inbound_id: impl Into<u64>,
    ) -> Result<serde_json::Value, MyError> {
        let id = inbound_id.into();
        let reset_clients_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/resetAllClientTraffics/{}/",
            id
        )) {
            Ok(reset_clients_endpoint) => reset_clients_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(reset_clients_endpoint, None).await
    }

    /// Resets traffic statistics for a specific client in a specific inbound.
    ///
    /// This function sends a POST request to reset the traffic statistics for a specific client
    /// identified by their email address within a particular inbound identified by its ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound.
    /// * `client_email` - Any type that can be converted into a String representing the email address of the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Reset traffic statistics for a specific client in inbound with ID 3
    ///     let response = client.reset_client_traffic(3_u64, "user@example.com").await?;
    ///     println!("Reset client traffic response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_client_traffic(
        &mut self,
        inbound_id: impl Into<u64>,
        client_email: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let id = inbound_id.into();
        let email = client_email.into();

        let reset_client_traffic_endpoint = match self.panel_base_url.join(&format!(
            "panel/api/inbounds/{}/resetClientTraffic/{}/",
            id, email
        )) {
            Ok(reset_client_traffic_endpoint) => reset_client_traffic_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(reset_client_traffic_endpoint, None)
            .await
    }

    /// Deletes a client from a specific inbound.
    ///
    /// This function sends a POST request to delete a client identified by its UUID
    /// from a specific inbound identified by its ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound.
    /// * `client_uuid` - Any type that can be converted into a String representing the UUID of the client to delete.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Delete a client from inbound with ID 3
    ///     let response = client.delete_client(
    ///         3_u64,
    ///         "bf036995-a81d-41b3-8e06-8e233418c96a"
    ///     ).await?;
    ///
    ///     println!("Delete client response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_client(
        &mut self,
        inbound_id: impl Into<u64>,
        client_uuid: impl Into<String>,
    ) -> Result<serde_json::Value, MyError> {
        let id = inbound_id.into();
        let uuid = client_uuid.into();

        let delete_client_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/{}/delClient/{}/", id, uuid))
        {
            Ok(delete_client_endpoint) => delete_client_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(delete_client_endpoint, None).await
    }

    /// Deletes an inbound from the 3X-UI panel.
    ///
    /// This function sends a POST request to delete an inbound identified by its ID.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Any type that can be converted into a u64 representing the ID of the inbound to delete.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Delete inbound with ID 3
    ///     let response = client.delete_inbound(3_u64).await?;
    ///     println!("Delete inbound response: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_inbound(
        &mut self,
        inbound_id: impl Into<u64>,
    ) -> Result<serde_json::Value, MyError> {
        let id = inbound_id.into();

        let delete_inbound_endpoint = match self
            .panel_base_url
            .join(&format!("panel/api/inbounds/del/{}/", id))
        {
            Ok(delete_inbound_endpoint) => delete_inbound_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(delete_inbound_endpoint, None).await
    }

    /// Deletes all depleted clients from a specific inbound or from all inbounds.
    ///
    /// This function sends a POST request to delete all depleted clients (clients that have used all their allocated traffic
    /// or have expired) from a specific inbound identified by its ID. If no inbound ID is provided,
    /// depleted clients will be deleted from all inbounds.
    ///
    /// # Arguments
    ///
    /// * `inbound_id` - Optional parameter that can be converted into a u64 representing the ID of the inbound.
    /// If None, depleted clients will be deleted from all inbounds.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the response if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Delete depleted clients from inbound with ID 4
    ///     let response = client.delete_depleted_clients(Some(4_u64)).await?;
    ///     println!("Delete depleted clients from specific inbound: {}", response);
    ///
    ///     // Delete depleted clients from all inbounds
    ///     let response = client.delete_depleted_clients(None::<u64>).await?;
    ///     println!("Delete depleted clients from all inbounds: {}", response);
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_depleted_clients(
        &mut self,
        inbound_id: Option<impl Into<u64>>,
    ) -> Result<serde_json::Value, MyError> {
        let endpoint_path = match inbound_id {
            Some(id) => format!("panel/api/inbounds/delDepletedClients/{}/", id.into()),
            None => "panel/api/inbounds/delDepletedClients/".to_string(),
        };

        let delete_depleted_clients_endpoint = match self.panel_base_url.join(&endpoint_path) {
            Ok(delete_depleted_clients_endpoint) => delete_depleted_clients_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(delete_depleted_clients_endpoint, None)
            .await
    }

    /// Retrieves a list of currently online clients in the 3X-UI panel.
    ///
    /// This function sends a POST request to fetch information about all clients
    /// that are currently online or active in the system.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value` with the online clients data if successful,
    /// or a `MyError` if an error occurred.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xui_rs::api::XUiClient;
    ///
    /// async fn example() -> Result<(), xui_rs::errors::MyError> {
    ///     let mut client = XUiClient::new("https://your-xui-panel.com/")?;
    ///     client.login("admin", "password").await?;
    ///
    ///     // Get list of online clients
    ///     let online_clients = client.get_online_clients().await?;
    ///     println!("Online clients: {}", online_clients);
    ///     Ok(())
    /// }
    /// ```
    /// TODO: look into multipart example in postman
    pub async fn get_online_clients(&mut self) -> Result<serde_json::Value, MyError> {
        let online_clients_endpoint = match self.panel_base_url.join("panel/api/inbounds/onlines/")
        {
            Ok(online_clients_endpoint) => online_clients_endpoint,
            Err(err) => return Err(MyError::UrlParseError(err)),
        };

        // This endpoint doesn't require a request body
        self.api_post_request(online_clients_endpoint, None).await
    }
}
