FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/roles-bot/Cargo.toml bots/roles-bot/Cargo.lock* /app/bots/roles-bot/
RUN mkdir -p bots/roles-bot/src && echo "fn main() {}" > bots/roles-bot/src/main.rs && cd bots/roles-bot && cargo build --release && rm -rf src
COPY bots/roles-bot/src /app/bots/roles-bot/src
RUN cd bots/roles-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/roles-bot/target/release/roles-bot /usr/local/bin/roles-bot
ENTRYPOINT ["roles-bot"]
