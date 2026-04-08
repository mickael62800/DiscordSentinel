FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/blackjack-bot/Cargo.toml bots/blackjack-bot/Cargo.lock* /app/bots/blackjack-bot/
RUN mkdir -p bots/blackjack-bot/src && echo "fn main() {}" > bots/blackjack-bot/src/main.rs && cd bots/blackjack-bot && cargo build --release && rm -rf src
COPY bots/blackjack-bot/src /app/bots/blackjack-bot/src
RUN cd bots/blackjack-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/blackjack-bot/target/release/blackjack-bot /usr/local/bin/blackjack-bot
ENTRYPOINT ["blackjack-bot"]
