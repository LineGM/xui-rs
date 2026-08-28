# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Replaced the 0.1 implementation with a new cloneable client foundation.
- Made API-token authentication the recommended automation path.
- Reimplemented cookie login around 3x-ui v3.6.0's CSRF lifecycle and a real
  cookie jar; removed credential retention and automatic re-login.
- Introduced typed, contextual transport, HTTP, API, and decoding errors.

### Added

- Username/password and optional 2FA login, logout, CSRF, and 2FA-status APIs.
- Secret-redaction, custom base-path support, strict lint configuration, and
  contract tests for authentication behaviour.
- Complete typed 3x-ui v3.6.0 inbound API: full/slim lists, options and share
  links, CRUD, enable/reset/delete operations, import, fallback management,
  and global client-traffic synchronization.
- A vendored upstream OpenAPI document, source-route snapshot, and coverage
  test that detects missing or extra inbound operations.
- Complete typed client and client-group API across all 43 routes registered by
  3x-ui v3.6.0: full/paged lookup, CRUD, attachments, links, traffic/IP/online
  state, import/export, bulk operations, and group lifecycle management.
- Typed server-side paging filters and sorting, null-safe Go slice decoding,
  exact `totalGB`/`allowedIPs` wire names, and percent-encoded path segments.
- Complete typed server API across all 38 routes registered by 3x-ui v3.6.0,
  including the three update-status and REALITY-scan routes omitted from
  upstream OpenAPI.
- Typed machine/Xray metrics, structured logs, maintenance and cryptographic
  helpers, cluster IP synchronization, in-memory database downloads, and
  multipart database restore.
- A server source-route snapshot and exhaustive mocked route coverage that
  tracks the authoritative Go response shapes where OpenAPI examples differ.
- Complete typed panel and Xray settings APIs across all 35 routes registered
  by v3.6.0, including API tokens, WARP/NordVPN, outbound tests, routing tests,
  and remote outbound subscriptions.
- Ergonomic grouped panel settings that flatten to exact upstream wire names,
  automatic encoding/decoding of nested Xray JSON, typed SMTP diagnostics, and
  a settings source-route snapshot covering two operations absent from OpenAPI.
- Complete typed Hosts API across all 12 routes registered by v3.6.0, including
  the source-only bulk-create alias and the controller's grouped-versus-row
  response distinction.
- Typed forward-compatible host security, Mihomo IP preference, subscription
  format, VLESS route, and nested JSON override values.
- Complete typed Nodes API across all 15 routes registered by v3.6.0, including
  registration, probes, remote inbound discovery, metric history, remote panel
  updates, certificate pinning, and node mTLS.
- Source-accurate 41-field `NodeView`, forward-compatible node enums, typed
  update channels/metrics, and explicit mutually exclusive API-token actions.

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
