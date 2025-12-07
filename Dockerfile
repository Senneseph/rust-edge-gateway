# Edge Gateway - Production Container with Rust Toolchain
# Includes Rust for handler compilation at runtime
FROM rust:1.83-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY . .

# Build the release binary
RUN cargo build --release --bin edge-hive

# Expose ports
# 8080 - Main gateway
# 8081 - Admin interface
EXPOSE 8080 8081

# Run the pre-built binary
CMD ["./target/release/edge-hive"]

