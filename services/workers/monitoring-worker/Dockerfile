# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/monitoring-worker/Cargo.toml services/workers/monitoring-worker/Cargo.lock* /app/services/workers/monitoring-worker/
RUN mkdir -p services/workers/monitoring-worker/src && echo "fn main() {}" > services/workers/monitoring-worker/src/main.rs && cd services/workers/monitoring-worker && cargo build --release && rm -rf src

COPY services/workers/monitoring-worker/src /app/services/workers/monitoring-worker/src
RUN cd services/workers/monitoring-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/monitoring-worker/target/release/sentinel-monitoring-worker /usr/local/bin/sentinel-monitoring-worker

ENTRYPOINT ["sentinel-monitoring-worker"]
