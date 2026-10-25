# The supermath MCP server: `supermath-mcp serve`, the tool domains and their
# wiring.
#
#   docker build -f docker/mcp/Dockerfile -t supermath-mcp .
#   docker run --rm -i -v "$PWD/projects:/projects" -e SUPERMATH_PROJECTS=/projects \
#       supermath-mcp serve my-project
#
# The build context is the repository root; only the workspace manifest, the
# lockfile and `crates/` are copied in.

FROM rust:1.99.1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked -p supermath-mcp \
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
