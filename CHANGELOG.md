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

### Security

- Redacted protocol settings, stream settings, sniffing data, UUIDs, and
  subscription IDs from model `Debug` output.
