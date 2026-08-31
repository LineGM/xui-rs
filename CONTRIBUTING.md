# Contributing to xui-rs

Thanks for helping build a reliable Rust SDK for 3x-ui.

## Before opening a pull request

1. Base changes on `main` and keep each pull request focused.
2. Use Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`,
   or `chore:`).
3. Document every public item and add tests for successful and failing paths.
4. Run the same quality gates as CI:

   ```console
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets --all-features
   cargo test --doc --all-features
   RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
   actionlint
   cargo deny check
   cargo package --locked --allow-dirty
   ```

5. Check the proposed 1.0 public contract when changing any public item:

   ```console
   cargo +1.98.0 install cargo-public-api --version 0.52.0 --locked
   rustup toolchain install nightly-2026-08-31 --profile minimal
   scripts/public-api.sh check
   ```

   Only run `scripts/public-api.sh update` after reviewing the complete diff
   and deciding that the change belongs in the next compatible release.

## API implementation rules

- Treat the pinned 3x-ui OpenAPI document and v3.6.0 source as the contract.
- Use explicit domain re-exports; a glob can accidentally make a helper part of
  the permanent public API.
- Mark enums that may gain variants `#[non_exhaustive]`, and prefer generic
  borrowed inputs such as `&[impl AsRef<str>]` over forcing caller allocations.
- Prefer explicit Rust types; use `serde_json::Value` only for intentionally
  open-ended Xray configuration fragments.
- Never include credentials, API tokens, cookies, CSRF tokens, or secret model
  fields in `Debug`, `Display`, tracing fields, or error response bodies.
- Never read a response body without the shared bounded reader. Keep ordinary
  API, explicit download, and public subscription limits semantically separate.
- Transport tracing must not include origins, paths, queries, headers, bodies,
  cookies, subscription identifiers, or other caller-controlled URL data.
- Preserve custom panel base paths and percent-encode user-controlled path
  segments.
- Mutating endpoints require both success and error contract tests.
- Do not add automatic retries to non-idempotent operations. The one CSRF retry
  is safe because 3x-ui rejects the original request before handler execution.

## Reporting bugs

Open a GitHub issue with the xui-rs version, 3x-ui version, minimal reproduction,
and the redacted request/response shape. Report security-sensitive bugs through
the private process in [SECURITY.md](SECURITY.md).

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
Release maintainers must additionally follow [docs/releasing.md](docs/releasing.md).
