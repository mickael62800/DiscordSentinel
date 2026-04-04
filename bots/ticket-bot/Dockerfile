FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/ticket-bot/Cargo.toml bots/ticket-bot/Cargo.lock* /app/bots/ticket-bot/
RUN mkdir -p bots/ticket-bot/src && echo "fn main() {}" > bots/ticket-bot/src/main.rs && cd bots/ticket-bot && cargo build --release && rm -rf src
COPY bots/ticket-bot/src /app/bots/ticket-bot/src
RUN cd bots/ticket-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/ticket-bot/target/release/ticket-bot /usr/local/bin/ticket-bot
ENTRYPOINT ["ticket-bot"]
