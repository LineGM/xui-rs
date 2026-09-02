use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{HostGroup, HostRow};
use crate::{Client, Error, Result, client::AuthenticationScope, response::ApiResponse};

const ROOT: &str = "panel/api/hosts";

/// Per-inbound subscription host override operations.
#[derive(Clone, Copy, Debug)]
pub struct HostsApi<'client> {
    client: &'client Client,
}

impl<'client> HostsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Lists every logical host group.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn list(self) -> Result<Vec<HostGroup>> {
        self.get_object("list").await
    }

    /// Gets one logical host group.
    ///
    /// The group ID is encoded as exactly one URL path segment.
    ///
    /// # Errors
    ///
    /// Returns an error when the group is absent or cannot be decoded.
    pub async fn get(self, group_id: &str) -> Result<HostGroup> {
        self.get_object(&format!("get/{}", segment(group_id))).await
    }

    /// Lists groups attached to one inbound.
    ///
    /// A source-level `null` result is normalized to an empty vector.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn list_by_inbound(self, inbound_id: i64) -> Result<Vec<HostGroup>> {
        let path = format!("{ROOT}/byInbound/{inbound_id}");
        let envelope = self
            .client
            .execute::<Value, ()>(Method::GET, &path, None, AuthenticationScope::PanelApi)
            .await?;
        let Some(value) = envelope.obj else {
            return Ok(Vec::new());
        };
        let url = self.client.endpoint(&path)?;
        serde_json::from_value(value).map_err(|source| Error::Decode {
            method: Method::GET,
            url: Box::new(url),
            source,
        })
    }

    /// Returns the sorted distinct host tags used by the panel.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn tags(self) -> Result<Vec<String>> {
        self.get_object("tags").await
    }

    /// Creates a host group and returns every physical row produced by the
    /// inbound/address Cartesian product.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, creation, or decoding fails.
    pub async fn create(self, group: &HostGroup) -> Result<Vec<HostRow>> {
        self.post_object("add", group).await
    }

    /// Creates a host group through the bulk alias.
    ///
    /// This has the same request and response semantics as [`Self::create`].
    ///
    /// # Errors
    ///
    /// Returns an error when validation, creation, or decoding fails.
    pub async fn bulk_create(self, group: &HostGroup) -> Result<Vec<HostRow>> {
        self.post_object("bulk/add", group).await
    }

    /// Fully replaces a group while preserving its path-level group ID.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, replacement, or decoding fails.
    pub async fn update(self, group_id: &str, group: &HostGroup) -> Result<Vec<HostRow>> {
        self.post_object(&format!("update/{}", segment(group_id)), group)
            .await
    }

    /// Deletes one logical host group.
    ///
    /// # Errors
    ///
    /// Returns an error when deletion fails.
    pub async fn delete(self, group_id: &str) -> Result<()> {
        self.post_empty::<()>(&format!("del/{}", segment(group_id)), None)
            .await
    }

    /// Enables or disables one logical host group.
    ///
    /// # Errors
    ///
    /// Returns an error when the state cannot be changed.
    pub async fn set_enabled(self, group_id: &str, enabled: bool) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            enable: bool,
        }

        self.post_empty(
            &format!("setEnable/{}", segment(group_id)),
            Some(&Body { enable: enabled }),
        )
        .await
    }

    /// Reorders groups by their positions in `group_ids`.
    ///
    /// # Errors
    ///
    /// Returns an error when the order cannot be persisted.
    pub async fn reorder(self, group_ids: &[impl AsRef<str>]) -> Result<()> {
        self.post_ids("reorder", group_ids).await
    }

    /// Enables or disables multiple groups atomically at the database level.
    ///
    /// # Errors
    ///
    /// Returns an error when the state cannot be changed.
    pub async fn bulk_set_enabled(
        self,
        group_ids: &[impl AsRef<str>],
        enabled: bool,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            ids: Vec<String>,
            enable: bool,
        }

        self.post_empty(
            "bulk/setEnable",
            Some(&Body {
                ids: owned_ids(group_ids),
                enable: enabled,
            }),
        )
        .await
    }

    /// Deletes multiple logical host groups.
    ///
    /// # Errors
    ///
    /// Returns an error when deletion fails.
    pub async fn bulk_delete(self, group_ids: &[impl AsRef<str>]) -> Result<()> {
        self.post_ids("bulk/del", group_ids).await
    }

    async fn post_ids(self, suffix: &str, group_ids: &[impl AsRef<str>]) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            ids: Vec<String>,
        }

        self.post_empty(
            suffix,
            Some(&Body {
                ids: owned_ids(group_ids),
            }),
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

fn owned_ids(values: &[impl AsRef<str>]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect()
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}
