# Upstream API contract

This directory pins the contract used to build and test `xui-rs`.

- Upstream: `MHSanaei/3x-ui`
- Release: `v3.7.0`
- Commit: `f727d04f6522bb94a8fb52e8352fdcafb51c11e1`
- OpenAPI source: `docs/public/openapi.json`
- OpenAPI SHA-256: `a8c92b434efc1f0c5e68de0a719150bd2056be6b45e784c2ccbad4fdd581cd50`
- OpenAPI operations: 186
- Official container runtime OpenAPI operations: 186

`3x-ui-v3.7.0.openapi.json` is an unmodified copy from that release. The
smaller per-domain route snapshots record endpoints found in the Go routers but
alongside the OpenAPI contract. Contract tests compare both sources with the
SDK route inventory so controller drift cannot hide behind stale generated
documentation.

The official v3.7.0 container's runtime document matches the 186 static
operations. The live test compares normalized method/path sets against the
vendored document, so runtime drift is detected without treating placeholder
names as different routes.

The OpenAPI document is vendored verbatim. Per-domain `*-routes.json` files are
small source snapshots maintained from the exact tagged Go routers; their
contract tests detect drift against both sources. When updating 3x-ui, replace
the OpenAPI file from one exact tag, regenerate the route snapshots, update this
provenance block, and adjust the typed modules in the same commit.

`3x-ui-v3.7.0.remaining-http-routes.json` records the two panel-wide routes and
the six routes on the separate subscription server. All three subscription
`HEAD` handlers exist in the tagged routers but not in the published document.
The OpenAPI operation count includes four documentation-only WebSocket message
pseudo-operations.

`3x-ui-v3.7.0.websocket-contract.json` records the authenticated `/ws` route,
actual `{type,payload,time}` envelope, limits, and all ten message constants
from the tagged hub and broadcaster call sites. The published OpenAPI includes
only four pseudo-message entries, calls one `xrayState` instead of the actual
`xray_state`, and shows `data`/top-level notification fields rather than the
source wire payload. WebSocket contract tests therefore treat the Go snapshot
as authoritative while separately checking the documented handshake and four
pseudo-operation IDs.
