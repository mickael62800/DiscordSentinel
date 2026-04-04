FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/automod-bot/Cargo.toml bots/automod-bot/Cargo.lock* /app/bots/automod-bot/
RUN mkdir -p bots/automod-bot/src && echo "fn main() {}" > bots/automod-bot/src/main.rs && cd bots/automod-bot && cargo build --release && rm -rf src
COPY bots/automod-bot/src /app/bots/automod-bot/src
RUN cd bots/automod-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/automod-bot/target/release/automod-bot /usr/local/bin/automod-bot
ENTRYPOINT ["automod-bot"]
