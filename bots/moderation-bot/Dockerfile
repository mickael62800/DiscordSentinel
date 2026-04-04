FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/moderation-bot/Cargo.toml bots/moderation-bot/Cargo.lock* /app/bots/moderation-bot/
RUN mkdir -p bots/moderation-bot/src && echo "fn main() {}" > bots/moderation-bot/src/main.rs && cd bots/moderation-bot && cargo build --release && rm -rf src
COPY bots/moderation-bot/src /app/bots/moderation-bot/src
RUN cd bots/moderation-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/moderation-bot/target/release/moderation-bot /usr/local/bin/moderation-bot
ENTRYPOINT ["moderation-bot"]
