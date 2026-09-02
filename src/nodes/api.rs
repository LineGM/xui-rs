use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{
    NodeMetric, NodeMtlsCa, NodeProbeResult, NodeRequest, NodeUpdateChannel, NodeUpdateResult,
    NodeView, RemoteInboundOption,
};
use crate::{
    Client, Error, HistoryBucket, MetricPoint, Result, WebCertificateFiles,
    client::AuthenticationScope, response::ApiResponse,
};

const ROOT: &str = "panel/api/nodes";

/// Remote node registration, health, discovery, update, and mTLS operations.
#[derive(Clone, Copy, Debug)]
pub struct NodesApi<'client> {
    client: &'client Client,
}

impl<'client> NodesApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Lists direct nodes and read-only transitive projections.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn list(self) -> Result<Vec<NodeView>> {
        self.get_object("list").await
    }

    /// Gets one direct node by its panel-local database ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the node is absent or cannot be decoded.
    pub async fn get(self, id: i64) -> Result<NodeView> {
        self.get_object(&format!("get/{id}")).await
    }

    /// Fetches TLS certificate/key paths that exist on the remote node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node is disabled, unavailable, or its response
    /// cannot be decoded.
    pub async fn web_certificate_files(self, id: i64) -> Result<WebCertificateFiles> {
        self.get_object(&format!("webCert/{id}")).await
    }

    /// Registers a node after the controller verifies reachability.
    ///
    /// API-token credentials are required unless mTLS is selected.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, probing, creation, or decoding fails.
    pub async fn create(self, request: &NodeRequest) -> Result<NodeView> {
        self.post_object("add", request).await
    }

    /// Fully replaces writable node settings.
    ///
    /// A request built with [`NodeView::to_request`] retains the stored token.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, probing, or persistence fails.
    pub async fn update(self, id: i64, request: &NodeRequest) -> Result<()> {
        self.post_empty(&format!("update/{id}"), Some(request))
            .await
    }

    /// Deletes a direct node without migrating its assigned inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when deletion fails.
    pub async fn delete(self, id: i64) -> Result<()> {
        self.post_empty::<()>(&format!("del/{id}"), None).await
    }

    /// Enables or disables node synchronization.
    ///
    /// # Errors
    ///
    /// Returns an error when the state cannot be persisted.
    pub async fn set_enabled(self, id: i64, enabled: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            enable: bool,
        }

        self.post_empty(&format!("setEnable/{id}"), Some(&Body { enable: enabled }))
            .await
    }

    /// Probes unsaved node connection details without persisting them.
    ///
    /// Connectivity failures are returned as [`NodeProbeResult`] with an
    /// offline status rather than as an SDK error.
    ///
    /// # Errors
    ///
    /// Returns an error for request validation, transport, envelope, or decode
    /// failures.
    pub async fn test_connection(self, request: &NodeRequest) -> Result<NodeProbeResult> {
        self.post_object("test", request).await
    }

    /// Fetches the HTTPS leaf certificate's base64 SHA-256 fingerprint.
    ///
    /// The panel intentionally skips verification for this bootstrap request;
    /// callers should compare or explicitly pin the returned fingerprint.
    ///
    /// # Errors
    ///
    /// Returns an error when the candidate is invalid, is not HTTPS, or cannot
    /// be contacted.
    pub async fn certificate_fingerprint(self, request: &NodeRequest) -> Result<String> {
        self.post_object("certFingerprint", request).await
    }

    /// Lists remote inbounds available for selective synchronization.
    ///
    /// # Errors
    ///
    /// Returns an error when the node cannot be contacted or decoded.
    pub async fn remote_inbounds(self, request: &NodeRequest) -> Result<Vec<RemoteInboundOption>> {
        self.post_object("inbounds", request).await
    }

    /// Probes a saved node and updates its cached heartbeat state.
    ///
    /// Connectivity failures are returned as an offline result.
    ///
    /// # Errors
    ///
    /// Returns an error when the node cannot be loaded or the response cannot
    /// be decoded.
    pub async fn probe(self, id: i64) -> Result<NodeProbeResult> {
        self.post_object_without_body(&format!("probe/{id}")).await
    }

    /// Starts the official panel updater on each selected eligible node.
    ///
    /// Offline, disabled, and missing nodes remain successful at the envelope
    /// level and carry a per-node error in [`NodeUpdateResult`].
    ///
    /// # Errors
    ///
    /// Returns an error when no nodes are selected or dispatch itself fails.
    pub async fn update_panels(
        self,
        ids: &[i64],
        channel: NodeUpdateChannel,
    ) -> Result<Vec<NodeUpdateResult>> {
        #[derive(Serialize)]
        struct Body<'a> {
            ids: &'a [i64],
            dev: bool,
        }

        self.post_object(
            "updatePanel",
            &Body {
                ids,
                dev: channel.is_development(),
            },
        )
        .await
    }

    /// Returns up to 60 aggregated points for one node metric.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn history(
        self,
        id: i64,
        metric: NodeMetric,
        bucket: HistoryBucket,
    ) -> Result<Vec<MetricPoint>> {
        self.get_object(&format!(
            "history/{id}/{}/{}",
            metric.as_str(),
            bucket.seconds()
        ))
        .await
    }

    /// Returns this panel's public node-auth CA, lazily creating it if needed.
    ///
    /// # Errors
    ///
    /// Returns an error when CA or master client-certificate generation fails.
    pub async fn mtls_ca(self) -> Result<NodeMtlsCa> {
        self.post_object_without_body("mtls/ca").await
    }

    /// Replaces the CA trusted for incoming node-API client certificates.
    ///
    /// An empty PEM disables incoming node mTLS. The change applies after the
    /// panel restarts.
    ///
    /// # Errors
    ///
    /// Returns an error when a non-empty value is not a valid PEM certificate
    /// or persistence fails.
    pub async fn set_mtls_trust_ca(self, ca_pem: &str) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            ca_cert: &'a str,
        }

        self.post_empty("mtls/trustCA", Some(&Body { ca_cert: ca_pem }))
            .await
    }

    /// Validates the stored master mTLS credential and drops cached pools.
    ///
    /// Subsequent node requests rebuild their transport with the rotated
    /// client certificate.
    ///
    /// # Errors
    ///
    /// Returns an error when the stored credential is invalid or reload fails.
    pub async fn reload_mtls_client(self) -> Result<()> {
        self.post_empty::<()>("mtls/reloadClient", None).await
    }

    async fn get_object<T>(self, suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute::<T, ()>(Method::GET, &path, None, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::GET, &path, envelope)
    }

    async fn post_object<T, B>(self, suffix: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute(
                Method::POST,
                &path,
                Some(body),
                AuthenticationScope::PanelApi,
            )
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    async fn post_object_without_body<T>(self, suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute::<T, ()>(Method::POST, &path, None, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    async fn post_empty<B>(self, suffix: &str, body: Option<&B>) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        self.client
            .execute::<Value, B>(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }

    fn required_object<T>(self, method: Method, path: &str, envelope: ApiResponse<T>) -> Result<T> {
        let url = self.client.endpoint(path)?;
        envelope.obj.ok_or_else(|| Error::MissingObject {
            method,
            url: Box::new(url),
        })
    }
}
