# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/audit-cache-worker/Cargo.toml services/workers/audit-cache-worker/Cargo.lock* /app/services/workers/audit-cache-worker/
RUN mkdir -p services/workers/audit-cache-worker/src && echo "fn main() {}" > services/workers/audit-cache-worker/src/main.rs && cd services/workers/audit-cache-worker && cargo build --release && rm -rf src

COPY services/workers/audit-cache-worker/src /app/services/workers/audit-cache-worker/src
RUN cd services/workers/audit-cache-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/audit-cache-worker/target/release/sentinel-audit-cache-worker /usr/local/bin/sentinel-audit-cache-worker

ENTRYPOINT ["sentinel-audit-cache-worker"]
