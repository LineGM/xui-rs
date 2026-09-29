# Upstream API contract

- Upstream: `MHSanaei/3x-ui`
- Release: `v3.8.5`
- Commit: `7ef22f94c950ff09f0870e2295fa65ad5968742c`
- OpenAPI source: `docs/public/openapi.json`
- OpenAPI SHA-256: `9a24b28541f35a447a2c6c5d17bfe78bd656cbb202d0648ab5726bf8415b7388`
- OpenAPI HTTP operations: 193 (includes the WebSocket upgrade)
- Container index: `sha256:e0f90c10902e0e74f947d5a9efe017b273804477430233bbfc4918542ffe366c`

`3x-ui-v3.8.5.openapi.json` is copied verbatim from the tagged source.
Per-domain route snapshots were checked against the tagged Go routers. The
subscription snapshot also records four conditional source-only operations:
GET/HEAD for `/mihomo/{subid}` and `/clash-legacy/{subid}`. The aliases are
registered only when Clash is enabled and their paths are not occupied by a
configured subscription format.

The separate subscription server exposes twelve GET/HEAD operations, including
the read-only HWID status route. Its HEAD routes now appear in OpenAPI.
The panel runtime document is checked against this snapshot by the disposable
live-test harness; tests normalize placeholder names before comparing routes.

`3x-ui-v3.8.5.websocket-contract.json` records the unchanged cookie-authenticated
`/ws` route, `{type,payload,time}` envelope, limits, and ten hub constants.
OpenAPI now documents nine emitted message types in `x-websocket-events` on
`GET /ws`, instead of four invalid pseudo-operations. The tenth constant,
`clients`, remains reserved without a direct broadcaster. Contract tests check
both the source inventory and the documented nine-event subset.

See [the complete release review](../docs/upgrading-3.8.5.md), including the
199-commit inventory and the provenance of the upstream documentation fixes.
