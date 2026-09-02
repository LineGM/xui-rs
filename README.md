<div align="center">

# xui-rs

**A modern, typed async Rust SDK for 3x-ui**

[![CI](https://github.com/LineGM/xui-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/ci.yml)
[![3x-ui live](https://github.com/LineGM/xui-rs/actions/workflows/live.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/live.yml)
[![Coverage](https://coveralls.io/repos/github/LineGM/xui-rs/badge.svg?branch=main)](https://coveralls.io/github/LineGM/xui-rs?branch=main)
[![crates.io](https://img.shields.io/crates/v/xui-rs.svg)](https://crates.io/crates/xui-rs)
[![docs.rs](https://docs.rs/xui-rs/badge.svg)](https://docs.rs/xui-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.88.0-dea584.svg)](https://www.rust-lang.org)
[![3x-ui](https://img.shields.io/badge/3x--ui-v3.7.0-0ea5e9.svg)](https://github.com/MHSanaei/3x-ui/releases/tag/v3.7.0)
[![License](https://img.shields.io/badge/license-Unlicense-blue.svg)](LICENSE)

</div>

> [!IMPORTANT]
> The `1.0` line targets the complete 3x-ui v3.7.0 HTTP and WebSocket API. It is
> a ground-up replacement for the legacy 0.1 client and is not source-compatible
> with it.

## Why xui-rs?

- Strong request and response types instead of unstructured JSON.
- A cheap-to-clone client designed for concurrent async workloads.
- API-token and cookie-session authentication with secrets redacted by default.
- Correct base-path handling for panels installed below a custom URL prefix.
- Contract tests derived from the upstream 3x-ui API surface.
- A reviewed rustdoc API baseline and exhaustive downstream re-export contract.
- RustSec, license, dependency-source, package, and SemVer release gates.
- A typed, forward-compatible real-time stream with explicit reconnect semantics.
- One explicit HTTP/HTTPS/SOCKS5 proxy configuration shared by HTTP,
  subscriptions, and WebSocket transports.
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

The complete 18-route v3.7.0 inbound surface is available through
`Client::inbounds`, including AmneziaWG settings and subscription sort order.

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

`Client::clients()` covers all 46 client and group routes registered by v3.7.0,
including server-side paging, portable import/export, bulk operations, online
and IP attribution, HWID device management, and group traffic baselines.

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

`Client::server()` covers all 39 routes registered by the v3.7.0 server
controller: typed host/Xray status and history, observatory data, lifecycle and
updates, logs, cryptographic helpers, REALITY target scanning, database
backup/restore, cluster IP synchronization, and AmneziaWG peer diagnostics.

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

`Client::settings()` covers all 14 v3.7.0 panel-settings routes, including
scoped and expiring API tokens, notification tests, and credential replacement.
`Client::xray_settings()` covers all 26 Xray settings and integration routes,
including WARP, NordVPN, PIA, geodata inspection, outbound and routing tests,
and remote outbound subscriptions.

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

`Client::hosts()` covers all 12 host-override routes registered by v3.7.0.
Logical groups span multiple inbounds and addresses, while create/update
results expose every physical row produced by that Cartesian expansion.

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

## Remote nodes

`Client::nodes()` covers all 16 v3.7.0 node routes: registration and lifecycle,
saved/unsaved probes, remote inbound discovery, health history, bulk panel
updates, certificate pinning, node mTLS, and live mTLS-client reload.

```rust,no_run
use xui_rs::{Client, NodeRequest};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let request = NodeRequest::new("edge-de", "node.example.com", 2053)
    .with_api_token(std::env::var("NODE_API_TOKEN").unwrap());

let probe = client.nodes().test_connection(&request).await?;
println!("candidate node is {:?}", probe.status);

// Registration is explicit because it persists and starts synchronization.
// client.nodes().create(&request).await?;
# Ok(())
# }
```

Read models never expose write-only node API tokens. Credential retain/replace/
clear states are mutually exclusive, legacy nullable tags are normalized, and
unknown enum/protocol values remain forward-compatible. See [the Nodes API
guide](docs/nodes.md).

## Public subscriptions and panel metadata

`SubscriptionClient` covers all six routes exposed by 3x-ui's separate public
subscription server: `GET` and `HEAD` for raw, Xray JSON, and Clash/Mihomo
formats. It intentionally carries no panel authentication because this server
can run on another origin and port.

```rust,no_run
use xui_rs::SubscriptionClient;

# async fn example() -> xui_rs::Result<()> {
let subscriptions = SubscriptionClient::new("https://sub.example.com:2096")?;
let metadata = subscriptions
    .raw_metadata("secret-subscription-id")
    .await?;

println!("profile: {:?}; traffic: {:?}", metadata.profile_title, metadata.traffic);
# Ok(())
# }
```

Custom raw/JSON/Clash paths, construction from panel settings, and the v3.7.0
HWID/device headers are supported. Successful responses expose the panel's
HWID registration, limit, and device-existence metadata.
Subscription identifiers, documents, generated links, routing rules, and
represented emails are redacted from `Debug` and request errors. The raw body
can be decoded through `SubscriptionDocument::decode_base64` when `subEncrypt`
is enabled.

`Client::panel()` adds the two remaining panel-wide HTTP operations: retrieve
the runtime OpenAPI document and explicitly send a fresh database backup to
configured Telegram administrators. See [the public subscription and
panel-wide operations guide](docs/subscriptions.md).

## Subscription balancers

`Client::subscription_balancers()` covers the complete v3.7.0 controller for
JSON-subscription balancers: list, create, replace, and both delete transports.
Strategies are typed, inbound IDs preserve the upstream repeated-form wire
contract, and unknown future strategy values fail closed for mutations.

```rust,no_run
use xui_rs::{Client, SubscriptionBalancerInput, SubscriptionBalancerStrategy};

# async fn example(client: &Client) -> xui_rs::Result<()> {
let mut input = SubscriptionBalancerInput::new("EU pool", vec![7, 9]);
input.strategy = SubscriptionBalancerStrategy::LeastPing;

// Creation is explicit and persistent.
// client.subscription_balancers().create(&input).await?;
# Ok(())
# }
```

## Outbound proxies

Panel HTTP, public subscriptions, and WebSocket connections support the same
typed `ProxyConfig`. HTTP, HTTPS, SOCKS5 with local DNS, and `socks5h` with
proxy-side DNS are supported, including Basic/username-password authentication.

```rust,no_run
use xui_rs::{Client, ProxyConfig};

# fn example() -> xui_rs::Result<()> {
let proxy = ProxyConfig::new("socks5h://proxy.example.com:1080")?
    .with_basic_auth("service", "proxy-password")?;
let client = Client::builder("https://panel.example.com/secret/")?
    .proxy(proxy)
    .build()?;
# let _ = client;
# Ok(())
# }
```

Proxy credentials are separate from the credential-free URL and redacted from
`Debug` and errors. Environment proxy variables are intentionally ignored, so
HTTP and WebSocket routing stays deterministic. Cookies remain scoped to the
target panel and continue to use the same standards-compliant jar through the
proxy. See [the outbound proxy guide](docs/proxies.md) for DNS, TLS, timeout,
and security behavior.

## Transport safety and observability

Response bodies are bounded in memory even when a server uses chunked transfer
encoding or reports an incorrect length. Ordinary panel API and public
subscription responses default to 64 MiB; explicit database downloads have an
independent 512 MiB default. All three limits are configurable.

```rust,no_run
use xui_rs::{Client, SubscriptionClient};

# fn example() -> xui_rs::Result<()> {
let panel = Client::builder("https://panel.example.com/secret/")?
    .response_body_limit(16 * 1024 * 1024)
    .download_body_limit(1024 * 1024 * 1024)
    .build()?;
let subscriptions = SubscriptionClient::builder("https://panel.example.com")?
    .response_body_limit(32 * 1024 * 1024)
    .build()?;
# let _ = (panel, subscriptions);
# Ok(())
# }
```

Oversized responses return the typed `ErrorKind::ResponseTooLarge`. Optional
`tracing` events correlate a request with its response headers using a request
ID, method, status, outcome, and latency while deliberately omitting URLs,
paths, queries, headers, cookies, tokens, and bodies. See the
[transport safety and observability guide](docs/transport.md) for exact
semantics and introspection.

## Real-time events

`Client::events()` covers the authenticated `/ws` handshake and all ten
message names declared by the v3.7.0 source. Status, traffic, inbounds,
outbounds, nodes, notifications, Xray transitions, client counters, reserved
clients payloads, and invalidations have distinct typed variants.

```rust,no_run
use xui_rs::{Client, LoginRequest, PanelEventKind};

# async fn example() -> xui_rs::Result<()> {
let client = Client::new("https://panel.example.com/secret/")?;
client
    .auth()
    .login(LoginRequest::new("admin", "password"))
    .await?;

let mut events = client.events().connect().await?;
while let Some(event) = events.next_event().await? {
    if let PanelEventKind::Invalidate(value) = event.kind {
        println!("refresh {} through HTTP", value.target.as_str());
    }
}
# Ok(())
# }
```

The endpoint does not support bearer-token authentication. HTTP and WebSocket
share one standards-compliant cookie jar, but cookie values are never exposed
or copied manually. Control frames and the 10 MiB source limit are handled
internally. Reconnect is explicit because 3x-ui does not replay events missed
during a disconnect. See [the real-time events guide](docs/events.md).

## Errors and retry policy

All operations return `xui_rs::Result<T>`. `ErrorKind` provides a stable,
copyable classification for metrics and policy, while `status()`, `method()`,
`url()`, and authentication/rate-limit/server/timeout helpers expose context
without destructuring variants.

```rust
use xui_rs::Error;

fn action(error: &Error) -> &'static str {
    if error.is_unauthorized() {
        "refresh credentials"
    } else if error.is_rate_limited() {
        "back off"
    } else if error.is_timeout() || error.is_server_error() {
        "reconcile and maybe retry"
    } else {
        "inspect the typed error"
    }
}
```

The SDK never retries API mutations implicitly: a timeout does not prove that
the server failed to apply a request. See [the error and retry guide](docs/errors.md)
for idempotency-aware recommendations and WebSocket recovery semantics.

## API stability

The complete 1.0 Rust surface is recorded in a reproducible rustdoc
snapshot. Downstream contract tests compile every concise crate-root re-export,
important trait guarantee, and representative multithreaded-runtime future.
Extensible enums are non-exhaustive, so matches should retain a wildcard arm.

See the [API stability policy](docs/api-stability.md) for the exact 1.x SemVer
guarantees and the boundary between Rust source compatibility and upstream
3x-ui server behavior.

## Compatibility

| xui-rs | 3x-ui | Status |
|---|---|---|
| `1.0.x` | `3.7.0` | Complete tagged HTTP and WebSocket API |
| `0.2.x` | `3.6.0` | Superseded release-candidate line |
| `0.1.x` | legacy API | Superseded |

Rust 1.88.0 is the minimum supported compiler. Development and CI use the
pinned Rust 1.98.0 toolchain.

## Development

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
actionlint
cargo deny check
scripts/public-api.sh check
scripts/package-check.sh --allow-dirty
```

The opt-in real-panel gate runs the ignored integration target against an
isolated, digest-pinned 3x-ui v3.7.0 container:

```console
scripts/live-test.sh
```

See [the live-testing guide](docs/live-testing.md) for its isolation model,
cookie/WebSocket coverage, mutation guard, and manual target configuration.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines and
[SECURITY.md](SECURITY.md) for private vulnerability reporting. Maintainers
should follow the [OIDC-based release procedure](docs/releasing.md); releases
never require a long-lived crates.io token.

## License

Released into the public domain under the [Unlicense](LICENSE).
