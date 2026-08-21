# syntax=docker/dockerfile:1.7

# Container image for the Loctree MCP server. Two stages: build loctree-mcp
# from the workspace, then ship that one binary on a slim Debian runtime under
# a non-root uid. VCS_REF is baked in so the server can report its own commit.

ARG VCS_REF=unknown

# Build stage: full Rust toolchain plus protoc, producing a --locked release
# build of loctree-mcp and staging it at /out.
FROM rust:1.93.0-bookworm AS builder

ARG VCS_REF
ENV LOCTREE_MCP_GIT_COMMIT=${VCS_REF}
ENV LOCTREE_MCP_GIT_DIRTY=false

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --locked --release --package loctree-mcp \
    && install -D -m 0755 target/release/loctree-mcp /out/loctree-mcp

# Runtime stage: the binary plus its only runtime needs (CA roots and git),
# running as uid 65532. /workspace is pre-marked safe.directory so a mounted
# repo owned by another uid is not refused by git.
FROM debian:bookworm-slim AS runtime

ARG VCS_REF

LABEL org.opencontainers.image.source="https://github.com/Loctree/loctree-mcp" \
      org.opencontainers.image.description="Loctree structural code intelligence MCP server" \
      org.opencontainers.image.licenses="BUSL-1.1" \
      org.opencontainers.image.revision="${VCS_REF}"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && git config --system --add safe.directory /workspace \
    && install -d -o 65532 -g 65532 /home/loctree /data /workspace

COPY --from=builder /out/loctree-mcp /usr/local/bin/loctree-mcp

# Keep snapshots out of the mounted source tree. Glama persists /data when a
# volume is enabled; local Docker clients can use a named volume there too.
ENV LOCT_CACHE_DIR=/data/loctree-cache
ENV HOME=/home/loctree

USER 65532:65532
WORKDIR /workspace

ENTRYPOINT ["loctree-mcp"]
CMD ["--root", "/workspace"]
