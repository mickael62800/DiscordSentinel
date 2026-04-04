FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/security-bot/Cargo.toml bots/security-bot/Cargo.lock* /app/bots/security-bot/
RUN mkdir -p bots/security-bot/src && echo "fn main() {}" > bots/security-bot/src/main.rs && cd bots/security-bot && cargo build --release && rm -rf src
COPY bots/security-bot/src /app/bots/security-bot/src
RUN cd bots/security-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/security-bot/target/release/security-bot /usr/local/bin/security-bot
ENTRYPOINT ["security-bot"]
