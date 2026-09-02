# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.2] - 2026-09-02

### Added

- Expanded behavioral coverage for wire-compatible models, secret-redacted
  diagnostics, configuration validation, HTTP failures, proxy handling,
  subscription decoding, and WebSocket lifecycle edge cases; all-target line
  coverage now exceeds 97% while the CI regression floor remains 80%.

## [1.0.1] - 2026-09-02

### Changed

- Refreshed the README and maintenance documentation after the first stable
  release.
- Replaced rustdoc-only hidden-line markers in Markdown examples with clean,
  readable Rust snippets for GitHub and crates.io.
- Clarified the supported-version and release procedures without retaining
  obsolete pre-rewrite compatibility claims.

## [1.0.0] - 2026-09-02

### Changed

- Replaced the original implementation with a new cloneable client foundation.
- Made API-token authentication the recommended automation path.
- Reimplemented cookie login around 3x-ui v3.7.0's CSRF lifecycle and a real
  cookie jar; removed credential retention and automatic re-login.
- Normalized the successful `null` returned by an empty panel's `allLinks`
  endpoint to an empty vector.
- Introduced typed, contextual transport, HTTP, API, and decoding errors.
- Replaced the opaque internal cookie store with one explicit shared
  standards-compliant jar so HTTP and WebSocket handshakes use identical
  domain, path, secure, replacement, and expiry rules.
- Exposed documented domain modules while retaining every concise crate-root
  re-export, making the large API navigable without breaking existing imports.
- Raised the MSRV from Rust 1.85 to 1.88 so the cookie stack can use the
  `time` release that fixes RUSTSEC-2026-0009.
- Made every extensible public enum non-exhaustive and replaced domain glob
  re-exports with explicit, reviewed export lists before the 1.0 API freeze.
- Generalized client bulk operations and REALITY batch scans to accept borrowed
  string-like slices instead of requiring caller-owned `String` values, with a
  dedicated empty-seed REALITY convenience method.

### Added

- Username/password and optional 2FA login, logout, CSRF, and 2FA-status APIs.
- Secret-redaction, custom base-path support, strict lint configuration, and
  contract tests for authentication behaviour.
- Complete typed 3x-ui v3.7.0 inbound API: full/slim lists, options and share
  links, CRUD, enable/reset/delete operations, import, fallback management,
  and global client-traffic synchronization.
- A vendored upstream OpenAPI document, source-route snapshot, and coverage
  test that detects missing or extra inbound operations.
- Complete typed client and client-group API across all 46 routes registered by
  3x-ui v3.7.0: full/paged lookup, CRUD, attachments, links, traffic/IP/online
  state, HWID device management, import/export, bulk operations, and group
  lifecycle management.
- Typed server-side paging filters and sorting, null-safe Go slice decoding,
  exact `totalGB`/`allowedIPs` wire names, and percent-encoded path segments.
- Complete typed server API across all 39 routes registered by 3x-ui v3.7.0,
  including AmneziaWG status, logs, and peer activity.
- Typed machine/Xray metrics, structured logs, maintenance and cryptographic
  helpers, cluster IP synchronization, in-memory database downloads, and
  multipart database restore.
- A server source-route snapshot and exhaustive mocked route coverage that
  tracks the authoritative Go response shapes where OpenAPI examples differ.
- Complete typed panel and Xray settings APIs across all 40 routes registered
  by v3.7.0, including scoped/expiring API tokens, WARP/NordVPN/PIA, geodata,
  outbound tests, routing tests, and remote outbound subscriptions.
- Ergonomic grouped panel settings that flatten to exact upstream wire names,
  automatic encoding/decoding of nested Xray JSON, typed SMTP diagnostics, and
  a settings source-route snapshot checked independently against OpenAPI.
- Complete typed Hosts API across all 12 routes registered by v3.7.0, including
  the bulk-create alias and the controller's grouped-versus-row response
  distinction.
- Typed forward-compatible host security, Mihomo IP preference, subscription
  format, VLESS route, and nested JSON override values.
- Complete typed Nodes API across all 16 routes registered by v3.7.0, including
  registration, probes, remote inbound discovery, metric history, remote panel
  updates, certificate pinning, node mTLS, and live mTLS-client reload.
- Source-accurate 41-field `NodeView`, forward-compatible node enums, typed
  update channels/metrics, and explicit mutually exclusive API-token actions.
- Complete remaining v3.7.0 HTTP surface: the runtime OpenAPI and Telegram
  backup panel routes plus `GET` and source-only `HEAD` operations for raw,
  Xray JSON, and Clash/Mihomo public subscriptions.
- A standalone no-auth subscription client with configurable public paths,
  construction from panel settings, deterministic content negotiation, typed
  traffic/profile/HWID headers, base64 decoding, device identity, and the typed
  `format=info` view.
- A source-route snapshot covering all eight operations outside the completed
  domain controllers, including four routes absent from upstream OpenAPI.
- Complete v3.7.0 WebSocket support: authenticated handshake, typed stream and
  all ten source-declared event names, including traffic/counter payloads,
  resource invalidation, forward-compatible unknown events, close metadata,
  explicit reconnect, ping/pong handling, and the source's 10 MiB limit.
- A source-level WebSocket contract snapshot plus end-to-end HTTP login →
  shared-cookie handshake → event decoding → close/reconnect tests that cover
  the six message types omitted from upstream OpenAPI and its stale examples.
- Stable `ErrorKind` classification plus HTTP status/method/URL,
  authentication, rate-limit, server-error, and cross-transport timeout
  introspection helpers.
- A compiled crate-level quick start, public API/trait contract tests, and an
  idempotency-aware error and retry guide.
- A shared explicit outbound proxy API for panel HTTP, public subscriptions,
  and WebSocket connections with HTTP, HTTPS, SOCKS5, and proxy-resolved
  `socks5h` support, authentication, deterministic no-ambient-proxy behavior,
  and stable proxy error introspection.
- Protocol-level proxy tests covering panel CSRF/login cookies, subscription
  downloads, authenticated HTTP CONNECT, SOCKS5 authentication, remote DNS,
  and WebSocket events through real local tunnels.
- Independently configurable response-body limits for ordinary panel APIs,
  database downloads, and public subscriptions, with stable oversized-body
  error introspection and safe defaults.
- Correlated HTTP transport tracing with request IDs, methods, statuses,
  outcomes, and response-header latency.
- A complete typed subscription-balancer controller with all five v3.7.0
  routes, exact repeated-form encoding, and forward-compatible strategies.
- Typed AmneziaWG inbound/server models, client traffic-reset additions, PIA
  integration, geodata browsing/validation, and all new v3.7.0 settings fields.
- A reproducible rustdoc snapshot of the complete 1.0 surface plus an
  exhaustive downstream contract for root re-exports, traits, constants, and
  multithreaded-runtime-safe endpoint futures.
- A release-engineering pipeline with immutable Action pins, package and public
  API gates, PR-base SemVer checks, protected tag releases, crates.io trusted
  publishing, checksum-verified workflow tooling, reproducible crate
  comparison, and signed SLSA provenance.
- A digest-pinned disposable 3x-ui v3.7.0 container harness covering real CSRF,
  cookie login/logout, the shared authenticated WebSocket cookie, API-token
  bearer authentication, runtime OpenAPI parity, and an inbound CRUD round
  trip with guarded mutations and layered cleanup.
- A manual and weekly live-integration workflow that runs the isolated panel
  harness independently of deterministic mocked CI.
- A release-candidate package gate that extracts the exact `.crate` archive and
  runs its packaged tests, examples, doctests, and documentation rather than
  assuming an in-repository build proves the published artifact.
- Mandatory MSRV, macOS, Windows, and real 3x-ui jobs in the protected release
  workflow, plus annotated-tag, `main` ancestry, and dated-changelog checks
  before any publish job can start.
- An enforced 80% all-target line-coverage floor, while retaining explicit
  route, wire-format, error, and live-panel assertions as the primary contract.

### Security

- Redacted protocol settings, stream settings, sniffing data, UUIDs, and
  subscription IDs from model `Debug` output.
- Redacted client protocol credentials, private/pre-shared keys, MTProto
  secrets, subscription identifiers, and external-link values from `Debug`.
- Redacted generated X25519, ML-DSA, ML-KEM, ECH, and VLESS encryption private
  material from `Debug`.
- Redacted full Xray configuration and database backup bytes from `Debug` while
  retaining explicit accessors for callers that intentionally need them.
- Redacted panel notification/LDAP credentials, integration payloads,
  subscription URLs and outbounds, API-token plaintext, and secret panel paths
  while retaining explicit accessors for intentional use.
- Redacted nested host JSON overrides from `Debug` while preserving explicit
  raw and parsed accessors.
- Kept node API tokens write-only and redacted token replacements, remote panel
  base paths, and internal egress tags from model `Debug` output.
- Kept panel authentication out of public subscription requests and redacted
  subscription identifiers from URLs/errors plus documents, generated links,
  emails, profile URLs, and routing rules from `Debug` output.
- Prevented API bearer tokens from reaching `/ws`; redacted notification/Xray
  details, opaque event payloads, connection paths, cookie state, and socket
  internals from `Debug` while preserving explicit typed access.
- Rejected credentials embedded in proxy URLs and kept proxy endpoints,
  usernames, and passwords out of client/proxy `Debug` and WebSocket proxy
  errors.
- Bounded both declared and actually received HTTP response bytes, including
  chunked and decompressed bodies, and omitted URLs, paths, queries, headers,
  cookies, tokens, and bodies from transport tracing.
- Reduced panel client and builder `Debug` output to the server origin so a
  deployment's secret base path is not exposed.
- Enforced RustSec advisory, license, source-registry, wildcard-version, and
  rustls-only dependency policies across Linux, macOS, and Windows graphs.
