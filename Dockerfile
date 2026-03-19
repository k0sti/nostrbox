# syntax=docker/dockerfile:1.7

# Build from the parent workspace so path dependencies remain available.
# Expected layout:
#   /workspace/nostrbox
#   /workspace/nostr-1
#   /workspace/rust-contextvm-sdk
#
# Example:
#   docker build -f nostrbox/Dockerfile -t nostrbox ~/work

FROM rust:1.85-slim-bookworm AS rust-builder
WORKDIR /workspace/nostrbox

# Copy manifests first for better layer caching.
COPY nostrbox/Cargo.toml nostrbox/Cargo.lock ./
COPY nostrbox/crates/core/Cargo.toml crates/core/Cargo.toml
COPY nostrbox/crates/nostr/Cargo.toml crates/nostr/Cargo.toml
COPY nostrbox/crates/store/Cargo.toml crates/store/Cargo.toml
COPY nostrbox/crates/contextvm/Cargo.toml crates/contextvm/Cargo.toml
COPY nostrbox/crates/relay/Cargo.toml crates/relay/Cargo.toml
COPY nostrbox/crates/server/Cargo.toml crates/server/Cargo.toml

# Path dependencies outside the repo.
COPY nostr-1 /workspace/nostr-1
COPY rust-contextvm-sdk /workspace/rust-contextvm-sdk

# Copy sources.
COPY nostrbox/crates crates

RUN cargo build --release --bin nostrbox-server

FROM oven/bun:1 AS web-builder
WORKDIR /workspace/nostrbox/web
COPY nostrbox/web/package.json nostrbox/web/bun.lock ./
RUN bun install --frozen-lockfile
COPY nostrbox/web .
RUN bun run build

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-builder /workspace/nostrbox/target/release/nostrbox-server /usr/local/bin/nostrbox-server
COPY --from=web-builder /workspace/nostrbox/web/dist /app/web/dist
COPY nostrbox/docker/nostrbox.toml /etc/nostrbox/nostrbox.toml

EXPOSE 3000 7777
ENV NOSTRBOX_CONFIG=/etc/nostrbox/nostrbox.toml
ENTRYPOINT ["/usr/local/bin/nostrbox-server"]
