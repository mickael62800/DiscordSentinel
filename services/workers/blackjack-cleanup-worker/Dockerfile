# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/blackjack-cleanup-worker/Cargo.toml services/workers/blackjack-cleanup-worker/Cargo.lock* /app/services/workers/blackjack-cleanup-worker/
RUN mkdir -p services/workers/blackjack-cleanup-worker/src && echo "fn main() {}" > services/workers/blackjack-cleanup-worker/src/main.rs && cd services/workers/blackjack-cleanup-worker && cargo build --release && rm -rf src

COPY services/workers/blackjack-cleanup-worker/src /app/services/workers/blackjack-cleanup-worker/src
RUN cd services/workers/blackjack-cleanup-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/blackjack-cleanup-worker/target/release/sentinel-blackjack-cleanup-worker /usr/local/bin/sentinel-blackjack-cleanup-worker

ENTRYPOINT ["sentinel-blackjack-cleanup-worker"]
