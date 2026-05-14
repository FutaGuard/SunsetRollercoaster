# syntax=docker/dockerfile:1.7

# ---- builder ----
FROM rust:1.92-slim-bookworm AS chef
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config curl ca-certificates && \
    rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin sunset-api

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/sunset-api /usr/local/bin/sunset-api

ENV RUST_LOG=info
EXPOSE 8080

# env_config.yml must be mounted at /app/env_config.yml
CMD ["sunset-api"]
