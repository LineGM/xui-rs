use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::models::{
    ActiveInboundsByGuid, AffectedCount, BulkAdjustRequest, BulkAdjustResult, BulkAttachResult,
    BulkCreateResult, BulkDeleteResult, BulkDetachResult, BulkSetEnabledResult, ClientConfig,
    ClientCreateRequest, ClientDetails, ClientExternalLinkInput, ClientIpInfo, ClientIpsByGuid,
    ClientMutationStatus, ClientPage, ClientPageRequest, ClientWithAttachments, ClientsByGuid,
    DeletedCount, GroupName, GroupSummary, LastOnlineByEmail,
};
use crate::{
    Client, ClientTraffic, Error, Result, client::AuthenticationScope, response::ApiResponse,
};

const ROOT: &str = "panel/api/clients";

/// Client and client-group endpoints for a [`Client`].
#[derive(Clone, Copy, Debug)]
pub struct ClientsApi<'client> {
    client: &'client Client,
}

impl<'client> ClientsApi<'client> {
    pub(crate) const fn new(client: &'client Client) -> Self {
        Self { client }
    }

    /// Lists all clients with full records, attachments, and traffic.
    ///
    /// # Errors
    ///
    /// Returns an error when the request or response contract fails.
    pub async fn list(self) -> Result<Vec<ClientWithAttachments>> {
        self.get_object("list").await
    }

    /// Lists one filtered page with stable global summary counters.
    ///
    /// # Errors
    ///
    /// Returns an error when query encoding, transport, or decoding fails.
    pub async fn list_paged(self, request: &ClientPageRequest) -> Result<ClientPage> {
        let query = PageQuery::from(request);
        let path = format!("{ROOT}/list/paged");
        let envelope = self
            .client
            .execute_query::<ClientPage, _>(
                Method::GET,
                &path,
                &query,
                AuthenticationScope::PanelApi,
            )
            .await?;
        self.required_object(Method::GET, &path, envelope)
    }

    /// Fetches a client by its unique email/name.
    ///
    /// # Errors
    ///
    /// Returns an error when the client cannot be fetched or decoded.
    pub async fn get(self, email: &str) -> Result<ClientDetails> {
        self.get_object(&format!("get/{}", segment(email))).await
    }

    /// Fetches all clients associated with a positive Telegram user ID.
    ///
    /// This source-defined v3.6.0 endpoint is absent from upstream `OpenAPI`.
    ///
    /// # Errors
    ///
    /// Returns an error when the lookup or response decoding fails.
    pub async fn get_by_telegram_id(self, telegram_id: i64) -> Result<Vec<ClientDetails>> {
        self.get_object(&format!("get/tgId/{telegram_id}")).await
    }

    /// Returns one client's shared traffic row.
    ///
    /// # Errors
    ///
    /// Returns an error when traffic cannot be fetched or decoded.
    pub async fn traffic(self, email: &str) -> Result<ClientTraffic> {
        self.get_object(&format!("traffic/{}", segment(email)))
            .await
    }

    /// Returns all share links for a subscription identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when link generation or decoding fails.
    pub async fn subscription_links(self, sub_id: &str) -> Result<Vec<String>> {
        self.get_object(&format!("subLinks/{}", segment(sub_id)))
            .await
    }

    /// Returns all share links for one client across attached inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when link generation or decoding fails.
    pub async fn links(self, email: &str) -> Result<Vec<String>> {
        self.get_object(&format!("links/{}", segment(email))).await
    }

    /// Creates and attaches a client.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, transport, or decoding fails.
    pub async fn create(self, request: &ClientCreateRequest) -> Result<ClientMutationStatus> {
        self.post_status("add", request).await
    }

    /// Replaces a client's full writable configuration on every attachment.
    ///
    /// Start with [`crate::ClientRecord::to_config`] to preserve fields. The endpoint
    /// is replacement-based, not PATCH.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, transport, or decoding fails.
    pub async fn update(self, email: &str, client: &ClientConfig) -> Result<ClientMutationStatus> {
        self.post_status(&format!("update/{}", segment(email)), client)
            .await
    }

    /// Restricts per-inbound settings replacement to selected attachments.
    ///
    /// Canonical client fields such as group and enabled state remain global.
    /// An empty `inbound_ids` slice has the server's unfiltered update meaning.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, transport, or decoding fails.
    pub async fn update_on_inbounds(
        self,
        email: &str,
        client: &ClientConfig,
        inbound_ids: &[i64],
    ) -> Result<ClientMutationStatus> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Query {
            inbound_ids: String,
        }

        let suffix = format!("update/{}", segment(email));
        let path = format!("{ROOT}/{suffix}");
        let query = Query {
            inbound_ids: join_numbers(inbound_ids),
        };
        let envelope = self
            .client
            .execute_query_json::<ClientMutationStatus, _, _>(
                Method::POST,
                &path,
                &query,
                client,
                AuthenticationScope::PanelApi,
            )
            .await?;
        Ok(envelope.obj.unwrap_or_default())
    }

    /// Deletes a client from every inbound.
    ///
    /// Set `keep_traffic` to retain its traffic row for a future recreation.
    ///
    /// # Errors
    ///
    /// Returns an error when the panel cannot complete the deletion.
    pub async fn delete(self, email: &str, keep_traffic: bool) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Query {
            keep_traffic: u8,
        }

        let path = format!("{ROOT}/del/{}", segment(email));
        self.client
            .execute_query::<Value, _>(
                Method::POST,
                &path,
                &Query {
                    keep_traffic: u8::from(keep_traffic),
                },
                AuthenticationScope::PanelApi,
            )
            .await?;
        Ok(())
    }

    /// Attaches an existing client to additional inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or attachment fails.
    pub async fn attach(self, email: &str, inbound_ids: &[i64]) -> Result<ClientMutationStatus> {
        self.post_status(
            &format!("{}/attach", segment(email)),
            &InboundIdsBody { inbound_ids },
        )
        .await
    }

    /// Detaches a client from selected inbounds without deleting its record.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or detachment fails.
    pub async fn detach(self, email: &str, inbound_ids: &[i64]) -> Result<ClientMutationStatus> {
        self.post_status(
            &format!("{}/detach", segment(email)),
            &InboundIdsBody { inbound_ids },
        )
        .await
    }

    /// Replaces all external links and remote subscriptions for a client.
    ///
    /// # Errors
    ///
    /// Returns an error when a link is invalid or replacement fails.
    pub async fn set_external_links(
        self,
        email: &str,
        external_links: &[ClientExternalLinkInput],
    ) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            external_links: &'a [ClientExternalLinkInput],
        }

        self.post_empty(
            &format!("{}/externalLinks", segment(email)),
            Some(&Body { external_links }),
        )
        .await
    }

    /// Exports every client in the portable create/import shape.
    ///
    /// # Errors
    ///
    /// Returns an error when export or response decoding fails.
    pub async fn export(self) -> Result<Vec<ClientCreateRequest>> {
        self.get_object("export").await
    }

    /// Imports portable client payloads without overwriting existing emails.
    ///
    /// # Errors
    ///
    /// Returns an error when nested JSON encoding, validation, or transport fails.
    pub async fn import(self, clients: &[ClientCreateRequest]) -> Result<BulkCreateResult> {
        #[derive(Serialize)]
        struct Body {
            data: String,
        }

        let data = serde_json::to_string(clients).map_err(|source| Error::Encode {
            operation: "import clients",
            source,
        })?;
        self.post_object("import", Some(&Body { data })).await
    }

    /// Deletes all unattached client records.
    ///
    /// # Errors
    ///
    /// Returns an error when cleanup or decoding fails.
    pub async fn delete_orphans(self) -> Result<DeletedCount> {
        self.post_object::<DeletedCount, ()>("delOrphans", None)
            .await
    }

    /// Resets traffic for every client.
    ///
    /// # Errors
    ///
    /// Returns an error when the reset fails.
    pub async fn reset_all_traffic(self) -> Result<()> {
        self.post_empty::<()>("resetAllTraffics", None).await
    }

    /// Deletes all expired or quota-depleted clients.
    ///
    /// # Errors
    ///
    /// Returns an error when cleanup or decoding fails.
    pub async fn delete_depleted(self) -> Result<DeletedCount> {
        self.post_object::<DeletedCount, ()>("delDepleted", None)
            .await
    }

    /// Applies a signed expiry/quota change and optional typed flow directive.
    ///
    /// # Errors
    ///
    /// Returns an error when adjustment or decoding fails.
    pub async fn bulk_adjust(self, request: &BulkAdjustRequest) -> Result<BulkAdjustResult> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            emails: &'a [String],
            add_days: i32,
            add_bytes: i64,
            flow: &'static str,
        }

        self.post_object(
            "bulkAdjust",
            Some(&Body {
                emails: &request.emails,
                add_days: request.add_days,
                add_bytes: request.add_bytes,
                flow: request.flow.as_str(),
            }),
        )
        .await
    }

    /// Enables multiple clients.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_enable(self, emails: &[String]) -> Result<BulkSetEnabledResult> {
        self.post_object("bulkEnable", Some(&EmailsBody { emails }))
            .await
    }

    /// Disables multiple clients.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_disable(self, emails: &[String]) -> Result<BulkSetEnabledResult> {
        self.post_object("bulkDisable", Some(&EmailsBody { emails }))
            .await
    }

    /// Deletes multiple clients.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_delete(
        self,
        emails: &[String],
        keep_traffic: bool,
    ) -> Result<BulkDeleteResult> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            emails: &'a [String],
            keep_traffic: bool,
        }

        self.post_object(
            "bulkDel",
            Some(&Body {
                emails,
                keep_traffic,
            }),
        )
        .await
    }

    /// Creates multiple clients and reports per-item skips.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_create(self, clients: &[ClientCreateRequest]) -> Result<BulkCreateResult> {
        self.post_object("bulkCreate", Some(clients)).await
    }

    /// Attaches multiple clients to multiple inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_attach(
        self,
        emails: &[String],
        inbound_ids: &[i64],
    ) -> Result<BulkAttachResult> {
        self.post_object(
            "bulkAttach",
            Some(&BulkAttachmentBody {
                emails,
                inbound_ids,
            }),
        )
        .await
    }

    /// Detaches multiple clients from multiple inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk operation or decoding fails.
    pub async fn bulk_detach(
        self,
        emails: &[String],
        inbound_ids: &[i64],
    ) -> Result<BulkDetachResult> {
        self.post_object(
            "bulkDetach",
            Some(&BulkAttachmentBody {
                emails,
                inbound_ids,
            }),
        )
        .await
    }

    /// Resets traffic for multiple clients and returns the affected count.
    ///
    /// # Errors
    ///
    /// Returns an error when the bulk reset or decoding fails.
    pub async fn bulk_reset_traffic(self, emails: &[String]) -> Result<AffectedCount> {
        self.post_object("bulkResetTraffic", Some(&EmailsBody { emails }))
            .await
    }

    /// Resets traffic for one client and re-enables it on attached inbounds.
    ///
    /// # Errors
    ///
    /// Returns an error when the reset fails.
    pub async fn reset_traffic(self, email: &str) -> Result<()> {
        self.post_empty::<()>(&format!("resetTraffic/{}", segment(email)), None)
            .await
    }

    /// Replaces one client's upload and download counters.
    ///
    /// # Errors
    ///
    /// Returns an error when the update fails.
    pub async fn update_traffic(self, email: &str, upload: i64, download: i64) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            upload: i64,
            download: i64,
        }

        self.post_empty(
            &format!("updateTraffic/{}", segment(email)),
            Some(&Body { upload, download }),
        )
        .await
    }

    /// Lists display-ready source IP observations for one client.
    ///
    /// # Errors
    ///
    /// Returns an error when lookup or decoding fails.
    pub async fn ips(self, email: &str) -> Result<Vec<ClientIpInfo>> {
        self.post_object::<Vec<ClientIpInfo>, ()>(&format!("ips/{}", segment(email)), None)
            .await
    }

    /// Clears one client's recorded source IPs.
    ///
    /// # Errors
    ///
    /// Returns an error when cleanup fails.
    pub async fn clear_ips(self, email: &str) -> Result<()> {
        self.post_empty::<()>(&format!("clearIps/{}", segment(email)), None)
            .await
    }

    /// Lists emails currently online across the panel tree.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn onlines(self) -> Result<Vec<String>> {
        self.post_object::<Vec<String>, ()>("onlines", None).await
    }

    /// Lists online emails grouped by stable panel GUID.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn onlines_by_guid(self) -> Result<ClientsByGuid> {
        self.post_object::<ClientsByGuid, ()>("onlinesByGuid", None)
            .await
    }

    /// Returns source-IP observations grouped by panel GUID and email.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn client_ips_by_guid(self) -> Result<ClientIpsByGuid> {
        self.post_object::<ClientIpsByGuid, ()>("clientIpsByGuid", None)
            .await
    }

    /// Returns recently active inbound tags grouped by panel GUID.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn active_inbounds_by_guid(self) -> Result<ActiveInboundsByGuid> {
        self.post_object::<ActiveInboundsByGuid, ()>("activeInbounds", None)
            .await
    }

    /// Returns last-online timestamps keyed by client email.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn last_online(self) -> Result<LastOnlineByEmail> {
        self.post_object::<LastOnlineByEmail, ()>("lastOnline", None)
            .await
    }

    /// Lists all client groups with counts and traffic since group reset.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn groups(self) -> Result<Vec<GroupSummary>> {
        self.get_object("groups").await
    }

    /// Lists emails belonging to one group.
    ///
    /// # Errors
    ///
    /// Returns an error when retrieval or decoding fails.
    pub async fn group_emails(self, name: &str) -> Result<Vec<String>> {
        self.get_object(&format!("groups/{}/emails", segment(name)))
            .await
    }

    /// Creates an empty group placeholder.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, creation, or decoding fails.
    pub async fn create_group(self, name: &str) -> Result<GroupName> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
        }

        self.post_object("groups/create", Some(&Body { name }))
            .await
    }

    /// Renames a group and all client memberships.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, rename, or decoding fails.
    pub async fn rename_group(self, old_name: &str, new_name: &str) -> Result<AffectedCount> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            old_name: &'a str,
            new_name: &'a str,
        }

        self.post_object("groups/rename", Some(&Body { old_name, new_name }))
            .await
    }

    /// Deletes a group while retaining its clients.
    ///
    /// # Errors
    ///
    /// Returns an error when deletion or decoding fails.
    pub async fn delete_group(self, name: &str) -> Result<AffectedCount> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
        }

        self.post_object("groups/delete", Some(&Body { name }))
            .await
    }

    /// Resets a group's traffic baseline without resetting client counters.
    ///
    /// This source-defined v3.6.0 endpoint is absent from upstream `OpenAPI`.
    ///
    /// # Errors
    ///
    /// Returns an error when reset or decoding fails.
    pub async fn reset_group_traffic(self, name: &str) -> Result<GroupName> {
        #[derive(Serialize)]
        struct Body<'a> {
            name: &'a str,
        }

        self.post_object("groups/resetTraffic", Some(&Body { name }))
            .await
    }

    /// Adds multiple clients to a group, creating it if needed.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, update, or decoding fails.
    pub async fn add_to_group(self, emails: &[String], group: &str) -> Result<AffectedCount> {
        #[derive(Serialize)]
        struct Body<'a> {
            emails: &'a [String],
            group: &'a str,
        }

        self.post_object("groups/bulkAdd", Some(&Body { emails, group }))
            .await
    }

    /// Removes the group label from multiple clients.
    ///
    /// # Errors
    ///
    /// Returns an error when update or decoding fails.
    pub async fn remove_from_group(self, emails: &[String]) -> Result<AffectedCount> {
        self.post_object("groups/bulkRemove", Some(&EmailsBody { emails }))
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

    async fn post_object<T, B>(self, suffix: &str, body: Option<&B>) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let path = format!("{ROOT}/{suffix}");
        let envelope = self
            .client
            .execute(Method::POST, &path, body, AuthenticationScope::PanelApi)
            .await?;
        self.required_object(Method::POST, &path, envelope)
    }

    async fn post_status<B>(self, suffix: &str, body: &B) -> Result<ClientMutationStatus>
    where
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
        Ok(envelope.obj.unwrap_or_default())
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InboundIdsBody<'a> {
    inbound_ids: &'a [i64],
}

#[derive(Serialize)]
struct EmailsBody<'a> {
    emails: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkAttachmentBody<'a> {
    emails: &'a [String],
    inbound_ids: &'a [i64],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageQuery {
    page: u32,
    page_size: u16,
    #[serde(skip_serializing_if = "String::is_empty")]
    search: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    filter: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    protocol: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    inbound: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<&'static str>,
    order: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry_from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry_to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_renew: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_tg_id: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_comment: Option<&'static str>,
    #[serde(skip_serializing_if = "String::is_empty")]
    group: String,
}

impl From<&ClientPageRequest> for PageQuery {
    fn from(request: &ClientPageRequest) -> Self {
        Self {
            page: request.page,
            page_size: request.page_size,
            search: request.search.clone(),
            filter: request
                .statuses
                .iter()
                .map(|status| status.as_str())
                .collect::<Vec<_>>()
                .join(","),
            protocol: request
                .protocols
                .iter()
                .map(|protocol| protocol.as_str())
                .collect::<Vec<_>>()
                .join(","),
            inbound: join_numbers(&request.inbound_ids),
            sort: request.sort.map(super::models::ClientSort::as_str),
            order: request.order.as_str(),
            expiry_from: request.expiry_from,
            expiry_to: request.expiry_to,
            usage_from: request.usage_from,
            usage_to: request.usage_to,
            auto_renew: request
                .auto_renew
                .map(|value| if value { "on" } else { "off" }),
            has_tg_id: request
                .has_telegram_id
                .map(|value| if value { "yes" } else { "no" }),
            has_comment: request
                .has_comment
                .map(|value| if value { "yes" } else { "no" }),
            group: request.groups.join(","),
        }
    }
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn join_numbers(values: &[i64]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientSort, ClientStatusFilter, InboundProtocol, SortOrder};

    #[test]
    fn path_segments_encode_reserved_and_unicode_characters() {
        assert_eq!(
            segment("alice+ops@example.com/a b"),
            "alice%2Bops%40example%2Ecom%2Fa%20b"
        );
        assert_eq!(segment("группа"), "%D0%B3%D1%80%D1%83%D0%BF%D0%BF%D0%B0");
    }

    #[test]
    fn page_request_uses_actual_v360_query_vocabulary() {
        let request = ClientPageRequest {
            statuses: vec![ClientStatusFilter::Online, ClientStatusFilter::Expiring],
            protocols: vec![InboundProtocol::Vless],
            inbound_ids: vec![3, 5],
            sort: Some(ClientSort::LastOnline),
            order: SortOrder::Descending,
            auto_renew: Some(true),
            has_telegram_id: Some(false),
            groups: vec!["Tier A".to_owned(), "Internal".to_owned()],
            ..ClientPageRequest::default()
        };
        let value = serde_json::to_value(PageQuery::from(&request)).unwrap();
        assert_eq!(value["filter"], "online,expiring");
        assert_eq!(value["protocol"], "vless");
        assert_eq!(value["inbound"], "3,5");
        assert_eq!(value["sort"], "lastOnline");
        assert_eq!(value["order"], "descend");
        assert_eq!(value["autoRenew"], "on");
        assert_eq!(value["hasTgId"], "no");
        assert_eq!(value["group"], "Tier A,Internal");
    }
}
