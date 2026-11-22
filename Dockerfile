# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/sigma-core/Cargo.toml crates/sigma-core/Cargo.toml
COPY crates/sigma-server/Cargo.toml crates/sigma-server/Cargo.toml
COPY crates/sigma-cli/Cargo.toml crates/sigma-cli/Cargo.toml

COPY crates/sigma-core/src crates/sigma-core/src
COPY crates/sigma-server/src crates/sigma-server/src
COPY crates/sigma-cli/src crates/sigma-cli/src
COPY docs/openapi.json docs/openapi.json

RUN cargo build --release -p sigma-server --bin sigma-serve

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 sigma

WORKDIR /app
COPY --from=builder /app/target/release/sigma-serve /usr/local/bin/sigma-serve

USER sigma
ENV SIGMA_BIND=0.0.0.0:8787 \
    RUST_LOG=sigma_serve=info,tower_http=info

EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/sigma-serve"]
