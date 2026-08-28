# Real-time WebSocket events

`Client::events()` covers the complete real-time protocol registered by 3x-ui
v3.6.0. The endpoint is the panel base path plus `/ws`, so custom secret paths
are handled in the same way as HTTP API routes.

## Authentication and the shared cookie jar

The upstream WebSocket controller calls `session.IsLogin` before upgrading the
connection. It does not run the `/panel/api/*` bearer-token middleware, so an
API token by itself cannot authenticate `/ws`.

Call `AuthApi::login` before connecting:

```rust,no_run
# use xui_rs::{Client, LoginRequest};
# async fn example() -> xui_rs::Result<()> {
let client = Client::new("https://panel.example.com/secret/")?;
client
    .auth()
    .login(LoginRequest::new("admin", "password"))
    .await?;

let mut events = client.events().connect().await?;
# events.close().await?;
# Ok(())
# }
```

HTTP login and the WebSocket handshake query one shared
`reqwest::cookie::Jar`. The jar applies domain, path, secure, expiry, and
replacement rules and produces only the `Cookie` header appropriate for the
exact `/ws` URL. The SDK never copies a full `Set-Cookie` header, parses cookie
strings itself, exposes cookie values, or sends a configured bearer token in
the handshake.

A client may have both a bearer token and a cookie session: use the token for
panel APIs, explicitly call `login` once for events, and the WebSocket still
sends only its matching cookie.

## Receiving events

`EventStream` implements `futures_util::Stream<Item = xui_rs::Result<PanelEvent>>`
and also offers `next_event` when importing `StreamExt` is undesirable:

```rust,no_run
# use xui_rs::{Client, PanelEventKind};
# async fn example(client: &Client) -> xui_rs::Result<()> {
let mut events = client.events().connect().await?;
while let Some(event) = events.next_event().await? {
    match event.kind {
        PanelEventKind::Status(status) => {
            println!("CPU {:.1}%; Xray {:?}", status.cpu, status.xray.state);
        }
        PanelEventKind::Invalidate(invalidation) => {
            println!("refresh {} through HTTP", invalidation.target.as_str());
        }
        _ => {}
    }
}
# Ok(())
# }
```

Every message carries the server-generated Unix timestamp in
`PanelEvent::timestamp_ms`. Ping/pong control frames are handled internally.
The SDK accepts at most 10 MiB per message, matching the hub's own outbound
limit, and rejects binary application frames because v3.6.0 sends JSON text
only.

3x-ui currently discards every client application frame and applies a 512-byte
read limit. Accordingly, the SDK intentionally exposes no arbitrary `send`
operation; only a standards-compliant close frame can be written.

## Complete message inventory

| Wire type | `PanelEventKind` | Payload semantics |
|---|---|---|
| `status` | `Status` | Complete `ServerStatus` snapshot every two seconds |
| `traffic` | `Traffic` | Partial local or remote-node live deltas |
| `inbounds` | `Inbounds` | Complete inbound-list replacement |
| `outbounds` | `Outbounds` | Complete outbound-traffic replacement |
| `nodes` | `Nodes` | Complete node-tree replacement |
| `notification` | `Notification` | Title, body, and typed severity |
| `xray_state` | `XrayState` | Xray `running`/`stop`/`error` transition |
| `client_stats` | `ClientStats` | Absolute client and inbound counters |
| `clients` | `Clients` | Reserved source constant with no v3.6.0 direct broadcaster |
| `invalidate` | `Invalidate` | Resource that must be refreshed over HTTP |

Unknown message names are preserved as `PanelEventKind::Unknown` with their
exact JSON payload, allowing applications to observe events added by newer
panels before upgrading the SDK. The reserved `clients` payload and unknown
payloads remain open-ended because v3.6.0 defines no stable structure for
them.

The published OpenAPI describes only `status`, `notification`, `invalidate`,
and a stale `xrayState` example. Its examples also use `data` instead of the
actual envelope field `payload`. The SDK contract snapshot follows the tagged
Go hub and broadcasters: the wire name is `xray_state`, the envelope is
`{type,payload,time}`, and all ten source message names are covered.

## Traffic and invalidation semantics

`TrafficUpdate` is deliberately partial. Local and remote-node jobs publish
independently:

- `traffics` contains local Xray deltas;
- `node_traffics` contains remote-node inbound deltas;
- `client_traffics` contains clients that moved bytes in the latest local
  collection window;
- online/active maps and last-online timestamps accompany both forms.

An absent optional delta list means that the other job produced this event.
For `node_traffics`, `Some(vec![])` is distinct from absence: it tells a
consumer to clear stale remote-node speeds. Counters in `ClientStatsUpdate`
are absolute. `snapshot == true` means `clients` replaces the complete
collection; `false` means it contains active rows only.

Large or structurally changed resources arrive as `Invalidate`. Re-fetch the
target through the typed HTTP API rather than treating invalidation as a data
payload. The current source emits `inbounds` and `clients` targets, while the
forward-compatible enum preserves any future target.

## Close and reconnect

`next_event` returns `Ok(None)` after a normal close frame and records its code
and reason in `close_info`. Abrupt transport/protocol failures are errors,
because one or more state-changing events may have been lost.

There is intentionally no hidden automatic reconnect and the server does not
replay missed events. After an unexpected end:

1. refresh the HTTP snapshots relevant to the application;
2. re-authenticate if the error is `Error::Unauthorized`;
3. call `EventStream::reconnect`.

Calling `reconnect` while the current socket is active is rejected. `close`
sends code 1000 with a fixed SDK reason and is idempotent after termination.

Notification text, Xray error details, opaque payloads, inbound credentials,
client UUID/subscription IDs, connection path, cookie jar, and socket internals
are summarized or redacted from `Debug`. Explicit fields remain available when
an application intentionally needs their contents.
