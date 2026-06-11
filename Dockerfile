FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libavdevice-dev \
    libavfilter-dev \
    libssl-dev \
    libclang-dev \
    clang \
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
  && touch src/main.rs \
  && for dir in src crates/*/src; do \
       [ -f "$dir/lib.rs" ] || touch "$dir/lib.rs"; \
     done \
  && touch crates/cosmox-sdk/build.rs \
  && cargo build --release 2>&1 || echo "--- Dep cache build completed (expected compile errors for dummy sources) ---"

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

# -----------------------------------------------------------
# Runtime image
# -----------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libavdevice-dev \
    libavfilter-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/cosmox /app/server
COPY --from=builder /app/application.yaml /app/application.yaml
COPY static /app/static

EXPOSE 8080

ENTRYPOINT [ "./server" ]
