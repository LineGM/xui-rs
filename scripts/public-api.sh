#!/usr/bin/env sh

set -eu

command_name=${1:-check}
case "$command_name" in
    check|update) ;;
    *)
        echo "usage: scripts/public-api.sh [check|update]" >&2
        exit 2
        ;;
esac

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
snapshot_path="$project_root/api/public-api.txt"
temporary_path=$(mktemp "${TMPDIR:-/tmp}/xui-rs-public-api.XXXXXX")
trap 'rm -f "$temporary_path"' EXIT HUP INT TERM

expected_version="cargo-public-api 0.52.0"
actual_version=$(cargo public-api --version)
if [ "$actual_version" != "$expected_version" ]; then
    echo "expected $expected_version, found $actual_version" >&2
    echo "install it with: cargo +1.98.0 install cargo-public-api --version 0.52.0 --locked" >&2
    exit 2
fi

(
    cd "$project_root"
    cargo +nightly-2026-08-31 public-api \
        --simplified --simplified --simplified \
        --all-features \
        --color never
) > "$temporary_path"

case "$command_name" in
    check)
        if [ ! -f "$snapshot_path" ]; then
            echo "public API snapshot is missing; run scripts/public-api.sh update" >&2
            exit 2
        fi
        diff -u "$snapshot_path" "$temporary_path"
        ;;
    update)
        mkdir -p "$(dirname -- "$snapshot_path")"
        cp "$temporary_path" "$snapshot_path"
        ;;
esac
