# Upstream API contract

This directory pins the contract used to build and test `xui-rs`.

- Upstream: `MHSanaei/3x-ui`
- Release: `v3.6.0`
- Commit: `c377dca27c23549cdf84e0ffd2d287a16bee577c`
- OpenAPI source: `web/html/xui/openapi.json`
- OpenAPI SHA-256: `1dd51816003c3ea28efda48bcdab1f3b117aa9fb74461981195263d75bb8a519`
- OpenAPI operations: 160

`3x-ui-v3.6.0.openapi.json` is an unmodified copy from that release. The
smaller per-domain route snapshots record endpoints found in the Go routers but
absent from OpenAPI as well. Contract tests compare both sources with the SDK
route inventory, because the published v3.6.0 document is useful but not
complete.

The OpenAPI document is vendored verbatim. Per-domain `*-routes.json` files are
small source snapshots maintained from the exact tagged Go routers; their
contract tests detect drift against both sources. When updating 3x-ui, replace
the OpenAPI file from one exact tag, regenerate the route snapshots, update this
provenance block, and adjust the typed modules in the same commit.
