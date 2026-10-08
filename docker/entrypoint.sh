#!/bin/sh
# Container entrypoint.
#
# The working directory is the data directory so that runtime state files
# (`.first_boot.lock`) end up on the mounted volume instead of the container
# layer. The server reads `./static`, `./README.md` and `./docs/openapi.yaml`
# relative to that directory, so symlinks are created here to keep the shipped
# assets reachable from the data volume.
set -e

DATA_DIR="${COSMX_DATA_DIR:-/var/lib/cosmox-server}"
APP_DIR="${COSMX_APP_DIR:-/app}"

cd "${DATA_DIR}"

if [ ! -e "${DATA_DIR}/static" ]; then
    ln -s "${APP_DIR}/static" "${DATA_DIR}/static"
fi

if [ ! -e "${DATA_DIR}/README.md" ]; then
    ln -s "${APP_DIR}/README.md" "${DATA_DIR}/README.md"
fi

if [ ! -e "${DATA_DIR}/docs" ]; then
    ln -s "${APP_DIR}/docs" "${DATA_DIR}/docs"
fi

exec "${APP_DIR}/server" "$@"
