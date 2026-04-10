# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/discord-audit-sync-worker/Cargo.toml services/workers/discord-audit-sync-worker/Cargo.lock* /app/services/workers/discord-audit-sync-worker/
RUN mkdir -p services/workers/discord-audit-sync-worker/src && echo "fn main() {}" > services/workers/discord-audit-sync-worker/src/main.rs && cd services/workers/discord-audit-sync-worker && cargo build --release && rm -rf src

COPY services/workers/discord-audit-sync-worker/src /app/services/workers/discord-audit-sync-worker/src
RUN cd services/workers/discord-audit-sync-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/discord-audit-sync-worker/target/release/sentinel-discord-audit-sync-worker /usr/local/bin/sentinel-discord-audit-sync-worker

ENTRYPOINT ["sentinel-discord-audit-sync-worker"]
