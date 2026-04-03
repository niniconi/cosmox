FROM rust:1.91-slim-bookworm AS builder

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

COPY cosmox-api/Cargo.toml cosmox-api/
COPY cosmox-macros/Cargo.toml cosmox-macros/
COPY cosmox-plugin-pack/Cargo.toml cosmox-plugin-pack/
COPY migration/Cargo.toml migration/
COPY Cargo.toml Cargo.lock ./

RUN mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && mkdir cosmox-api/src \
  && echo "" > cosmox-api/src/lib.rs \
  && mkdir cosmox-macros/src \
  && echo "" > cosmox-macros/src/lib.rs \
  && mkdir cosmox-plugin-pack/src \
  && echo "" > cosmox-plugin-pack/src/lib.rs \
  && mkdir migration/src \
  && echo "" > migration/src/lib.rs \
  && cargo build --release

RUN find . -not -path "./target*" -delete

COPY . .

RUN find . -path "./target" -prune -o \( -name "lib.rs" -o -name "main.rs" -o -name "build.rs" \) -exec touch {} +

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/cosmox /app/server
COPY --from=builder /app/application.yaml /app/application.yaml
COPY static /app/static

RUN sed -i 's/127.0.0.1/0.0.0.0/g' /app/application.yaml

RUN chmod +x /app/server

EXPOSE 8080

ENTRYPOINT [ "./server" ]
