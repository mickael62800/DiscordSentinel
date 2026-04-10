# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/export-worker/Cargo.toml services/workers/export-worker/Cargo.lock* /app/services/workers/export-worker/
RUN mkdir -p services/workers/export-worker/src && echo "fn main() {}" > services/workers/export-worker/src/main.rs && cd services/workers/export-worker && cargo build --release && rm -rf src

COPY services/workers/export-worker/src /app/services/workers/export-worker/src
RUN cd services/workers/export-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/export-worker/target/release/sentinel-export-worker /usr/local/bin/sentinel-export-worker

ENTRYPOINT ["sentinel-export-worker"]
