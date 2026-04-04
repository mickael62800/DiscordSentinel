# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/cache-worker/Cargo.toml services/workers/cache-worker/Cargo.lock* /app/services/workers/cache-worker/
RUN mkdir -p services/workers/cache-worker/src && echo "fn main() {}" > services/workers/cache-worker/src/main.rs && cd services/workers/cache-worker && cargo build --release && rm -rf src

COPY services/workers/cache-worker/src /app/services/workers/cache-worker/src
RUN cd services/workers/cache-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/cache-worker/target/release/sentinel-cache-worker /usr/local/bin/sentinel-cache-worker

ENTRYPOINT ["sentinel-cache-worker"]
