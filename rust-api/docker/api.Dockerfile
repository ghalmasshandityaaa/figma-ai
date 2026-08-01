# syntax=docker/dockerfile:1
FROM rust:1.90-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release --package api

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home appuser

COPY --from=builder /build/target/release/api /usr/local/bin/api
USER appuser
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
