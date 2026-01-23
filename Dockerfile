# ==================================================================================================
# Stage 1: Builder
# ==================================================================================================
FROM rust:1-slim-bookworm AS builder

# Install system dependencies for building
RUN apt-get update \
    && apt-get install -y pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Print Rust version for debugging
RUN rustc --version \
    && cargo --version

# Create dummy workspace to cache dependencies
WORKDIR /app
RUN cargo init --name aircade-api \
    && mkdir -p migration/src \
    && echo "fn main() {}" > migration/src/main.rs \
    && echo "pub fn lib() {}" > migration/src/lib.rs

# Copy all manifests (root + workspace members)
COPY Cargo.toml Cargo.lock ./
COPY migration/Cargo.toml migration/Cargo.toml

# Build dependencies only (caching layer)
RUN cargo build --release \
    && rm src/*.rs \
    && rm migration/src/*.rs

# Copy full source code
COPY . .

# Build the application
# Force rebuild of application code (dependencies are cached)
RUN rm -f ./target/release/deps/aircade_api* \
    && rm -f ./target/release/deps/libmigration* \
    && cargo build --release --locked

# ==================================================================================================
# Stage 2: Runtime
# ==================================================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update \
    && apt-get install -y ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 aircade \
    && mkdir -p /app \
    && chown -R aircade:aircade /app

# Switch to non-root user
USER aircade
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/aircade-api /app/aircade-api

# Expose the port (Railway will override with $PORT)
EXPOSE 3000

# Run the binary
CMD ["/app/aircade-api"]
