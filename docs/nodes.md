# Remote nodes

`Client::nodes()` covers all 16 routes registered by the 3x-ui v3.7.0
`NodeController`: registration, connectivity tests, remote inbound discovery,
health/history, panel updates, certificate pinning, and node mTLS.

| Area | SDK methods |
|---|---|
| Inventory | `list`, `get`, `web_certificate_files` |
| Lifecycle | `create`, `update`, `delete`, `set_enabled` |
| Connectivity | `test_connection`, `certificate_fingerprint`, `remote_inbounds`, `probe` |
| Operations | `update_panels`, `history` |
| mTLS | `mtls_ca`, `set_mtls_trust_ca`, `reload_mtls_client` |

## Safe read and write contracts

The tagged controller returns `NodeView`, not the database `Node` described by
the stale upstream OpenAPI schema. In particular, API tokens are write-only:
reads expose `has_api_token` and never contain `apiToken`. `NodeView` covers all
41 fields returned by the source, including health, Xray state, traffic rates,
client counts, dirty configuration state, and multi-hop identity.

`NodeRequest` is the complete write/probe contract. Its constructor supplies
the panel defaults for HTTPS, normal certificate verification, `/` base path,
all-inbound synchronization, and an enabled node.

```rust,no_run
# use xui_rs::{Client, NodeRequest, NodeTlsVerifyMode};
# async fn example(client: &Client) -> xui_rs::Result<()> {
let mut request = NodeRequest::new("edge-de", "node.example.com", 2053)
    .with_api_token(std::env::var("NODE_API_TOKEN").unwrap());
request.remark = "Frankfurt edge".into();
request.tls_verify_mode = NodeTlsVerifyMode::Pin;
request.pinned_cert_sha256 = client
    .nodes()
    .certificate_fingerprint(&request)
    .await?;

let node = client.nodes().create(&request).await?;
println!("registered node {} as {}", node.name, node.guid);
# Ok(())
# }
```

The fingerprint endpoint deliberately makes an unverified bootstrap TLS
connection. Verify the fingerprint through another trusted channel before
pinning it when the node is security-sensitive.

## Updating credentials

Node updates replace every writable connection/synchronization field, but the
API token has three distinct actions:

- `NodeView::to_request()` or `retain_stored_api_token()` omits both credential
  fields and retains the stored token.
- `set_api_token()` sends a replacement token.
- `clear_stored_api_token()` sends only `clearApiToken: true`.

The SDK makes replacement and clearing mutually exclusive. 3x-ui rejects
clearing credentials on an enabled node unless `NodeTlsVerifyMode::Mtls` is
active. A token is required when creating every non-mTLS node.

```rust,no_run
# use xui_rs::Client;
# async fn example(client: &Client) -> xui_rs::Result<()> {
let node = client.nodes().get(7).await?;
let mut request = node.to_request();
request.remark = "renamed edge".into();
client.nodes().update(node.id, &request).await?;
# Ok(())
# }
```

Transitive entries from a chained topology have `transitive == true` and
`id == 0`. They are read-only projections managed from their direct parent,
not valid targets for update/delete operations.

## Probe and remote discovery semantics

`test_connection` probes unsaved details. `probe` probes a stored node and
updates its heartbeat cache. An unreachable remote is a successfully decoded
`NodeProbeResult` with `NodeStatus::Offline` and a friendly `error`; only
validation, transport, envelope, and decoding failures become SDK errors.

`remote_inbounds` returns the remote tags available to
`NodeInboundSyncMode::Selected`. Protocol values are typed but preserve unknown
strings from newer remote panels. Legacy `null` inbound-tag lists in `NodeView`
are normalized to empty vectors.

Private and loopback destinations are blocked by the panel's SSRF guard unless
`allow_private_address` is explicitly enabled. Only enable it for an intended,
trusted private-network node.

## Monitoring and updates

`history` accepts `NodeMetric::{Cpu, Memory, NetworkUp, NetworkDown}` and the
shared `HistoryBucket` type. Results use the same uniform `MetricPoint { t, v }`
shape and 60-point maximum as server history.

`update_panels` targets either `NodeUpdateChannel::Stable` or `Development`.
Missing, disabled, or offline nodes are reported as individual
`NodeUpdateResult { ok: false, error }` entries; they do not fail successful
dispatch to other nodes.

## Node mTLS

`mtls_ca()` returns the managing panel's public node-auth CA and lazily creates
its master client certificate. Call `set_mtls_trust_ca()` through a `Client`
connected to the remote panel, restart that remote panel, then configure the
managing panel's node with `NodeTlsVerifyMode::Mtls`.

```rust,no_run
# use xui_rs::Client;
# async fn example(manager: &Client, remote: &Client) -> xui_rs::Result<()> {
let ca = manager.nodes().mtls_ca().await?;
remote.nodes().set_mtls_trust_ca(&ca.ca_cert).await?;
// Restart `remote`, then select NodeTlsVerifyMode::Mtls on `manager`.
# Ok(())
# }
```

Sending an empty CA disables trust for incoming node client certificates after
restart.

`reload_mtls_client()` reloads the local node client certificate without a
panel restart after the certificate material has changed.

API tokens, remote secret base paths, and internal egress tags are redacted
from `Debug`; explicit fields/accessors and serialization remain available for
intentional use.
