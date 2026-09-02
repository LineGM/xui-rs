# API stability

xui-rs follows Semantic Versioning for its Rust API. The 1.0 line is the stable
contract for 3x-ui v3.7.0 and is intentionally not source-compatible with the
original implementation.

## What 1.x guarantees

Within the 1.x series, ordinary dependency updates must preserve:

- public module paths and the concise crate-root re-exports;
- constructors, endpoint methods, argument types, return types, and public
  model fields;
- documented trait guarantees such as `Send`, `Sync`, `Clone`, and `Copy`;
- stable `ErrorKind` labels and error-introspection semantics;
- serde wire names and the v3.7.0 endpoint contract.

Adding a method or a new type is normally compatible. Removing or renaming an
item, narrowing a generic input, changing a return type, removing a trait
implementation, or changing a public field requires a major release.

Enums that may grow are marked `#[non_exhaustive]`. Downstream matches must
keep a wildcard arm. Unknown server vocabulary is preserved where the wire
model supports an explicit unknown/custom representation, but a newer 3x-ui
release can still require a new xui-rs compatibility release.

The guarantee covers source compatibility, not successful execution against
an arbitrarily changed server. Network availability, panel authorization,
Xray behavior, and effects of administrative endpoints remain outside the Rust
SemVer contract.

## How the contract is enforced

[`api/public-api.txt`](../api/public-api.txt) is generated from rustdoc JSON
and records the complete 1.0 API, including module paths, root
re-exports, signatures, fields, and explicit trait implementations. The
snapshot deliberately omits compiler-generated blanket, auto-trait, and
auto-derived noise.

`tests/public_api.rs` independently compiles as a downstream crate. It verifies
every concise root re-export, important trait guarantees, typed error
introspection, public constants, and representative `Send` endpoint futures.

Run the reproducible snapshot comparison with:

```console
scripts/public-api.sh check
```

The repository pins both the snapshot tool and its rustdoc nightly. See
[`api/README.md`](../api/README.md) before deliberately updating the baseline.
After 1.0, CI SemVer comparison against the latest release is an additional
guard; the snapshot remains useful for human review of additions as well as
breakage.
