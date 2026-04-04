FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/voice-bot/Cargo.toml bots/voice-bot/Cargo.lock* /app/bots/voice-bot/
RUN mkdir -p bots/voice-bot/src && echo "fn main() {}" > bots/voice-bot/src/main.rs && cd bots/voice-bot && cargo build --release && rm -rf src
COPY bots/voice-bot/src /app/bots/voice-bot/src
RUN cd bots/voice-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/voice-bot/target/release/voice-bot /usr/local/bin/voice-bot
ENTRYPOINT ["voice-bot"]
