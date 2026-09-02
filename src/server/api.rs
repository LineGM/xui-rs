use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Method, header};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::models::{
    AmneziaWgLogs, ClientIpRecord, DatabaseFile, EchKeyPair, Fail2banStatus, HistoryBucket,
    LegacyCpuPoint, MetricPoint, Mldsa65KeyPair, Mlkem768KeyPair, NodeSummary, PanelLogRequest,
    PanelUpdateInfo, PanelUpdateRun, PanelUpdateStatus, RealityScanRequest, RealityScanResult,
    ServerStatus, SystemMetric, VlessEncryptionOptions, WebCertificateFiles, X25519KeyPair,
    XrayConfig, XrayLogEntry, XrayLogRequest, XrayMetric, XrayMetricsState, XrayObservatoryEntry,
};
use crate::{Client, Error, Result, client::AuthenticationScope, response::ApiResponse};

const ROOT: &str = "panel/api/server";

/// Server status, Xray lifecycle, maintenance, and utility endpoints.
#[derive(Clone, Copy, Debug)]
pub struct ServerApi<'client> {
    client: &'client Client,
}

impl<'client> ServerApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Returns the latest cached machine and Xray status snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the request or response contract fails.
    pub async fn status(self) -> Result<ServerStatus> {
        self.get_object("status").await
    }

    /// Returns CPU history through the legacy `{t, cpu}` endpoint.
    ///
    /// Prefer [`Self::system_history`] for the uniform `{t, v}` shape.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn legacy_cpu_history(self, bucket: HistoryBucket) -> Result<Vec<LegacyCpuPoint>> {
        self.get_object(&format!("cpuHistory/{}", bucket.seconds()))
            .await
    }

    /// Returns aggregated history for one host metric.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn system_history(
        self,
        metric: SystemMetric,
        bucket: HistoryBucket,
    ) -> Result<Vec<MetricPoint>> {
        self.get_object(&format!("history/{}/{}", metric.as_str(), bucket.seconds()))
            .await
    }

    /// Returns Xray metrics discovery and collection state.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_metrics_state(self) -> Result<XrayMetricsState> {
        self.get_object("xrayMetricsState").await
    }

    /// Returns aggregated history for one Xray runtime metric.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_metrics_history(
        self,
        metric: XrayMetric,
        bucket: HistoryBucket,
    ) -> Result<Vec<MetricPoint>> {
        self.get_object(&format!(
            "xrayMetricsHistory/{}/{}",
            metric.as_str(),
            bucket.seconds()
        ))
        .await
    }

    /// Returns the latest Xray observatory snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_observatory(self) -> Result<Vec<XrayObservatoryEntry>> {
        self.get_object("xrayObservatory").await
    }

    /// Returns observatory delay history for one outbound tag.
    ///
    /// The tag is encoded as exactly one URL path segment.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_observatory_history(
        self,
        tag: &str,
        bucket: HistoryBucket,
    ) -> Result<Vec<MetricPoint>> {
        self.get_object(&format!(
            "xrayObservatoryHistory/{}/{}",
            segment(tag),
            bucket.seconds()
        ))
        .await
    }

    /// Lists Xray versions available for installation.
    ///
    /// # Errors
    ///
    /// Returns an error when lookup or decoding fails.
    pub async fn xray_versions(self) -> Result<Vec<String>> {
        self.get_object("getXrayVersion").await
    }

    /// Returns current and latest panel release information.
    ///
    /// # Errors
    ///
    /// Returns an error when lookup or decoding fails.
    pub async fn panel_update_info(self) -> Result<PanelUpdateInfo> {
        self.get_object("getPanelUpdateInfo").await
    }

    /// Returns the latest panel self-update status.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn panel_update_status(self) -> Result<PanelUpdateStatus> {
        self.get_object("getUpdateStatus").await
    }

    /// Returns the complete assembled Xray configuration currently in use.
    ///
    /// Xray configuration is intentionally represented as open-ended JSON because
    /// its schema depends on the installed xray-core version and enabled protocols.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_config(self) -> Result<XrayConfig> {
        self.get_object("getConfigJson").await
    }

    /// Downloads a database backup without writing it to disk.
    ///
    /// # Errors
    ///
    /// Returns an error when the download fails.
    pub async fn download_database(self) -> Result<DatabaseFile> {
        self.download("getDb").await
    }

    /// Downloads the cross-engine migration artifact without writing it to disk.
    ///
    /// # Errors
    ///
    /// Returns an error when the download fails.
    pub async fn download_migration(self) -> Result<DatabaseFile> {
        self.download("getMigration").await
    }

    /// Restores a database upload and lets the panel restart Xray.
    ///
    /// This is destructive. The SDK sends a genuine multipart part named `db` and
    /// never reads from or writes to the local filesystem itself.
    ///
    /// # Errors
    ///
    /// Returns an error when upload, validation, import, or restart fails.
    pub async fn import_database(self, filename: &str, database_bytes: &[u8]) -> Result<()> {
        let path = format!("{ROOT}/importDB");
        self.client
            .execute_multipart::<Value>(
                Method::POST,
                &path,
                "db",
                filename,
                database_bytes,
                AuthenticationScope::PanelApi,
            )
            .await?;
        Ok(())
    }

    /// Generates a UUID v4 using the panel helper.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_uuid(self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct UuidObject {
            uuid: String,
        }

        Ok(self.get_object::<UuidObject>("getNewUUID").await?.uuid)
    }

    /// Returns the panel web certificate and private-key paths.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn web_certificate_files(self) -> Result<WebCertificateFiles> {
        self.get_object("getWebCertFiles").await
    }

    /// Returns read-only summaries of nodes directly managed by this panel.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn descendants(self) -> Result<Vec<NodeSummary>> {
        self.get_object("descendants").await
    }

    /// Generates an X25519 keypair for REALITY.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_x25519(self) -> Result<X25519KeyPair> {
        self.get_object("getNewX25519Cert").await
    }

    /// Generates ML-DSA-65 signing material.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_mldsa65(self) -> Result<Mldsa65KeyPair> {
        self.get_object("getNewmldsa65").await
    }

    /// Generates ML-KEM-768 key material.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_mlkem768(self) -> Result<Mlkem768KeyPair> {
        self.get_object("getNewmlkem768").await
    }

    /// Generates VLESS encryption/decryption choices.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_vless_encryption(self) -> Result<VlessEncryptionOptions> {
        self.get_object("getNewVlessEnc").await
    }

    /// Generates ECH server keys and a public ECH config list for an SNI.
    ///
    /// # Errors
    ///
    /// Returns an error when generation or decoding fails.
    pub async fn generate_ech(self, sni: &str) -> Result<EchKeyPair> {
        #[derive(Serialize)]
        struct Form<'a> {
            sni: &'a str,
        }

        self.post_form_object("getNewEchCert", &Form { sni }).await
    }

    /// Computes SHA-256 hashes for a certificate referenced by panel file path.
    ///
    /// The server accepts only certificate files already referenced by its own
    /// configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, hashing, or decoding fails.
    pub async fn certificate_file_hashes(self, certificate_file: &str) -> Result<Vec<String>> {
        self.certificate_hashes(certificate_file, "").await
    }

    /// Computes SHA-256 hashes for inline PEM or DER certificate content.
    ///
    /// # Errors
    ///
    /// Returns an error when parsing, hashing, or decoding fails.
    pub async fn certificate_content_hashes(
        self,
        certificate_content: &str,
    ) -> Result<Vec<String>> {
        self.certificate_hashes("", certificate_content).await
    }

    /// Fetches the live leaf certificate hash from a remote TLS server.
    ///
    /// # Errors
    ///
    /// Returns an error when probing, hashing, or decoding fails.
    pub async fn remote_certificate_hashes(self, server: &str) -> Result<Vec<String>> {
        #[derive(Serialize)]
        struct Form<'a> {
            server: &'a str,
        }

        self.post_form_object("getRemoteCertHash", &Form { server })
            .await
    }

    /// Probes one REALITY target and returns its feasibility verdict.
    ///
    /// # Errors
    ///
    /// Returns an error when the live probe or decoding fails.
    pub async fn scan_reality_target(
        self,
        request: &RealityScanRequest,
    ) -> Result<RealityScanResult> {
        #[derive(Serialize)]
        struct Form<'a> {
            target: &'a str,
            xver: i32,
        }

        self.post_form_object(
            "scanRealityTarget",
            &Form {
                target: &request.target,
                xver: request.xray_version,
            },
        )
        .await
    }

    /// Probes comma-separated REALITY targets, or the panel seed list when empty.
    ///
    /// # Errors
    ///
    /// Returns an error when live probes or decoding fail.
    pub async fn scan_reality_targets(
        self,
        targets: &[impl AsRef<str>],
    ) -> Result<Vec<RealityScanResult>> {
        #[derive(Serialize)]
        struct Form {
            targets: String,
        }

        self.post_form_object(
            "scanRealityTargets",
            &Form {
                targets: targets
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .join(","),
            },
        )
        .await
    }

    /// Probes the panel's built-in REALITY target seed list.
    ///
    /// This is the unambiguous convenience form of
    /// [`Self::scan_reality_targets`] for the upstream empty-list behavior.
    ///
    /// # Errors
    ///
    /// Returns an error when live probes or decoding fail.
    pub async fn scan_default_reality_targets(self) -> Result<Vec<RealityScanResult>> {
        self.scan_reality_targets(&[] as &[&str]).await
    }

    /// Returns the cluster-wide recently observed client IP table.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn client_ips(self) -> Result<Vec<ClientIpRecord>> {
        self.get_object("clientIps").await
    }

    /// Merges client IP observations into the panel's cluster-wide table.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or merge fails.
    pub async fn merge_client_ips(self, records: &[ClientIpRecord]) -> Result<()> {
        self.post_empty("clientIps", Some(records)).await
    }

    /// Reports whether Fail2ban-backed per-client IP limits are usable.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn fail2ban_status(self) -> Result<Fail2banStatus> {
        self.get_object("fail2banStatus").await
    }

    /// Stops Xray immediately.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot stop Xray.
    pub async fn stop_xray(self) -> Result<()> {
        self.post_empty::<()>("stopXrayService", None).await
    }

    /// Restarts Xray with the current configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or restart fails.
    pub async fn restart_xray(self) -> Result<()> {
        self.post_empty::<()>("restartXrayService", None).await
    }

    /// Downloads and installs an Xray release tag, or `latest`.
    ///
    /// The version is encoded as exactly one URL path segment.
    ///
    /// # Errors
    ///
    /// Returns an error when download or installation fails.
    pub async fn install_xray(self, version: &str) -> Result<()> {
        self.post_empty::<()>(&format!("installXray/{}", segment(version)), None)
            .await
    }

    /// Starts a panel self-update and returns its run identifier.
    ///
    /// `dev_override` selects the channel for this run when present; `None` uses
    /// the panel's configured update channel.
    ///
    /// # Errors
    ///
    /// Returns an error when the updater cannot be started or decoded.
    pub async fn update_panel(self, dev_override: Option<bool>) -> Result<PanelUpdateRun> {
        #[derive(Serialize)]
        struct Form {
            #[serde(skip_serializing_if = "Option::is_none")]
            dev: Option<bool>,
        }

        self.post_form_object("updatePanel", &Form { dev: dev_override })
            .await
    }

    /// Selects stable (`false`) or rolling dev (`true`) for future panel updates.
    ///
    /// # Errors
    ///
    /// Returns an error when the setting cannot be changed.
    pub async fn set_update_channel(self, dev: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Form {
            dev: bool,
        }

        self.post_form_empty("setUpdateChannel", &Form { dev })
            .await
    }

    /// Refreshes the panel's default `GeoIP` and `GeoSite` files.
    ///
    /// # Errors
    ///
    /// Returns an error when the update fails.
    pub async fn update_geofiles(self) -> Result<()> {
        self.post_empty::<()>("updateGeofile", None).await
    }

    /// Refreshes one named Xray geo file.
    ///
    /// The filename is encoded as one path segment; the server additionally
    /// enforces its v3.7.0 safe-filename allowlist.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or update fails.
    pub async fn update_geofile(self, filename: &str) -> Result<()> {
        self.post_empty::<()>(&format!("updateGeofile/{}", segment(filename)), None)
            .await
    }

    /// Returns filtered trailing lines from the panel log.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn panel_logs(self, request: &PanelLogRequest) -> Result<Vec<String>> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            level: &'a str,
            syslog: bool,
        }

        self.post_form_object(
            &format!("logs/{}", request.count),
            &Form {
                level: request.level.as_str(),
                syslog: request.syslog,
            },
        )
        .await
    }

    /// Returns filtered trailing lines from the Xray process log.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn xray_logs(self, request: &XrayLogRequest) -> Result<Vec<XrayLogEntry>> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            filter: &'a str,
            show_direct: bool,
            show_blocked: bool,
            show_proxy: bool,
        }

        self.post_form_object(
            &format!("xraylogs/{}", request.count),
            &Form {
                filter: &request.filter,
                show_direct: request.show_direct,
                show_blocked: request.show_blocked,
                show_proxy: request.show_proxy,
            },
        )
        .await
    }

    /// Returns live `AmneziaWG` peer activity and recent lifecycle log lines.
    ///
    /// The server caps invalid or out-of-range counts to 100 and applies the
    /// case-insensitive filter to both peer metadata and event text.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or response decoding fails.
    pub async fn amneziawg_logs(self, count: u32, filter: &str) -> Result<AmneziaWgLogs> {
        #[derive(Serialize)]
        struct Form<'a> {
            filter: &'a str,
        }

        self.post_form_object(&format!("amneziawglogs/{count}"), &Form { filter })
            .await
    }

    async fn certificate_hashes(
        self,
        certificate_file: &str,
        certificate_content: &str,
    ) -> Result<Vec<String>> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Form<'a> {
            cert_file: &'a str,
            cert_content: &'a str,
        }

        self.post_form_object(
            "getCertHash",
            &Form {
                cert_file: certificate_file,
                cert_content: certificate_content,
            },
        )
        .await
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

    async fn post_form_object<T, B>(self, suffix: &str, form: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute_form(Method::POST, &path, form, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    async fn post_form_empty<B>(self, suffix: &str, form: &B) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        self.client
            .execute_form::<Value, _>(Method::POST, &path, form, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
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

    async fn download(self, suffix: &str) -> Result<DatabaseFile> {
        let path = format!("{ROOT}/{suffix}");
        let (headers, bytes) = self
            .client
            .execute_bytes(Method::GET, &path, AuthenticationScope::PanelApi)
            .await?;
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        if content_type
            .as_deref()
            .is_some_and(|value| value.starts_with("application/json"))
        {
            let url = self.client.endpoint(&path)?;
            let envelope: ApiResponse<Value> =
                serde_json::from_slice(&bytes).map_err(|source| Error::Decode {
                    method: Method::GET,
                    url: Box::new(url.clone()),
                    source,
                })?;
            if !envelope.success {
                return Err(Error::Api {
                    method: Method::GET,
                    url: Box::new(url),
                    message: if envelope.msg.is_empty() {
                        "database download failed".to_owned()
                    } else {
                        envelope.msg
                    },
                });
            }
        }
        Ok(DatabaseFile {
            filename: attachment_filename(&headers),
            content_type,
            bytes,
        })
    }

    fn required_object<T>(self, method: Method, path: &str, envelope: ApiResponse<T>) -> Result<T> {
        let url = self.client.endpoint(path)?;
        envelope.obj.ok_or_else(|| Error::MissingObject {
            method,
            url: Box::new(url),
        })
    }
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn attachment_filename(headers: &header::HeaderMap) -> Option<String> {
    let disposition = headers.get(header::CONTENT_DISPOSITION)?.to_str().ok()?;
    disposition.split(';').skip(1).find_map(|parameter| {
        let (name, value) = parameter.trim().split_once('=')?;
        name.eq_ignore_ascii_case("filename")
            .then(|| value.trim().trim_matches('"').to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_segments_encode_reserved_and_unicode_characters() {
        assert_eq!(segment("v1/latest"), "v1%2Flatest");
        assert_eq!(segment("узел 1"), "%D1%83%D0%B7%D0%B5%D0%BB%201");
    }

    #[test]
    fn extracts_quoted_attachment_filename() {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_static("attachment; filename=\"x-ui.db\""),
        );
        assert_eq!(attachment_filename(&headers).as_deref(), Some("x-ui.db"));
    }
}
