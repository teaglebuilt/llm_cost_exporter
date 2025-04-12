FROM rust:1.86-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
COPY .env.sample .env.sample

RUN cargo build --release && \
    cargo install --path . && \
    rm -rf target/release/deps/llm_cost_exporter*

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/llm_cost_exporter /usr/local/bin/
COPY --from=builder /app/.env.sample /app/.env

RUN useradd -m -u 1000 llmuser && \
    chown -R llmuser:llmuser /app

USER llmuser
WORKDIR /app

HEALTHCHECK --interval=30s --timeout=3s \
    CMD curl -f http://localhost:8000/health || exit 1

EXPOSE 8000

ENTRYPOINT ["llm_cost_exporter"]
