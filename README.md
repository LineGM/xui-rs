<div align="center">

# xui-rs

**A modern, typed async Rust SDK for 3x-ui**

[![CI](https://github.com/LineGM/xui-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/ci.yml)
[![Coverage](https://coveralls.io/repos/github/LineGM/xui-rs/badge.svg?branch=main)](https://coveralls.io/github/LineGM/xui-rs?branch=main)
[![MSRV](https://img.shields.io/badge/MSRV-1.85.0-dea584.svg)](https://www.rust-lang.org)
[![3x-ui](https://img.shields.io/badge/3x--ui-v3.6.0-0ea5e9.svg)](https://github.com/MHSanaei/3x-ui/releases/tag/v3.6.0)
[![License](https://img.shields.io/badge/license-Unlicense-blue.svg)](LICENSE)

</div>

> [!IMPORTANT]
> The `0.2` line is an active ground-up rewrite targeting the complete 3x-ui
> v3.6.0 API. The authentication and transport foundation is ready; domain API
> modules are being added incrementally. Do not expect compatibility with 0.1.

## Why xui-rs?

- Strong request and response types instead of unstructured JSON.
- A cheap-to-clone client designed for concurrent async workloads.
- API-token and cookie-session authentication with secrets redacted by default.
- Correct base-path handling for panels installed below a custom URL prefix.
- Contract tests derived from the upstream 3x-ui API surface.
- Strict formatting, linting, documentation, MSRV, and cross-platform CI gates.

## Authentication

API tokens are the recommended choice for services, bots, and automation. They
avoid browser-session state and 3x-ui's CSRF flow.

```rust,no_run
use xui_rs::Client;

let client = Client::builder("https://panel.example.com/secret/")?
    .bearer_token(std::env::var("XUI_API_TOKEN")?)
    .build()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Cookie login remains available for panels where an API token cannot be used.
The SDK obtains a pre-auth CSRF token, stores only cookie name/value pairs in a
standards-compliant jar, and reuses the CSRF token for unsafe requests.

```rust,no_run
use xui_rs::{Client, LoginRequest};

# async fn example() -> xui_rs::Result<()> {
let client = Client::new("https://panel.example.com/secret/")?;
let login = LoginRequest::new("admin", "password")
    .with_two_factor_code("123456");

client.auth().login(login).await?;
client.auth().logout().await?;
# Ok(())
# }
```

Credentials are never retained for automatic re-login. An expired session is
reported as [`Error::Unauthorized`](https://docs.rs/xui-rs/latest/xui_rs/enum.Error.html),
allowing the application to decide how and when credentials may be requested.
See [the authentication design](docs/authentication.md) for the rationale.

## Inbounds

The complete v3.6.0 inbound surface is available through `Client::inbounds`,
including the source-only `allLinks` route omitted from upstream OpenAPI.

```rust,no_run
use xui_rs::{Client, InboundConfig, InboundProtocol};

# async fn example() -> xui_rs::Result<()> {
let client = Client::builder("https://panel.example.com/secret/")?
    .bearer_token("api-token")
    .build()?;

let mut config = InboundConfig::new(InboundProtocol::Vless, 443);
config.remark = "production-vless".into();
config.settings = serde_json::json!({ "clients": [] });
config.stream_settings = serde_json::json!({
    "network": "tcp",
    "security": "reality"
});

let created = client.inbounds().create(&config).await?;

// Update is a full replacement in 3x-ui, so preserve the fetched state.
let mut edited = created.to_config();
edited.remark = "renamed-vless".into();
client.inbounds().update(created.id, &edited).await?;
# Ok(())
# }
```

See [the inbound API guide](docs/inbounds.md) for the operation inventory,
replacement/import semantics, and intentionally open-ended Xray JSON fields.

## Clients

`Client::clients()` covers all 43 client and group routes registered by v3.6.0,
including server-side paging, portable import/export, bulk operations, online
and IP attribution, and group traffic baselines.

```rust,no_run
use xui_rs::{
    Client, ClientConfig, ClientCreateRequest, ClientPageRequest,
    ClientStatusFilter,
};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let page = client
    .clients()
    .list_paged(&ClientPageRequest {
        statuses: vec![ClientStatusFilter::Online],
        ..ClientPageRequest::default()
    })
    .await?;

let config = ClientConfig::new("alice@example.com");
let request = ClientCreateRequest::new(config, vec![7, 9]);

// Uncomment only when creation is intended.
// client.clients().create(&request).await?;
println!("{} online clients", page.summary.online_count);
# Ok(())
# }
```

Every email, subscription ID, and group name is encoded as one URL path
segment. Client credentials, subscription IDs, external links, and private
keys are redacted from `Debug`. See [the client API guide](docs/clients.md).

## Server and Xray

`Client::server()` covers all 38 routes registered by the v3.6.0 server
controller: typed host/Xray status and history, observatory data, lifecycle and
updates, logs, cryptographic helpers, REALITY target scanning, database
backup/restore, and cluster IP synchronization.

```rust,no_run
use xui_rs::{Client, HistoryBucket, SystemMetric};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let status = client.server().status().await?;
let cpu = client
    .server()
    .system_history(SystemMetric::Cpu, HistoryBucket::Hour1)
    .await?;

println!("Xray {:?}; {} CPU samples", status.xray.state, cpu.len());
# Ok(())
# }
```

Database downloads stay in memory and preserve attachment metadata. Restore,
self-update, and Xray lifecycle methods are explicit operations and never run
implicitly. Generated private material is redacted from `Debug`. See [the
server API guide](docs/server.md).

## Panel and Xray settings

`Client::settings()` covers all 14 v3.6.0 panel-settings routes, including API
tokens, notification tests, credential replacement, and the two source-only
factory-default and regex-validation operations. `Client::xray_settings()`
covers all 21 Xray settings, WARP/NordVPN, outbound-test, routing-test, and
remote outbound-subscription routes.

```rust,no_run
use xui_rs::{Client, PanelSettingsUpdate};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let view = client.settings().all().await?;
let mut update = PanelSettingsUpdate::new(view.settings);
update.settings.display.page_size = 100;

// This endpoint is a full replacement; update the fetched settings object.
client.settings().update(&update).await?;
# Ok(())
# }
```

The ergonomic grouped settings model flattens to exact upstream wire names.
Nested Xray JSON is encoded and decoded automatically. Stored notification and
LDAP secrets, integration payloads, subscription URLs/outbounds, and API-token
plaintext are redacted from `Debug`. See [the settings API guide](docs/settings.md).

## Subscription hosts

`Client::hosts()` covers all 12 host-override routes registered by v3.6.0,
including the source-only bulk-create alias. Logical groups span multiple
inbounds and addresses, while create/update results expose every physical row
produced by that Cartesian expansion.

```rust,no_run
use xui_rs::{Client, HostGroup, HostSecurity};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let mut group = HostGroup::new(vec![7, 9], "production CDN");
group.hosts = vec!["cdn.example.com".into()];
group.options.port = 443;
group.options.security = HostSecurity::Tls;

// Creation is explicit and persistent.
// client.hosts().create(&group).await?;
# Ok(())
# }
```

Nested mux, sockopt, and final-mask JSON is encoded automatically, path values
are segment-safe, and the source's nullable empty list is normalized. See [the
Hosts API guide](docs/hosts.md).

## Compatibility

| xui-rs | 3x-ui | Status |
|---|---|---|
| `0.2.x` | `3.6.0` | Auth, inbounds, clients, server, settings/Xray, and Hosts APIs |
| `0.1.x` | legacy API | Superseded |

Rust 1.85.0 is the minimum supported compiler. Development and CI use the
pinned Rust 1.98.0 toolchain.

## Development

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines and
[SECURITY.md](SECURITY.md) for private vulnerability reporting.

## License

Released into the public domain under the [Unlicense](LICENSE).
