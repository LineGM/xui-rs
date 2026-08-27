# Inbound API

`Client::inbounds()` covers every inbound route registered by 3x-ui v3.6.0:

| SDK method | Upstream operation |
|---|---|
| `list`, `list_slim`, `options`, `all_links`, `get` | Read and share-link views |
| `create`, `update`, `delete`, `delete_many` | Lifecycle management |
| `set_enabled` | Lightweight enable switch |
| `reset_traffic`, `reset_all_traffic` | Traffic reset operations |
| `delete_all_clients` | Optimized bulk client deletion |
| `import` | Portable form-based import |
| `fallbacks`, `set_fallbacks` | Atomic fallback configuration |
| `push_client_traffic` | Master-panel traffic synchronization |

Upstream OpenAPI documents 16 of these 17 routes. `GET
/panel/api/inbounds/allLinks` is registered in `inbound.go` but absent from the
published document. The SDK contract test checks the OpenAPI operations and a
separate source-derived route snapshot, so neither surface is silently lost.

## Create and update

`InboundConfig::new(protocol, port)` supplies the normalization defaults used
by the panel: enabled, unlimited traffic, one-based subscription sorting,
`never` traffic reset, day 1, and node-based share addressing.

3x-ui's update handler is a replacement operation despite using a `POST`
route. It copies every writable field from the payload. Fetch the record and
call `Inbound::to_config()` before changing fields to avoid dropping settings
or resetting limits:

```rust,no_run
# use xui_rs::Client;
# async fn example(client: &Client, id: i64) -> xui_rs::Result<()> {
let inbound = client.inbounds().get(id).await?;
let mut config = inbound.to_config();
config.enable = false;
client.inbounds().update(id, &config).await?;
# Ok(())
# }
```

For enable switches, prefer `set_enabled`; the dedicated upstream endpoint
does not resend potentially thousands of client settings.

## Xray JSON

`settings`, `stream_settings`, and `sniffing` remain `serde_json::Value` on
purpose. Their shape depends on the selected Xray protocol, transport,
security layer, and installed Xray version; pretending they have one closed
schema would reject valid panel configurations. All surrounding panel models
and response envelopes are strongly typed.

These fragments can contain client credentials and private keys. Their values,
along with client UUID and subscription IDs, are redacted from `Debug` output.

## Import and traffic synchronization

`import` sends the form-encoded JSON format required by the panel and preserves
the supplied `client_stats` rows while the server replaces panel-local IDs.
This differs from an ordinary create and is why it has a separate SDK method.

`push_client_traffic` accepts only the fields the receiving v3.6.0 service
actually consumes: master GUID, client email, upload bytes, and download
bytes. Unknown clients are ignored by the panel and each master's latest
snapshot replaces its previous values.
