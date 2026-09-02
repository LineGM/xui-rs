# Release engineering

Releases are built and published only by
[`.github/workflows/release.yml`](../.github/workflows/release.yml). A manual
run performs the complete verification and uploads a `.crate` artifact but
cannot publish. Only an exact stable-version tag such as `v1.0.2` enables the
protected publish job.

## One-time repository setup

1. Create a GitHub environment named `release`. Require approval, prevent
   self-review where the repository plan supports it, and restrict deployment
   branches and tags to protected release tags.
2. On crates.io, add a trusted publisher for crate `xui-rs` with owner
   `LineGM`, repository `xui-rs`, workflow `release.yml`, and environment
   `release`.
3. Protect `main` and require every CI job before merging. Keep tag creation
   limited to maintainers who can release the crate.

The workflow intentionally has no long-lived `CARGO_REGISTRY_TOKEN`. The
official crates.io authentication action exchanges GitHub's OIDC identity for
a short-lived token and revokes that token after the job. The publish job has
write permissions; all other jobs and workflows remain read-only.

GitHub Actions are pinned to full commit SHAs because a full SHA is the only
immutable action reference. Dependabot may propose pin updates, but each update
must retain the full SHA and its human-readable release comment.

## Preparing a release

1. Update `Cargo.toml` and `Cargo.lock` to the intended version.
2. Move the relevant `CHANGELOG.md` entries from `Unreleased` into a dated
   version section and review the complete public API diff.
3. Run the local release gates:

   ```console
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --all-features -- -D warnings
   cargo test --locked --all-targets --all-features
   cargo test --locked --doc --all-features
   RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features
   actionlint
   cargo deny check
   scripts/public-api.sh check
   scripts/package-check.sh --allow-dirty
   scripts/live-test.sh
   ```

4. Merge the release-preparation commit and wait for all required `main` CI
   checks.
5. Manually run the `Release` workflow from `main`. This is the non-publishing
   rehearsal; inspect the generated `.crate` artifact.
6. Create and push the exact annotated tag, for example:

   ```console
   git tag -a v1.0.2 -m "xui-rs 1.0.2"
   git push origin v1.0.2
   ```

7. Review and approve the protected `release` deployment. The workflow checks
   that the tag equals the Cargo package version, proves the annotated tag is
   reachable from `main`, tests the extracted `.crate` with its packaged tests,
   examples, doctests, and documentation, repeats MSRV and macOS/Windows
   portability checks, exercises the official 3x-ui container, reproduces the
   verified package across jobs, records signed SLSA provenance, publishes
   through crates.io trusted publishing, and finally creates the GitHub release.

The publish job is safe to rerun after a partial external failure. If the
version already exists on crates.io, the workflow continues only when its
published SHA-256 exactly matches the independently verified artifact; it can
then repair a missing GitHub release or replace its attached copy.

## Verification and recovery

Download the `.crate` file from the GitHub release and verify its provenance:

```console
gh attestation verify xui-rs-1.0.2.crate --repo LineGM/xui-rs
cargo info xui-rs@1.0.2
```

Published crates.io versions are immutable. If a release is defective, yank
that exact version, document the reason, and publish a new patch version; never
attempt to replace an existing archive or move its tag.

The policy enforced by `deny.toml` covers all enabled features and Linux,
macOS, and Windows dependency graphs. RustSec advisories, yanked packages,
unapproved licenses, wildcard requirements, unknown registries, unknown Git
sources, native TLS, and OpenSSL dependencies fail CI. Exact unavoidable
duplicate versions are documented in the policy and must be reconsidered when
their parent dependencies update.
