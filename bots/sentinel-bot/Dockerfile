# Bot Discord unifie — fusion des 15 anciens bots en un seul binaire.
# Pattern cargo-chef + BuildKit cache mounts pour des rebuilds rapides.
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
RUN apk add --no-cache \
    musl-dev pkgconf openssl-dev openssl-libs-static \
    protoc protobuf-dev \
    make g++ linux-headers
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json --bin sentinel-bot
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --bin sentinel-bot && \
    cp target/release/sentinel-bot /tmp/sentinel-bot

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /tmp/sentinel-bot /usr/local/bin/sentinel-bot
# Assets blackjack (cartes JPG) — cherche dans ./assets/cards au runtime.
COPY bots/sentinel-bot/assets /app/assets
WORKDIR /app
ENTRYPOINT ["sentinel-bot"]
