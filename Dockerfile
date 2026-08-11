# Build stage
FROM rust:1.80-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/pankh /usr/local/bin/pankh

WORKDIR /workspace

ENTRYPOINT ["pankh"]
CMD ["--help"]
