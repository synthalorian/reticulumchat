# syntax=docker/dockerfile:1

# Multi-stage Dockerfile for ReticulumChat
# Builds a minimal container with the reticulumchat binary

# Stage 1: Build
FROM rust:1.82-slim-bookworm AS builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary (without GUI feature for headless container)
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash reticulum

# Copy binary from builder
COPY --from=builder /app/target/release/reticulumchat /usr/local/bin/reticulumchat

# Set up data directory
RUN mkdir -p /data && chown -R reticulum:reticulum /data

USER reticulum

# Default data volume
VOLUME ["/data"]

# Expose default Reticulum port
EXPOSE 3742

# Set default environment
ENV RUST_LOG=info
ENV RETICULUMCHAT_IDENTITY=/data/identity
ENV RETICULUMCHAT_HOST=127.0.0.1
ENV RETICULUMCHAT_PORT=3742

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD reticulumchat --help > /dev/null || exit 1

ENTRYPOINT ["reticulumchat"]
CMD ["--host", "127.0.0.1", "--port", "3742", "--identity", "/data/identity"]
