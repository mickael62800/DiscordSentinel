# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/moderation-worker/Cargo.toml services/workers/moderation-worker/Cargo.lock* /app/services/workers/moderation-worker/
RUN mkdir -p services/workers/moderation-worker/src && echo "fn main() {}" > services/workers/moderation-worker/src/main.rs && cd services/workers/moderation-worker && cargo build --release && rm -rf src

COPY services/workers/moderation-worker/src /app/services/workers/moderation-worker/src
RUN cd services/workers/moderation-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/moderation-worker/target/release/sentinel-moderation-worker /usr/local/bin/sentinel-moderation-worker

ENTRYPOINT ["sentinel-moderation-worker"]
