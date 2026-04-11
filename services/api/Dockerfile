# Phase 7A optimisations — cargo-chef + BuildKit cache mounts.
#
# API = Debian Trixie (requis pour ONNX Runtime / glibc 2.38+).
# Le workspace racine permet un builder partage : deps compilees une fois et
# cachees entre builds via --mount=type=cache. Rebuild apres modif source ~30s.
#
# Contexte de build = racine du repo car sentinel-api est membre du workspace
# et depend de sentinel-proto via path.

FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential pkg-config libssl-dev ca-certificates g++ \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook des dependances (cacheable via BuildKit mount)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json --bin sentinel-api
# Build reel
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --bin sentinel-api && \
    cp target/release/sentinel-api /tmp/sentinel-api && \
    cp -r services/api/migrations /tmp/migrations

# ── Runtime stage ──
FROM debian:trixie-slim

# Phase 7A opt B.2 : grpc_health_probe pour le healthcheck Docker gRPC.
# Binaire statique ~10 MB, injecte via wget depuis les releases officielles.
ARG GRPC_HEALTH_PROBE_VERSION=v0.4.24
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 libstdc++6 wget \
    && wget -q -O /usr/local/bin/grpc_health_probe \
       "https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64" \
    && chmod +x /usr/local/bin/grpc_health_probe \
    && apt-get purge -y wget \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/sentinel-api /usr/local/bin/sentinel-api
COPY --from=builder /tmp/migrations /app/migrations

WORKDIR /app
EXPOSE 3000
EXPOSE 50051
# Phase 7A : 3000 = HTTP/Axum ; 50051 = gRPC/tonic.

ENTRYPOINT ["sentinel-api"]
