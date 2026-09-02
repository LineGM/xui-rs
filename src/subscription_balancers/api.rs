use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{SubscriptionBalancer, SubscriptionBalancerInput};
use crate::{Client, Error, Result, client::AuthenticationScope, response::ApiResponse};

const ROOT: &str = "panel/api/sub-balancers";

/// Subscription-balancer endpoints for a [`Client`].
#[derive(Clone, Copy, Debug)]
pub struct SubscriptionBalancersApi<'client> {
    client: &'client Client,
}

impl<'client> SubscriptionBalancersApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Lists balancers in subscription order.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or response decoding fails.
    pub async fn list(self) -> Result<Vec<SubscriptionBalancer>> {
        let envelope = self
            .client
            .execute::<Vec<SubscriptionBalancer>, ()>(
                Method::GET,
                ROOT,
                None,
                AuthenticationScope::PanelApi,
            )
            .await?;
        required_object(self.client, Method::GET, ROOT, envelope)
    }

    /// Creates a subscription balancer.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, transport, or decoding fails.
    pub async fn create(self, input: &SubscriptionBalancerInput) -> Result<SubscriptionBalancer> {
        self.mutate(Method::POST, ROOT, input).await
    }

    /// Replaces a subscription balancer.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, transport, or decoding fails.
    pub async fn update(
        self,
        id: i64,
        input: &SubscriptionBalancerInput,
    ) -> Result<SubscriptionBalancer> {
        self.mutate(Method::POST, &format!("{ROOT}/{id}"), input)
            .await
    }

    /// Deletes a subscription balancer with HTTP DELETE.
    ///
    /// # Errors
    ///
    /// Returns an error when the balancer cannot be deleted.
    pub async fn delete(self, id: i64) -> Result<()> {
        let path = format!("{ROOT}/{id}");
        self.client
            .execute::<Value, ()>(Method::DELETE, &path, None, AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }

    /// Deletes a subscription balancer through the POST compatibility alias.
    ///
    /// # Errors
    ///
    /// Returns an error when the balancer cannot be deleted.
    pub async fn delete_via_post(self, id: i64) -> Result<()> {
        let path = format!("{ROOT}/{id}/del");
        self.client
            .execute_form::<Value, _>(Method::POST, &path, &(), AuthenticationScope::PanelApi)
            .await?;
        Ok(())
    }

    async fn mutate(
        self,
        method: Method,
        path: &str,
        input: &SubscriptionBalancerInput,
    ) -> Result<SubscriptionBalancer> {
        let strategy = input.strategy.as_str().ok_or_else(|| {
            Error::Configuration("an unknown subscription-balancer strategy cannot be sent".into())
        })?;
        let mut form = vec![
            ("remark", input.remark.clone()),
            ("strategy", strategy.to_owned()),
            ("sortOrder", input.sort_order.to_string()),
        ];
        form.extend(
            input
                .inbound_ids
                .iter()
                .map(|id| ("inboundIds", id.to_string())),
        );
        if let Some(enabled) = input.enabled {
            form.push(("enabled", enabled.to_string()));
        }
        let envelope = self
            .client
            .execute_form(method.clone(), path, &form, AuthenticationScope::PanelApi)
            .await?;
        required_object(self.client, method, path, envelope)
    }
}

fn required_object<T>(
    client: &Client,
    method: Method,
    path: &str,
    envelope: ApiResponse<T>,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let url = client.endpoint(path)?;
    envelope.obj.ok_or_else(|| Error::MissingObject {
        method,
        url: Box::new(url),
    })
}
