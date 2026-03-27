# ── Build stage ──
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ──
FROM alpine:3.21

RUN apk add --no-cache ca-certificates libstdc++

# ONNX Runtime est telecharge automatiquement par le crate ort au build.
# La lib dynamique est linkee dans le binaire.
# libstdc++ est necessaire pour le runtime ONNX.

COPY --from=builder /app/target/release/sentinel-api /usr/local/bin/sentinel-api
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
EXPOSE 3000

ENTRYPOINT ["sentinel-api"]
