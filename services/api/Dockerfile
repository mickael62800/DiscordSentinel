# ── Build stage (Debian Trixie pour compatibilite glibc 2.38+ / ONNX Runtime) ──
FROM debian:trixie-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential pkg-config libssl-dev ca-certificates g++ \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.88.0
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs && cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release

# ── Runtime stage ──
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sentinel-api /usr/local/bin/sentinel-api
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
EXPOSE 3000

ENTRYPOINT ["sentinel-api"]
