FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/coude-bot/Cargo.toml bots/coude-bot/Cargo.lock* /app/bots/coude-bot/
RUN mkdir -p bots/coude-bot/src && echo "fn main() {}" > bots/coude-bot/src/main.rs && cd bots/coude-bot && cargo build --release && rm -rf src
COPY bots/coude-bot/src /app/bots/coude-bot/src
RUN cd bots/coude-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/coude-bot/target/release/coude-bot /usr/local/bin/coude-bot
ENTRYPOINT ["coude-bot"]
