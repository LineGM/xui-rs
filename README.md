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

## Compatibility

| xui-rs | 3x-ui | Status |
|---|---|---|
| `0.2.x` | `3.6.0` | Active rewrite |
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
