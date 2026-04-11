# Phase 7A optimisations — cargo-chef + BuildKit cache mounts.
# Le workspace racine permet un builder partage : deps compilees une fois et
# cachees entre builds via --mount=type=cache. Rebuild apres modif source ~30s.
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
RUN apk add --no-cache     musl-dev pkgconf openssl-dev openssl-libs-static     protoc protobuf-dev     make g++ linux-headers
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook des dependances uniquement (couche cacheable)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked     --mount=type=cache,target=/app/target,sharing=locked     cargo chef cook --release --recipe-path recipe.json --bin security-bot
# Build reel
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked     --mount=type=cache,target=/app/target,sharing=locked     cargo build --release --bin security-bot &&     cp target/release/security-bot /tmp/security-bot

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /tmp/security-bot /usr/local/bin/security-bot
ENTRYPOINT ["security-bot"]
