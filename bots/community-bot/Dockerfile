FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/community-bot/Cargo.toml bots/community-bot/Cargo.lock* /app/bots/community-bot/
RUN mkdir -p bots/community-bot/src && echo "fn main() {}" > bots/community-bot/src/main.rs && cd bots/community-bot && cargo build --release && rm -rf src
COPY bots/community-bot/src /app/bots/community-bot/src
RUN cd bots/community-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/community-bot/target/release/community-bot /usr/local/bin/community-bot
ENTRYPOINT ["community-bot"]
