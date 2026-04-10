# -- Build stage --
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY services/workers/worker-common /app/services/workers/worker-common
COPY services/workers/temp-roles-worker/Cargo.toml services/workers/temp-roles-worker/Cargo.lock* /app/services/workers/temp-roles-worker/
RUN mkdir -p services/workers/temp-roles-worker/src && echo "fn main() {}" > services/workers/temp-roles-worker/src/main.rs && cd services/workers/temp-roles-worker && cargo build --release && rm -rf src

COPY services/workers/temp-roles-worker/src /app/services/workers/temp-roles-worker/src
RUN cd services/workers/temp-roles-worker && touch src/main.rs && cargo build --release

# -- Runtime stage --
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/services/workers/temp-roles-worker/target/release/sentinel-temp-roles-worker /usr/local/bin/sentinel-temp-roles-worker

ENTRYPOINT ["sentinel-temp-roles-worker"]
