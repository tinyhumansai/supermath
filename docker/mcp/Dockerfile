# The supermath MCP server: `supermath-mcp serve` and nothing else. It holds
# the tool domains and their wiring, not the agent, so it never compiles
# OpenHuman and the image stays small.
#
#   docker build -f docker/mcp/Dockerfile -t supermath-mcp .
#   docker run --rm -i -v "$PWD/projects:/projects" -e SUPERMATH_PROJECTS=/projects \
#       supermath-mcp serve my-project
#
# The build context is the repository root, but only the workspace manifest and
# the nine crates `supermath-mcp` depends on are copied in. `vendor/openhuman`
# and the OpenHuman crates (`supermath-subagents`, `supermath-agent`) are not.

FROM rust:1.99.1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/supermath-tool crates/supermath-tool
COPY crates/supermath-files crates/supermath-files
COPY crates/supermath-calc crates/supermath-calc
COPY crates/supermath-ledger crates/supermath-ledger
COPY crates/supermath-lean crates/supermath-lean
COPY crates/supermath-literature crates/supermath-literature
COPY crates/supermath-python crates/supermath-python
COPY crates/supermath-web crates/supermath-web
COPY crates/supermath-mcp crates/supermath-mcp
# The root manifest lists every crate as a member and re-roots OpenHuman's
# `[patch]` table at `vendor/openhuman`, which this image does not carry. Keep
# the shared package metadata, dependencies, lints and release profile; drop
# the patch tables and name the one member. Its path dependencies join the
# workspace by themselves. `Cargo.lock` is only a starting point here: cargo
# prunes it to the smaller graph, so this build is not `--locked`.
RUN awk '/^\[patch/ { skip = 1; next } /^\[/ { skip = 0 } !skip' Cargo.toml \
      | sed 's|members = \["crates/\*"\]|members = ["crates/supermath-mcp"]|' > Cargo.toml.mcp \
 && mv Cargo.toml.mcp Cargo.toml
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p supermath-mcp \
 && cp target/release/supermath-mcp /usr/local/bin/supermath-mcp

FROM debian:bookworm-slim
# TLS roots for the arXiv, Semantic Scholar, OEIS and Gemini calls.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --gid 10001 supermath \
 && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin supermath \
 && mkdir -p /projects \
 && chown supermath:supermath /projects
COPY --from=builder /usr/local/bin/supermath-mcp /usr/local/bin/supermath-mcp
USER supermath
# Where the project workspaces live; mount the same volume the lean and python
# services mount at /projects.
ENV SUPERMATH_PROJECTS=/projects
# MCP speaks over standard input and output: run it with `-i`, no tty.
ENTRYPOINT ["supermath-mcp"]
CMD ["serve"]
