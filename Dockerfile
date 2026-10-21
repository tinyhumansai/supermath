# The supermath solving harness: `crates/supermath-agent` with the OpenHuman
# runtime embedded, built from the `vendor/openhuman` submodule.
#
#   docker compose build agent
#
# Initialise submodules first (`git submodule update --init --recursive`): the
# build context carries `vendor/openhuman` and every crate it vendors.

# Matches the toolchain OpenHuman's own image builds with.
FROM rust:1.99.1-bookworm AS builder

# OpenHuman's build dependencies, as its Dockerfile installs them. The X11 and
# input libraries are linked by core crates even in a headless build.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    libxdo-dev \
    libxtst-dev \
    libx11-dev \
    libevdev-dev \
    clang \
    mold \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY vendor/openhuman/ vendor/openhuman/
COPY crates/supermath-agent/ crates/supermath-agent/

WORKDIR /build/crates/supermath-agent
# Cache mounts keep the registry and the (large) OpenHuman build between image
# builds; the binary is copied out because the target cache is not in the image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/crates/supermath-agent/target \
    cargo build --release --locked --bin supermath-agent \
    && cp target/release/supermath-agent /usr/local/bin/supermath-agent

# OpenHuman's bundled TinyBus modules, staged the way its own image stages them.
FROM node:24-bookworm-slim AS modules
WORKDIR /repo
COPY vendor/openhuman/scripts/lib/module-pins.mjs scripts/lib/module-pins.mjs
COPY vendor/openhuman/scripts/ci/self-hosted/test-module-assets.mjs scripts/ci/self-hosted/test-module-assets.mjs
COPY vendor/openhuman/scripts/release/stage-modules.mjs scripts/release/stage-modules.mjs
COPY vendor/openhuman/crates/openhuman-core/src/modules/registry.rs crates/openhuman-core/src/modules/registry.rs
COPY vendor/openhuman/crates/openhuman-core/src/modules/registry/ crates/openhuman-core/src/modules/registry/
RUN node scripts/release/stage-modules.mjs --output /bundled-modules

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libasound2 \
    libxdo3 \
    libxtst6 \
    libx11-6 \
    libevdev2 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 10001 supermath \
 && useradd --uid 10001 --gid 10001 --create-home --shell /bin/bash supermath \
 && mkdir -p /data /problems \
 && chown supermath:supermath /data

COPY --from=builder /usr/local/bin/supermath-agent /usr/local/bin/supermath-agent
COPY --from=modules /bundled-modules /opt/openhuman/bundled-modules

ENV OPENHUMAN_BUNDLED_MODULES=/opt/openhuman/bundled-modules \
    OPENHUMAN_DOCKER=1 \
    OPENHUMAN_WORKSPACE=/data/openhuman \
    SUPERMATH_WORKSPACE=/data/openhuman \
    HOME=/home/supermath \
    RUST_LOG=info

USER supermath
WORKDIR /data
ENTRYPOINT ["supermath-agent"]
CMD ["--help"]
