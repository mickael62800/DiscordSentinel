# ── Build stage ──
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app

# Copier les dependances locales
COPY services/workers/worker-common /app/services/workers/worker-common

# Deps cache
COPY services/gateway/Cargo.toml services/gateway/Cargo.lock* /app/services/gateway/
RUN mkdir -p services/gateway/src && echo "fn main() {}" > services/gateway/src/main.rs && cd services/gateway && cargo build --release && rm -rf src

# Build reel
COPY services/gateway/src /app/services/gateway/src
RUN cd services/gateway && touch src/main.rs && cargo build --release

# ── Runtime stage ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/gateway/target/release/sentinel-gateway /usr/local/bin/sentinel-gateway

ENTRYPOINT ["sentinel-gateway"]
