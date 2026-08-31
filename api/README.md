# Public API baseline

`public-api.txt` is the proposed xui-rs 1.0 public contract generated from
rustdoc JSON. It includes owned items, methods, fields, explicit trait
implementations, module paths, and crate-root re-exports while omitting noisy
blanket, auto-trait, and auto-derived implementations.

Check the working tree against the committed baseline:

```console
scripts/public-api.sh check
```

The check intentionally pins `cargo-public-api` 0.52.0 and
`nightly-2026-08-31`. The nightly compiler is used only to produce rustdoc
JSON; it does not change the crate's Rust 1.88 MSRV.

After deliberately reviewing a pre-1.0 API change, update the baseline with:

```console
scripts/public-api.sh update
git diff -- api/public-api.txt
```

Never update the snapshot merely to make a check pass. After 1.0, additions
must remain backward compatible and removals or signature changes require the
next major release.
