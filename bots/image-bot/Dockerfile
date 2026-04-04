FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/image-bot/Cargo.toml bots/image-bot/Cargo.lock* /app/bots/image-bot/
RUN mkdir -p bots/image-bot/src && echo "fn main() {}" > bots/image-bot/src/main.rs && cd bots/image-bot && cargo build --release && rm -rf src
COPY bots/image-bot/src /app/bots/image-bot/src
RUN cd bots/image-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/image-bot/target/release/image-bot /usr/local/bin/image-bot
ENTRYPOINT ["image-bot"]
