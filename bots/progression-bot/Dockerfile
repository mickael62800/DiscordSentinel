FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/progression-bot/Cargo.toml bots/progression-bot/Cargo.lock* /app/bots/progression-bot/
RUN mkdir -p bots/progression-bot/src && echo "fn main() {}" > bots/progression-bot/src/main.rs && cd bots/progression-bot && cargo build --release && rm -rf src
COPY bots/progression-bot/src /app/bots/progression-bot/src
RUN cd bots/progression-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/progression-bot/target/release/progression-bot /usr/local/bin/progression-bot
ENTRYPOINT ["progression-bot"]
