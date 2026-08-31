#!/usr/bin/env sh

set -eu

version="1.7.12"
expected_sha256="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"
install_dir="${RUNNER_TEMP:-/tmp}/actionlint-bin"
archive="$install_dir/actionlint.tar.gz"
checksums="$install_dir/checksums.txt"

if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
    echo "actionlint bootstrap only supports Linux x86_64 CI runners" >&2
    exit 2
fi

mkdir -p "$install_dir"
curl \
    --proto '=https' \
    --tlsv1.2 \
    --fail \
    --silent \
    --show-error \
    --location \
    --output "$archive" \
    "https://github.com/rhysd/actionlint/releases/download/v$version/actionlint_${version}_linux_amd64.tar.gz"

printf '%s  %s\n' "$expected_sha256" "actionlint.tar.gz" > "$checksums"
(
    cd "$install_dir"
    sha256sum --check checksums.txt
)
tar -xzf "$archive" -C "$install_dir" actionlint

if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n' "$install_dir" >> "$GITHUB_PATH"
else
    printf 'actionlint installed at %s/actionlint\n' "$install_dir"
fi
