# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/appeal-sla-worker/Cargo.toml services/workers/appeal-sla-worker/Cargo.lock* /app/services/workers/appeal-sla-worker/
RUN mkdir -p services/workers/appeal-sla-worker/src && echo "fn main() {}" > services/workers/appeal-sla-worker/src/main.rs && cd services/workers/appeal-sla-worker && cargo build --release && rm -rf src

COPY services/workers/appeal-sla-worker/src /app/services/workers/appeal-sla-worker/src
RUN cd services/workers/appeal-sla-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/appeal-sla-worker/target/release/sentinel-appeal-sla-worker /usr/local/bin/sentinel-appeal-sla-worker

ENTRYPOINT ["sentinel-appeal-sla-worker"]
