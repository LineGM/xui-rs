# Live 3x-ui testing

The ordinary test suite is deterministic and uses protocol-level local test
servers. Before release, the SDK is also exercised against the official 3x-ui
v3.7.0 container:

```console
scripts/live-test.sh
```

The harness pulls the official multi-architecture image by immutable digest,
verifies the reported panel version, initializes a fresh SQLite database, and
publishes the panel only on a random loopback port. The container has no
Linux capabilities, cannot gain privileges, and has CPU, memory, and process
limits. Fail2ban is disabled because the disposable panel is not exposed to the
network.

Both the container and its uniquely named Docker volume are removed by an exit
trap, including after a test failure or interrupt. Container logs are printed
on failure. Do not store anything valuable in a resource carrying the
`io.xui-rs.live-test=true` label.

## What the harness proves

The ignored `tests/live.rs` target checks:

- the real CSRF, username/password login, shared HTTP/WebSocket cookie, logout,
  and post-logout rejection lifecycle;
- settings, server status, runtime OpenAPI, full/slim inbound lists, and an
  authenticated WebSocket handshake;
- API-token creation, bearer authentication, listing, and deletion;
- a disabled VLESS inbound create/read/update/list/options/reset/delete round
  trip, with best-effort cleanup before the container is destroyed.

Mutation tests refuse to start unless `XUI_LIVE_ALLOW_MUTATION=1` is set. The
script supplies this only for its disposable panel.

## Testing another disposable panel

Read-only and cookie/WebSocket checks can be run directly when the target is
exactly 3x-ui v3.7.0 and the account does not require 2FA:

```console
export XUI_LIVE_BASE_URL=https://panel.example.com/secret/
export XUI_LIVE_USERNAME=integration-user
export XUI_LIVE_PASSWORD=replace-me
export XUI_LIVE_EXPECTED_VERSION=3.7.0
cargo test --locked --test live live_cookie_http_and_websocket_smoke \
  -- --ignored --nocapture
```

Only enable the mutation test for a disposable panel. It creates and deletes
real database rows:

```console
XUI_LIVE_ALLOW_MUTATION=1 cargo test --locked --test live \
  live_token_and_inbound_round_trip_with_cleanup -- --ignored --nocapture
```

The repository's **3x-ui live** GitHub Actions workflow runs the complete
container harness manually and every Monday. A normal `cargo test` compiles the
live target but keeps both tests ignored.

## Troubleshooting

Docker, curl, and the pinned Rust toolchain must be available. Docker must be
able to pull from GitHub Container Registry and bind a loopback port. On
failure, inspect the emitted panel log before the trap removes the container;
the harness never prints its generated password or API token.
