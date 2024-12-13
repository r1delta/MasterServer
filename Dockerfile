# Builder stage
FROM rust:1.75-slim as builder

WORKDIR /usr/src/r1ms
COPY . .

# Build with optimizations
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Create non-root user
RUN useradd -r -s /bin/false r1ms

# Install OpenSSL and CA certificates (needed for HTTPS)
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/src/r1ms/target/release/r1ms /usr/local/bin/

# Set ownership and permissions
RUN chown r1ms:r1ms /usr/local/bin/r1ms \
    && chmod +x /usr/local/bin/r1ms

# Switch to non-root user
USER r1ms

# Expose port
EXPOSE 80

# Run the server
CMD ["r1ms"]
