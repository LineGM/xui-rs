#!/usr/bin/env bash

set -Eeuo pipefail

readonly PANEL_VERSION="3.7.0"
readonly PANEL_IMAGE="ghcr.io/mhsanaei/3x-ui@sha256:3b3131f1876e6bf35063a9ec4dd1c594e4525180bfc2e1c477dcc8a3c9550ca1"
readonly PANEL_PORT="2053"
readonly PANEL_BASE_PATH="/xui-live/"
readonly PANEL_USERNAME="xui-live"
readonly PANEL_PASSWORD="xui-live-${RANDOM}-${RANDOM}-password"
readonly RESOURCE_SUFFIX="$(date +%s)-$$-${RANDOM}"
readonly CONTAINER_NAME="xui-rs-live-${RESOURCE_SUFFIX}"
readonly VOLUME_NAME="xui-rs-live-${RESOURCE_SUFFIX}"

container_created=0
volume_created=0

cleanup() {
    local status=$?
    trap - EXIT INT TERM

    if ((status != 0)) && ((container_created != 0)); then
        printf '\n3x-ui container logs:\n' >&2
        docker logs "$CONTAINER_NAME" >&2 || true
    fi
    if ((container_created != 0)); then
        docker rm --force "$CONTAINER_NAME" >/dev/null 2>&1 || true
    fi
    if ((volume_created != 0)); then
        docker volume rm --force "$VOLUME_NAME" >/dev/null 2>&1 || true
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

for command in docker curl cargo; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf 'required command is unavailable: %s\n' "$command" >&2
        exit 2
    fi
done

docker info >/dev/null
docker pull "$PANEL_IMAGE"

reported_version="$(docker run --rm --entrypoint /app/x-ui "$PANEL_IMAGE" -v)"
if [[ "$reported_version" != *"$PANEL_VERSION"* ]]; then
    printf 'expected 3x-ui %s, image reported: %s\n' "$PANEL_VERSION" "$reported_version" >&2
    exit 1
fi

docker volume create \
    --label "org.opencontainers.image.source=https://github.com/LineGM/xui-rs" \
    --label "io.xui-rs.live-test=true" \
    "$VOLUME_NAME" >/dev/null
volume_created=1

docker run --rm \
    --entrypoint /app/x-ui \
    --mount "type=volume,source=${VOLUME_NAME},target=/etc/x-ui" \
    "$PANEL_IMAGE" \
    setting \
    -username "$PANEL_USERNAME" \
    -password "$PANEL_PASSWORD" \
    -webBasePath "$PANEL_BASE_PATH" \
    -port "$PANEL_PORT"

docker run --detach \
    --name "$CONTAINER_NAME" \
    --label "io.xui-rs.live-test=true" \
    --cap-drop ALL \
    --security-opt no-new-privileges:true \
    --pids-limit 256 \
    --memory 768m \
    --cpus 2 \
    --env XUI_ENABLE_FAIL2BAN=false \
    --mount "type=volume,source=${VOLUME_NAME},target=/etc/x-ui" \
    --publish "127.0.0.1::${PANEL_PORT}" \
    "$PANEL_IMAGE" >/dev/null
container_created=1

published_address="$(docker port "$CONTAINER_NAME" "${PANEL_PORT}/tcp")"
host_port="${published_address##*:}"
if [[ ! "$host_port" =~ ^[0-9]+$ ]]; then
    printf 'could not determine the published 3x-ui port from: %s\n' "$published_address" >&2
    exit 1
fi
readonly PANEL_URL="http://127.0.0.1:${host_port}${PANEL_BASE_PATH}"

ready=0
for _ in {1..90}; do
    if ! docker inspect --format '{{.State.Running}}' "$CONTAINER_NAME" | grep -qx true; then
        printf '3x-ui stopped before becoming ready\n' >&2
        exit 1
    fi
    if curl --fail --silent --output /dev/null "$PANEL_URL"; then
        ready=1
        break
    fi
    sleep 1
done
if ((ready == 0)); then
    printf '3x-ui did not become ready within 90 seconds\n' >&2
    exit 1
fi

printf 'Running xui-rs live tests against an isolated 3x-ui %s container\n' "$PANEL_VERSION"
XUI_LIVE_BASE_URL="$PANEL_URL" \
XUI_LIVE_USERNAME="$PANEL_USERNAME" \
XUI_LIVE_PASSWORD="$PANEL_PASSWORD" \
XUI_LIVE_EXPECTED_VERSION="$PANEL_VERSION" \
XUI_LIVE_ALLOW_MUTATION=1 \
cargo test --locked --test live -- --ignored --test-threads=1 --nocapture
