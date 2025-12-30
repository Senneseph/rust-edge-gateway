# Rust Edge Gateway - Production Container with Rust Toolchain
# Multi-stage build: toolchain layers are cached, only binary transfers on updates

# =============================================================================
# Stage 1: Build the application
# =============================================================================
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release --bin rust-edge-gateway

# =============================================================================
# Stage 2: Runtime image with Rust toolchain for handler compilation
# =============================================================================
FROM rust:1.92-slim-bookworm

# Install runtime dependencies (this layer is cached after first deploy)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the compiled binary and static assets (small, changes frequently)
COPY --from=builder /build/target/release/rust-edge-gateway ./rust-edge-gateway
COPY --from=builder /build/static ./static

# Expose ports
# 8080 - Main gateway
# 8081 - Admin interface
EXPOSE 8080 8081

CMD ["./rust-edge-gateway"]
