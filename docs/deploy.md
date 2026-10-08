# Deployment

This document describes how to run cosmox on a server with Docker. Three
scenarios are covered:

| Scenario | Use it when |
| --- | --- |
| [docker compose](#1-docker-compose-recommended) | Default choice: cosmox + MariaDB on the same host |
| [Prebuilt image](#2-prebuilt-image-only) | The database is managed elsewhere (external MySQL/MariaDB) |
| [Local development](#3-local-development) | Running from the sources on a workstation |

## Configuration model

`application.yaml` is read from the first existing location:

1. `/etc/cosmox-server/application.yaml` (the packaged configuration inside the image)
2. `application.yaml` in the working directory

On top of that file, every value can be overridden with an environment
variable: prefix `COSMX_`, nesting separator `__`.

```bash
COSMX_SERVER__HOST=0.0.0.0        # server.host
COSMX_SERVER__PORT=9090           # server.port
COSMX_DATABASE__HOST=mariadb      # database.host
COSMX_DATABASE__PASSWORD=secret   # database.password
COSMX_COSMOX__LOG__PATH=/var/log/cosmox-server   # cosmox.log.path
```

The convention is `COSMX_` + `<top level key>` + `__` + `<nested key>`. Because
the top-level section holding the data paths is itself named `cosmox`, the data
path overrides are prefixed twice, e.g. `COSMX_COSMOX__DATA__PATH`.

Environment variable values are parsed as config scalars (bool/i64/f64), so `8080` stays a number and `123456` can still be used as a password (via lenient string conversion).

Each environment variable segment mirrors the corresponding yaml key: write
the yaml path in SCREAMING_SNAKE_CASE, e.g. `cosmox.scanner.metadata-path`
becomes `COSMX_COSMOX__SCANNER__METADATA_PATH`.

> Never store real passwords in the image: inject them through the
> environment (see `.env.example`) or mount your own `application.yaml` over
> `/etc/cosmox-server/application.yaml`.

### Filesystem layout inside the container

| Path | Content | Volume |
| --- | --- | --- |
| `/app/server` | the cosmox binary | image |
| `/app/static` | web UI assets (served from the working directory) | image |
| `/etc/cosmox-server/application.yaml` | configuration | image or bind mount |
| `/var/lib/cosmox-server` | working directory: data, library, metadata, plugin, state | `cosmox-data` |
| `/var/lib/cosmox-server/state` | state directory, including `.first_boot.lock` | `cosmox-data` |
| `/var/log/cosmox-server` | rolling application logs | `cosmox-log` |
| `/var/cache/cosmox-server` | scanner cache | `cosmox-cache` |

The working directory is `/var/lib/cosmox-server` on purpose: `.first_boot.lock`
is created under `cosmox.state.path` (the `state` directory inside the data
volume by default), so keeping the working directory on a volume makes the
first-boot state survive container restarts and recreations.

## 1. docker compose (recommended)

```bash
cp .env.example .env      # adjust credentials
docker compose up -d
docker compose ps
docker compose logs -f cosmox
```

The compose file starts:

- `cosmox` on `${COSMX_HTTP_PORT:-8080}`, waiting for the database health check;
- `mariadb` with a persistent volume and a `utf8mb4` charset.

By default the image is built from the current sources. To deploy an image
published by the release pipeline instead, remove the `build:` block from
`docker-compose.yml` and set in `.env`:

```dotenv
COSMX_IMAGE=ghcr.io/<owner>/<repo>:<tag>
```

(With the `build:` block still present, compose would build locally and just
tag the result with this name.)

To use your own configuration file instead of the image default:

```yaml
services:
  cosmox:
    volumes:
      - ./application.yaml:/etc/cosmox-server/application.yaml:ro
```

Named volumes inherit the ownership of the directories declared in the image,
so the unprivileged `cosmox` user can write into them. When using **bind
mounts** instead, make sure the host directory is writable by uid/gid `10001`:

```bash
sudo chown -R 10001:10001 /srv/cosmox
```

Common operations:

```bash
docker compose logs -f cosmox          # follow the server logs
docker compose restart cosmox          # restart after a configuration change
docker compose pull && docker compose up -d   # upgrade to a new image tag
docker compose down                    # stop, keep the volumes
docker compose down -v                 # stop and delete all the data
```

### First boot

On the very first start the server exposes its initialization endpoints without
authentication and creates `$STATE_PATH/.first_boot.lock` (i.e.
`/var/lib/cosmox-server/state/.first_boot.lock` by default) once the
initialization succeeded. As long as this file is missing (or the data volume is
empty) the server stays in first-boot mode, therefore **do not expose the port
to the public internet before the initialization is finished**.

### Reverse proxy

Terminate TLS in front of cosmox and forward to the container port:

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

## 2. Prebuilt image only

```bash
docker run -d \
    --name cosmox \
    --restart unless-stopped \
    -p 8080:8080 \
    -v cosmox-data:/var/lib/cosmox-server \
    -v cosmox-log:/var/log/cosmox-server \
    -e COSMX_DATABASE__HOST=10.0.0.5 \
    -e COSMX_DATABASE__USER=cosmox \
    -e COSMX_DATABASE__PASSWORD='***' \
    -e COSMX_DATABASE__DATABASE=cosmox \
    ghcr.io/<owner>/<repo>:<tag>
```

The image runs as the unprivileged `cosmox` user (uid/gid `10001`) and declares
a health check against `GET /api/system/info`; the container is reported as
healthy only when the web API answers.

## 3. Local development

```bash
cp .env.example .env   # only needed for docker compose
cargo run              # reads ./application.yaml, writes logs into test/log
```

When no `application.yaml` is reachable, the built-in defaults are used
(`server.host = 127.0.0.1`, `server.port = 8080`).

## Release pipeline

`.github/workflows/ci.yaml` runs `fmt`, `clippy`, `test` and `doc` on every
pull request. Tag pushes (`v*`) additionally cross-build the binaries and build
the Docker image with `docker/build-push-action`, pushing to
`ghcr.io/<owner>/<repo>:<tag>`.

The image build keeps using the repository root as build context and
`./Dockerfile` as the Dockerfile, so the pipeline is unaffected by local
deployment files (`docker-compose.yml`, `.env`, ...), which are excluded from the
Docker context through `.dockerignore`.
