# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/coude-worker/Cargo.toml services/workers/coude-worker/Cargo.lock* /app/services/workers/coude-worker/
RUN mkdir -p services/workers/coude-worker/src && echo "fn main() {}" > services/workers/coude-worker/src/main.rs && cd services/workers/coude-worker && cargo build --release && rm -rf src

COPY services/workers/coude-worker/src /app/services/workers/coude-worker/src
RUN cd services/workers/coude-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/coude-worker/target/release/sentinel-coude-worker /usr/local/bin/sentinel-coude-worker

ENTRYPOINT ["sentinel-coude-worker"]
