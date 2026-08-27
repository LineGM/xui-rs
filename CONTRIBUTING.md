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
   RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
   ```

## API implementation rules

- Treat the pinned 3x-ui OpenAPI document and v3.6.0 source as the contract.
- Prefer explicit Rust types; use `serde_json::Value` only for intentionally
  open-ended Xray configuration fragments.
- Never include credentials, API tokens, cookies, CSRF tokens, or secret model
  fields in `Debug`, `Display`, tracing fields, or error response bodies.
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
