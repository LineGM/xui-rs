# Client API

`Client::clients()` covers all 46 routes registered and documented by the
v3.7.0 client and group controllers.

| Area | SDK methods |
|---|---|
| Lookup | `list`, `list_paged`, `get`, `get_by_telegram_id`, `traffic` |
| Lifecycle | `create`, `update`, `update_on_inbounds`, `delete` |
| Attachments | `attach`, `detach`, `bulk_attach`, `bulk_detach` |
| Links | `links`, `subscription_links`, `set_external_links` |
| Portability | `export`, `import`, `bulk_create` |
| Bulk maintenance | `bulk_adjust`, `bulk_enable`, `bulk_disable`, `bulk_delete` |
| Traffic, IPs, and devices | reset/update methods, IP/GUID maps, and HWID list/clear/delete |
| Groups | list/create/rename/delete/reset and bulk membership methods |

## Client types and replacement updates

`ClientConfig` is the writable protocol model. `ClientConfig::new(email)`
creates an enabled client with unlimited quota and lets the server generate the
protocol credential appropriate for each target inbound. The writable fields
cover VMess/VLESS, Trojan, Shadowsocks, Hysteria, WireGuard, MTProto,
per-inbound allowed IPs and forwarded ports, HWID limits, and the expanded
v3.7.0 traffic-reset policy.

`ClientRecord` is the canonical database-backed response. The different types
are deliberate: for example, writable WireGuard `allowedIPs` is an array while
the record endpoint exposes the database's comma-separated string.

Update replaces the supplied client fields rather than patching them. Fetch
the client and convert its record before editing:

```rust,no_run
# use xui_rs::Client;
# async fn example(client: &Client) -> xui_rs::Result<()> {
let details = client.clients().get("alice@example.com").await?;
let mut config = details.client.to_config();
config.total_gb += 50 * 1024 * 1024 * 1024;
client.clients().update("alice@example.com", &config).await?;
# Ok(())
# }
```

`update_on_inbounds` sends the v3.7.0 `inboundIds` query filter when only
selected attachments should have their settings JSON rewritten. Canonical
record fields such as group and enabled state remain global; an empty filter
has the server's ordinary unfiltered-update meaning.

## Server-side paging

`ClientPageRequest` models the complete implementation contract, including
fields missing from the OpenAPI parameter list:

- multiple status buckets, protocols, inbound IDs, and groups;
- expiry and used-traffic ranges;
- auto-renew, Telegram-ID, and comment presence filters;
- all nine actual sort keys and both sort directions.

The server defaults to page 1 with 25 rows and caps page size at 200. Summary
counters cover the full client table rather than only the selected page.

## Import, bulk operations, and groups

`export` returns `Vec<ClientCreateRequest>`. `import` performs the endpoint's
required double encoding automatically: the outer JSON has a `data` field whose
value is the JSON text of that array. Existing emails are reported as skipped,
not overwritten.

Bulk methods return typed counts and per-client skip reports. Attachment
results normalize Go's possible `null` slices into empty Rust vectors. Group
traffic reset moves the group's accounting baseline without changing the
underlying client counters.

## HWID devices

`hwid_devices`, `clear_hwid_devices`, and `delete_hwid_device` expose the
v3.7.0 device-limit controller. Returned hardware identifiers are redacted
from `Debug`; the explicit field remains available for administration.

## Paths and secrets

All user-controlled emails, subscription IDs, and group names are percent
encoded as single path segments. A slash inside an identifier therefore cannot
change the requested route or escape a custom panel base path.

Protocol IDs, passwords, Hysteria auth, WireGuard private/pre-shared keys,
MTProto secrets, subscription IDs, advertisement tags, and external-link URLs
are redacted from `Debug`. They remain explicitly accessible and serializable
when an application intentionally needs to update or export them.
