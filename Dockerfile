# Build stage
FROM rust:1-bookworm AS builder
WORKDIR /app

# Install libsensors build deps + python3 for build script
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsensors-dev \
    pkg-config \
    python3 \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY static/ ./static/
COPY build.rs build_dashboard.py ./
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install libsensors runtime + wget for healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsensors5 \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Work directory
WORKDIR /app

COPY --from=builder /app/target/release/lm-sensors-web /usr/local/bin/lm-sensors-web
COPY config.example.json /etc/lm-sensors-web/config.json

ENV RUST_LOG=info
ENV CONFIG_PATH=/etc/lm-sensors-web/config.json

EXPOSE 47890

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:47890/api/health || exit 1

USER nobody

# Shell form so $CONFIG_PATH gets expanded
ENTRYPOINT ["lm-sensors-web", "--config", "/etc/lm-sensors-web/config.json"]
CMD []