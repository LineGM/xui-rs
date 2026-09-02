#!/usr/bin/env bash

set -Eeuo pipefail

usage() {
    printf 'usage: %s [--allow-dirty] [target-directory]\n' "$0" >&2
    exit 2
}

allow_dirty=0
if [[ "${1:-}" == "--allow-dirty" ]]; then
    allow_dirty=1
    shift
fi
if [[ "${1:-}" == -* ]]; then
    usage
fi
if (($# > 1)); then
    usage
fi

temporary_parent="${CARGO_TARGET_DIR:-target}"
mkdir -p "$temporary_parent"
temporary_root="$(mktemp -d "${temporary_parent%/}/xui-rs-package-check.XXXXXX")"
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    rm -rf -- "$temporary_root"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

target_directory="${1:-${temporary_root}/target}"
mkdir -p "$target_directory"

# The gate compiles every packaged target a second time. Keep the isolated
# artifacts compact and deterministic enough for constrained CI runners.
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

package_id="$(cargo pkgid)"
version="${package_id##*#}"
version="${version##*@}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
    printf 'could not derive a SemVer package version from: %s\n' "$package_id" >&2
    exit 1
fi

package_arguments=(--locked --target-dir "$target_directory")
if ((allow_dirty != 0)); then
    package_arguments+=(--allow-dirty)
fi
cargo package "${package_arguments[@]}"

archive="${target_directory}/package/xui-rs-${version}.crate"
if [[ ! -f "$archive" ]]; then
    printf 'cargo package did not produce the expected archive: %s\n' "$archive" >&2
    exit 1
fi

package_root="${temporary_root}/unpacked/xui-rs-${version}"
mkdir -p "${temporary_root}/unpacked"
tar -xzf "$archive" -C "${temporary_root}/unpacked"
for required in \
    Cargo.toml \
    Cargo.lock \
    LICENSE \
    README.md \
    src/lib.rs \
    tests/public_api.rs \
    tests/live.rs \
    spec/3x-ui-v3.7.0.openapi.json \
    docs/api-stability.md \
    scripts/live-test.sh \
    scripts/package-check.sh; do
    if [[ ! -f "${package_root}/${required}" ]]; then
        printf 'publishable archive is missing required file: %s\n' "$required" >&2
        exit 1
    fi
done

manifest="${package_root}/Cargo.toml"
cargo test \
    --locked \
    --manifest-path "$manifest" \
    --all-targets \
    --all-features \
    --target-dir "$target_directory"
cargo test \
    --locked \
    --manifest-path "$manifest" \
    --doc \
    --all-features \
    --target-dir "$target_directory"
RUSTDOCFLAGS="-D warnings" cargo doc \
    --locked \
    --manifest-path "$manifest" \
    --no-deps \
    --all-features \
    --target-dir "$target_directory"

printf 'verified publishable archive: %s\n' "$archive"
