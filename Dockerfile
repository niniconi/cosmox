# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Build stage
#
# NOTE: the build context, the file path (`./Dockerfile`) and the GHA cache
# strategy used by `.github/workflows/build-and-push-image.yaml` must stay
# unchanged, so that the release pipeline keeps working as-is.
# ---------------------------------------------------------------------------
FROM rust:slim-bookworm AS builder

# Core build dependencies required by the root package and its direct
# dependencies. cosmox-lua and cosmox-ffmpeg are workspace members but not in
# the default `cargo build --release` graph (only the root package + deps are
# built), so the media/lua dev packages below are optional and only needed for
# those other build targets / kept for future use.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    clang \
    libclang-dev \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libswresample-dev \
    libavdevice-dev \
    libavfilter-dev \
    libpostproc-dev \
    libssl-dev \
    liblua5.4-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# -----------------------------------------------------------
# Dependency caching layer
# Copy only Cargo.toml files and create dummy sources so that
# `cargo build` downloads and compiles all external dependencies
# without needing real source code.
# -----------------------------------------------------------
COPY Cargo.toml Cargo.lock ./

COPY crates/common/Cargo.toml                 crates/common/
COPY crates/cosmox-adapter-ffi/Cargo.toml     crates/cosmox-adapter-ffi/
COPY crates/cosmox-adapter-ipc/Cargo.toml     crates/cosmox-adapter-ipc/
COPY crates/cosmox-adapter-web/Cargo.toml     crates/cosmox-adapter-web/
COPY crates/cosmox-agent/Cargo.toml           crates/cosmox-agent/
COPY crates/cosmox-api/Cargo.toml             crates/cosmox-api/
COPY crates/cosmox-backend-api/Cargo.toml     crates/cosmox-backend-api/
COPY crates/cosmox-backend-data/Cargo.toml    crates/cosmox-backend-data/
COPY crates/cosmox-configuration/Cargo.toml   crates/cosmox-configuration/
COPY crates/cosmox-ffmpeg/Cargo.toml          crates/cosmox-ffmpeg/
COPY crates/cosmox-lua/Cargo.toml             crates/cosmox-lua/
COPY crates/cosmox-macros/Cargo.toml          crates/cosmox-macros/
COPY crates/cosmox-plugin-manager/Cargo.toml  crates/cosmox-plugin-manager/
COPY crates/cosmox-plugin-packager/Cargo.toml crates/cosmox-plugin-packager/
COPY crates/cosmox-python/Cargo.toml          crates/cosmox-python/
COPY crates/cosmox-scanner/Cargo.toml         crates/cosmox-scanner/
COPY crates/cosmox-sdk/Cargo.toml             crates/cosmox-sdk/
COPY crates/cosmox-template/Cargo.toml        crates/cosmox-template/
COPY crates/migration/Cargo.toml              crates/migration/

RUN mkdir -p src \
    crates/common/src \
    crates/cosmox-adapter-ffi/src \
    crates/cosmox-adapter-ipc/src \
    crates/cosmox-adapter-web/src \
    crates/cosmox-agent/src \
    crates/cosmox-api/src \
    crates/cosmox-backend-api/src \
    crates/cosmox-backend-data/src \
    crates/cosmox-configuration/src \
    crates/cosmox-ffmpeg/src \
    crates/cosmox-lua/src \
    crates/cosmox-macros/src \
    crates/cosmox-plugin-manager/src \
    crates/cosmox-plugin-packager/src \
    crates/cosmox-python/src \
    crates/cosmox-scanner/src \
    crates/cosmox-sdk/src \
    crates/cosmox-template/src \
    crates/migration/src \
   && echo 'fn main() {}' > src/main.rs \
  && touch crates/migration/src/main.rs \
  && for dir in src crates/*/src; do \
       [ -f "$dir/lib.rs" ] || touch "$dir/lib.rs"; \
     done \
  && touch crates/cosmox-sdk/build.rs \
  && cargo build --release

# -----------------------------------------------------------
# Real build layer
# Remove dummy sources and copy the full project.
# -----------------------------------------------------------
RUN find . -not -path "./target*" -delete

COPY . .

# Touch all source & build files so Cargo recompiles them
# (timestamps are newer than the cached dependency artifacts).
RUN find . -path "./target" -prune -o \( -name "*.rs" -o -name "*.wit" -o -name "build.rs" \) -exec touch {} +

RUN cargo build --release

# The plugin packager CLI is needed by the `plugins` stage below to turn each
# plugin source tree into the on-disk layout the server loads. Build it here so
# its dependencies are cached together with the rest of the workspace.
RUN cargo build --release -p cosmox-plugin-packager

# ---------------------------------------------------------------------------
# Frontend stage
# Builds the Flutter web client and emits the static bundle that the server
# serves at `/`.
#
# No ref is pinned yet on purpose: the repositories currently have no tagged
# releases. The client's default branch (`master`) is an empty stub, so the
# project is taken from its `dev` branch.
#
# The client is a Flutter + Rust project (flutter_rust_bridge over `cosmox-sdk`),
# so a Rust toolchain is required next to Flutter for `flutter build web` to
# work.
# ---------------------------------------------------------------------------
FROM ghcr.io/cirruslabs/flutter:stable AS frontend

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    unzip \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup target add wasm32-unknown-unknown

WORKDIR /src

RUN git clone --depth 1 --branch dev https://github.com/niniconi/cosmox-client-flutter.git .

RUN flutter config --no-analytics || true

RUN flutter pub get

RUN flutter build web --release

# ---------------------------------------------------------------------------
# Plugin stage
# Builds the bundled plugins (wasm32-wasip2) and packages each of them into
# the layout expected by the plugin loader:
#
#     <name>/about.yaml
#     <name>/wasm/<name>.wasm
#     <name>/defines/   (optional)
#
# The result is copied into the runtime image at `/app/plugins`; the entrypoint
# seeds it into `cosmox.plugin.path` at startup. No ref is pinned yet on purpose
# (see the frontend stage).
# ---------------------------------------------------------------------------
FROM rust:slim-bookworm AS plugins

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-wasip2

# Packager CLI reused from the builder stage.
COPY --from=builder /app/target/release/cosmox-plugin-packager /usr/local/bin/cosmox-plugin-packager

WORKDIR /plugins-src

# Each repository is cloned, built with its own `.cargo/config.toml`
# (which selects the wasm32-wasip2 target), then packaged. `cargo build` must
# run from inside the repository so that its `.cargo/config.toml` and
# `rust-toolchain.toml` are picked up.
RUN set -eux; \
    for repo in cosmox-plugin-anime cosmox-plugin-bangumi; do \
        git clone --depth 1 "https://github.com/niniconi/${repo}.git" "${repo}"; \
        ( cd "${repo}" && cargo build --release ); \
        cosmox-plugin-packager pack --input "${repo}" --release --output /plugins-out; \
        mkdir -p "/plugins/${repo}"; \
        cp -a "/plugins-out/release/${repo}/build/." "/plugins/${repo}/"; \
    done

# ---------------------------------------------------------------------------
# Runtime stage
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

# Install runtime dependencies.
# The released binary does not link libav/ffmpeg/liblua (the ffmpeg/lua
# workspace members are not in the default build graph); curl is needed for the
# health check. The ffmpeg/lua libraries are kept as convenience/tooling deps
# and so they are already present if those features are enabled later.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    libavcodec59 \
    libavformat59 \
    libavutil57 \
    libswscale6 \
    libswresample4 \
    libavfilter8 \
    libavdevice59 \
    libpostproc56 \
    liblua5.4-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Unprivileged runtime user owning every writable location.
RUN groupadd --system --gid 10001 cosmox \
  && useradd --system --uid 10001 --gid cosmox --home-dir /var/lib/cosmox-server --shell /usr/sbin/nologin cosmox \
  && mkdir -p /app /etc/cosmox-server \
    /var/lib/cosmox-server /var/log/cosmox-server /var/cache/cosmox-server \
  && chown -R cosmox:cosmox /app /var/lib/cosmox-server /var/log/cosmox-server /var/cache/cosmox-server

WORKDIR /app

COPY --from=builder --chown=cosmox:cosmox /app/target/release/cosmox /app/server
COPY --from=builder --chown=cosmox:cosmox /app/application.yaml /etc/cosmox-server/application.yaml
# Pre-built web UI (Flutter client) emitted by the `frontend` stage.
COPY --from=frontend --chown=cosmox:cosmox /src/build/web /app/static
# Runtime documentation: `system::about` reads `./README.md` and the docs
# controller reads `./docs/openapi.yaml`, both relative to the working
# directory. The entrypoint symlinks them from here into the data volume.
COPY --chown=cosmox:cosmox README.md /app/README.md
COPY --chown=cosmox:cosmox docs/openapi.yaml /app/docs/openapi.yaml
# Bundled plugins. They are kept outside the data volume (`/app/plugins`) so
# they are never shadowed by a pre-existing/bind-mounted `cosmox-data` volume;
# the entrypoint seeds them into the configured plugin directory
# (`cosmox.plugin.path`) on every start.
COPY --from=plugins --chown=cosmox:cosmox /plugins /app/plugins

# Keep a copy inside the image so the binary also works when run from /app
# without mounting an external configuration (the /etc path is checked first,
# so this copy is only needed if /etc is absent or overridden).
RUN cp /etc/cosmox-server/application.yaml /app/application.yaml \
  && chown cosmox:cosmox /app/application.yaml

# `.first_boot.lock` now lives under `cosmox.state.path`, so the working
# directory is set to the data volume to keep that state on the volume too.
WORKDIR /var/lib/cosmox-server

ENV RUST_LOG=info

# The image ships the bundled plugins at /app/plugins; the entrypoint copies
# them into this directory at startup. The packaged `application.yaml` ships
# development paths (`test/plugin`), so pin the plugin directory here so the
# bundled plugins are found in every deployment, including a plain `docker run`
# without extra env overrides.
ENV COSMOX_COSMOX__PLUGIN__PATH=/var/lib/cosmox-server/plugins

COPY --chmod=0755 docker/entrypoint.sh /usr/local/bin/cosmox-entrypoint

USER cosmox

EXPOSE 8080

# Liveness only: any HTTP response (including 401/404) means the server
# process is up. This is NOT a readiness probe — use the `/api/system/info`
# response body or a 200 check if you need a true readiness signal.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl --silent --output /dev/null --write-out '%{http_code}' http://127.0.0.1:8080/api/system/info | grep -qE '^(2|3|4)' || exit 1

ENTRYPOINT ["/usr/local/bin/cosmox-entrypoint"]
