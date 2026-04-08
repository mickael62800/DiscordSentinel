FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/cleanup-bot/Cargo.toml bots/cleanup-bot/Cargo.lock* /app/bots/cleanup-bot/
RUN mkdir -p bots/cleanup-bot/src && echo "fn main() {}" > bots/cleanup-bot/src/main.rs && cd bots/cleanup-bot && cargo build --release && rm -rf src
COPY bots/cleanup-bot/src /app/bots/cleanup-bot/src
RUN cd bots/cleanup-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/cleanup-bot/target/release/cleanup-bot /usr/local/bin/cleanup-bot
ENTRYPOINT ["cleanup-bot"]
