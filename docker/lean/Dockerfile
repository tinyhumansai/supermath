# syntax=docker/dockerfile:1
#
# The supermath Lean service: a pinned Lean toolchain and Mathlib, the
# leanprover-community REPL, and a small HTTP wrapper (server.py). See
# docs/specs/toolkit/lean/README.md and docs/adr/0003-run-lean-as-a-pinned-service.md.
#
#   docker build -t supermath-lean docker/lean
#
# The first build downloads Mathlib's prebuilt cache (several gigabytes) and is
# slow; nothing at runtime compiles Mathlib.

FROM debian:bookworm-slim AS build

# The REPL commit built against the toolchain in lean-toolchain (and so the
# Mathlib tag in lakefile.toml). The REPL repository has no `v4.20.0` tag, so
# this pins the head of its `bump_to_v4.20.0` branch by commit.
ARG REPL_REV=16054ca34b80803d6621dccd11882ca2ea03e215

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl git build-essential \
 && rm -rf /var/lib/apt/lists/*

ENV ELAN_HOME=/opt/elan
ENV PATH=/opt/elan/bin:$PATH
RUN curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh \
  | sh -s -- -y --default-toolchain none

# Mathlib at the pinned revision, with its prebuilt oleans. `lake-manifest.json`
# is copied when it exists, so a committed manifest pins the whole dependency
# graph; without one `lake update` resolves the tag in lakefile.toml.
WORKDIR /opt/lean-env
COPY lean-toolchain lakefile.toml lake-manifest.jso[n] ./
RUN elan toolchain install "$(cat lean-toolchain)" \
 && elan default "$(cat lean-toolchain)" \
 && { [ -f lake-manifest.json ] || lake update; } \
 && lake exe cache get \
 && git -C .lake/packages/mathlib rev-parse HEAD > /opt/mathlib-rev \
 && cp lean-toolchain /opt/toolchain

# The REPL, built with the same toolchain.
RUN git init -q /opt/repl \
 && cd /opt/repl \
 && git fetch -q --depth 1 https://github.com/leanprover-community/repl "$REPL_REV" \
 && git checkout -q FETCH_HEAD \
 && lake build

FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates git python3 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --create-home --shell /usr/sbin/nologin lean

COPY --from=build /opt /opt
ENV ELAN_HOME=/opt/elan
ENV PATH=/opt/elan/bin:$PATH
ENV PROJECTS_DIR=/projects
ENV LEAN_ENV=/opt/lean-env
ENV REPL_BIN=/opt/repl/.lake/build/bin/repl

COPY build.py normalize.py repl.py server.py /srv/
WORKDIR /srv

USER lean
EXPOSE 3150
HEALTHCHECK --interval=10s --timeout=5s --start-period=120s --retries=12 \
  CMD python3 -c "import urllib.request; urllib.request.urlopen('http://127.0.0.1:3150/health', timeout=4)"
ENTRYPOINT ["python3", "-B", "/srv/server.py"]
