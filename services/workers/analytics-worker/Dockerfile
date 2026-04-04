# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/analytics-worker/Cargo.toml services/workers/analytics-worker/Cargo.lock* /app/services/workers/analytics-worker/
RUN mkdir -p services/workers/analytics-worker/src && echo "fn main() {}" > services/workers/analytics-worker/src/main.rs && cd services/workers/analytics-worker && cargo build --release && rm -rf src

COPY services/workers/analytics-worker/src /app/services/workers/analytics-worker/src
RUN cd services/workers/analytics-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/analytics-worker/target/release/sentinel-analytics-worker /usr/local/bin/sentinel-analytics-worker

ENTRYPOINT ["sentinel-analytics-worker"]
