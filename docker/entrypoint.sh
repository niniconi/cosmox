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

# Seed the bundled plugins into the (writable) plugin directory. They are
# shipped in the image at ${APP_DIR}/plugins, but the data volume may already
# exist, in which case Docker does not copy the image contents into it. Copying
# here keeps the bundled plugins available regardless of the volume state while
# leaving any plugins the user added in place.
PLUGIN_DIR="${COSMX_COSMOX__PLUGIN__PATH:-${DATA_DIR}/plugins}"
case "${PLUGIN_DIR}" in
    /*) ;;
    *) PLUGIN_DIR="${DATA_DIR}/${PLUGIN_DIR}" ;;
esac
if [ -d "${APP_DIR}/plugins" ] && [ "${PLUGIN_DIR}" != "${APP_DIR}/plugins" ]; then
    mkdir -p "${PLUGIN_DIR}"
    cp -a "${APP_DIR}/plugins/." "${PLUGIN_DIR}/"
fi

exec "${APP_DIR}/server" "$@"
