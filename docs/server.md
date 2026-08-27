# Server and Xray API

`Client::server()` covers all 38 routes registered by the 3x-ui v3.6.0 server
controller. Upstream OpenAPI documents 35. The source-only routes are
`getUpdateStatus`, `scanRealityTarget`, and `scanRealityTargets`.

| Area | SDK methods |
|---|---|
| Host state | `status`, `system_history`, `legacy_cpu_history`, `fail2ban_status` |
| Xray metrics | `xray_metrics_state`, `xray_metrics_history`, observatory methods |
| Xray lifecycle | `xray_versions`, `install_xray`, `stop_xray`, `restart_xray` |
| Panel updates | `panel_update_info`, `update_panel`, `panel_update_status`, `set_update_channel` |
| Diagnostics | `panel_logs`, `xray_logs`, `xray_config` |
| Database | `download_database`, `download_migration`, `import_database` |
| Crypto and REALITY | UUID/key/ECH/hash generators and REALITY scan methods |
| Cluster utilities | `descendants`, `client_ips`, `merge_client_ips`, web certificate paths |

## Status and history

`ServerStatus` follows the full Go `service.Status` structure rather than the
smaller OpenAPI example. It includes capacity and IO counters, current and
cumulative network traffic, public addresses, panel process statistics, stable
panel identity, and Xray state.

`SystemMetric` exposes all 16 names accepted by the controller's source
allowlist. `XrayMetric` exposes the five accepted expvar series.
`HistoryBucket` similarly models the exact v3.6.0 allowlist, preventing an
unsupported bucket from reaching the server. Its variants describe the visible
window (`Minutes2` through `Days7`); the wire value is the per-point bucket size
in seconds and each response contains at most 60 points.

The old CPU endpoint returns `LegacyCpuPoint { timestamp, cpu }`. New code
should use `system_history`, whose `MetricPoint { timestamp, value }` shape is
shared by host, Xray, and observatory history.

## Logs and configuration

The upstream examples show both log endpoints as strings, but the controller
returns different real shapes:

- `panel_logs` returns `Vec<String>`;
- `xray_logs` returns `Vec<XrayLogEntry>` with parsed addresses, tags, email,
  timestamp, and a typed direct/blocked/proxied event.

The assembled Xray configuration is an `XrayConfig` wrapper around
`serde_json::Value`. Its schema depends on the installed xray-core version and
protocol extensions, so a closed Rust model would reject valid configurations.
Use `as_value` or `into_value` for explicit access. Its `Debug` output is
redacted because the document can contain credentials and private keys.

## Backup and restore

`download_database` and `download_migration` return `DatabaseFile` in memory,
including the panel's suggested filename and content type. The SDK does not
choose a filesystem destination or overwrite files. It also detects the
HTTP-200 JSON error envelope that 3x-ui emits when generating a download fails.

`import_database(filename, bytes)` sends the required multipart field named
`db`. Import replaces panel state and restarts Xray; callers should treat it as
a destructive operation and retain a verified backup first.

## Updates and service control

`update_panel(None)` uses the configured channel. `Some(false)` and
`Some(true)` explicitly select stable or dev for that run. The returned decimal
`run_id` is a string because the panel derives it from a nanosecond timestamp
that cannot be represented exactly by JavaScript numbers. Poll
`panel_update_status` and match that identifier to avoid accepting stale state.

Installing/restarting/stopping Xray, updating the panel, updating geo files,
and importing a database are never performed implicitly by another SDK call.

## Cryptographic material and paths

The source implementation differs from several OpenAPI examples. The SDK uses
the authoritative v3.6.0 shapes: UUID is `{uuid}`, ML-DSA-65 is `{seed,
verify}`, and ML-KEM-768 is `{seed, client}`. Generated private keys, seeds,
ECH server keys, and VLESS encryption strings are redacted from `Debug`.

User-controlled versions, geo filenames, and observatory tags are percent
encoded as one URL path segment. Server-side validation remains authoritative.
