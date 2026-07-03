# Build stage
FROM rust:1-bookworm AS builder
WORKDIR /app

# Install libsensors build deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsensors-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install libsensors runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsensors5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/lm-sensors-api /usr/local/bin/lm-sensors-api
COPY config.example.json /etc/lm-sensors-api/config.json

ENV RUST_LOG=info
ENV CONFIG_PATH=/etc/lm-sensors-api/config.json

EXPOSE 47890

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:47890/api/health || exit 1

ENTRYPOINT ["lm-sensors-api"]
CMD []
