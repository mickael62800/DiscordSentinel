FROM rust:1.88-alpine AS builder
RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static
WORKDIR /app
COPY bots/shared /app/bots/shared
COPY bots/audit-bot/Cargo.toml bots/audit-bot/Cargo.lock* /app/bots/audit-bot/
RUN mkdir -p bots/audit-bot/src && echo "fn main() {}" > bots/audit-bot/src/main.rs && cd bots/audit-bot && cargo build --release && rm -rf src
COPY bots/audit-bot/src /app/bots/audit-bot/src
RUN cd bots/audit-bot && touch src/main.rs && cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/bots/audit-bot/target/release/audit-bot /usr/local/bin/audit-bot
ENTRYPOINT ["audit-bot"]
