# Public subscriptions and panel-wide operations

3x-ui serves subscriptions from a separate HTTP/HTTPS listener. It may use a
different host, port, TLS certificate, and path set than the authenticated
panel. `SubscriptionClient` therefore owns an independent transport and never
sends panel bearer tokens, cookies, or CSRF headers.

| Route family | SDK methods |
|---|---|
| Raw subscription | `raw`, `html`, `info`, `info_with_metadata`, `raw_metadata` |
| Xray JSON | `json`, `json_metadata` |
| Clash/Mihomo YAML | `clash`, `clash_metadata` |

The three metadata methods use the source-defined `HEAD` routes. They retrieve
traffic, expiry, profile, routing, and content headers without downloading a
credential-bearing body.

## Constructing the public client

The defaults are `/sub/`, `/json/`, and `/clash/` below the supplied server
origin. Deployments with custom paths can configure each prefix explicitly.

```rust
use xui_rs::SubscriptionClient;
let subscriptions = SubscriptionClient::builder("https://sub.example.com:2096")?
    .raw_path("public/raw/")
    .json_path("public/json/")
    .clash_path("public/mihomo/")
    .build()?;
Ok::<(), xui_rs::Error>(())
```

When an authenticated panel client is already available, the public addresses
can be derived from its settings snapshot:

```rust
use xui_rs::{Client, SubscriptionClient};
async fn example(panel: &Client) -> xui_rs::Result<()> {
let settings = panel.settings().all().await?;
let subscriptions = SubscriptionClient::from_settings(&settings.settings.subscription)?;
let metadata = subscriptions.raw_metadata("secret-subscription-id").await?;
println!("traffic: {:?}", metadata.traffic);
Ok(())
}
```

`from_settings` uses the absolute public URIs supplied by 3x-ui and falls back
to the configured JSON/Clash paths when their absolute URIs are empty. Invalid
TLS certificates remain rejected unless the builder's explicit `danger_*`
option is enabled.

## Documents and content negotiation

`raw` asks for plain text with a neutral SDK user agent, avoiding 3x-ui's
client-specific format auto-detection. Its body is standard base64 when the
panel setting `subEncrypt` is enabled:

```rust
use xui_rs::SubscriptionClient;
async fn example(subscriptions: &SubscriptionClient) -> xui_rs::Result<()> {
let response = subscriptions.raw("secret-subscription-id").await?;
let document = response.content.decode_base64()?;
println!("{} subscription entries", document.lines().count());
Ok(())
}
```

Use `decode_base64` only for encrypted raw subscriptions; plaintext documents
can be consumed directly with `as_str`, `into_string`, or `lines`. `json`
parses the exact Xray JSON document into a deliberately open-ended
`serde_json::Value`, while `clash` preserves the exact YAML text. Both methods
force 3x-ui's raw-download view so browser HTML negotiation cannot replace the
document. `html` explicitly retrieves that browser information page instead.

`info` decodes the source's typed `format=info` JSON view. It includes status,
human-readable and byte traffic totals, expiry, last-online time, represented
emails, and generated public links.

## HWID device identity

v3.7.0 can limit a subscription to registered devices. Configure
`SubscriptionDevice` on the builder to send the source-defined `X-HWID`,
device OS/version/model, and user-agent headers. HWIDs shorter than six bytes
are rejected before any request is sent. `info_with_metadata` returns the
typed info document and response metadata together.

`SubscriptionMetadata` exposes whether the device was registered, already
known, over the limit, or whether device limiting is disabled. Hardware IDs
are redacted from `Debug` and are never sent to the authenticated panel API.

## Metadata and secret handling

`SubscriptionMetadata` parses `Subscription-Userinfo`, base64 profile title
and announcement headers, polling interval, routing flags, content type, and
download disposition. Profile-page URLs and routing rules have explicit
accessors because they can contain credentials or sensitive policy.

Subscription IDs are encoded as exactly one URL path segment. The ID is
replaced with `[REDACTED]` in transport, status, and decode errors. Response
bodies, parsed JSON, profile URLs, routing rules, generated links, and client
emails are also redacted from `Debug`; explicit accessors and public fields are
available when an application intentionally needs them. Applications should
apply the same care when logging returned documents themselves.

## Panel-wide routes

The two HTTP routes outside the domain controllers live under
`Client::panel()`:

- `openapi` returns the runtime OpenAPI JSON directly, including the connected
  panel's configured base path. `OpenApiDocument` preserves the full document
  and can count real HTTP operations without documentation-only WebSocket
  message entries.
- `backup_to_telegram` asks 3x-ui to generate a fresh database backup and send
  it to configured Telegram administrators. The upstream handler returns only
  an empty successful response and exposes no per-recipient result.

Backup delivery is an explicit mutating operation and is never triggered by
client construction or any read method.
