# ── Build stage (Debian pour compatibilite ONNX Runtime) ──
FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libstdc++6 libstdc++-12-dev ca-certificates g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs && cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release

# ── Runtime stage ──
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sentinel-api /usr/local/bin/sentinel-api
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
EXPOSE 3000

ENTRYPOINT ["sentinel-api"]
