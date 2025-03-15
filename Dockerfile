FROM rust:1.85-slim as builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 
#  Switch to non-root user
# 
RUN useradd -ms /bin/sh c5run 
USER c5run

WORKDIR /usr/src/app
# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Cache dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src/ src/

# Build the application
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

USER root
# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

USER c5run

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/andy-coin /app/andy-coin

# Create directories for logs and data
RUN mkdir -p /app/logs /app/data

# Set the working directory to /app/data where the YAML file will be stored
WORKDIR /app/data

# Set environment variables
ENV RUST_LOG=info

# Run the binary
CMD ["/app/andy-coin"]
