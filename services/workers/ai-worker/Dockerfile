# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/ai-worker/Cargo.toml services/workers/ai-worker/Cargo.lock* /app/services/workers/ai-worker/
RUN mkdir -p services/workers/ai-worker/src && echo "fn main() {}" > services/workers/ai-worker/src/main.rs && cd services/workers/ai-worker && cargo build --release && rm -rf src

COPY services/workers/ai-worker/src /app/services/workers/ai-worker/src
RUN cd services/workers/ai-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/ai-worker/target/release/sentinel-ai-worker /usr/local/bin/sentinel-ai-worker

ENTRYPOINT ["sentinel-ai-worker"]
