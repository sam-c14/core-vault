# Multi-stage Docker build for CoreVault using cargo-chef
FROM rust:1.97-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin corevault

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpq5 libssl3 tzdata \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 10001 corevault && \
    useradd -u 10001 -g corevault -s /bin/false -M corevault

WORKDIR /app
COPY --from=builder /app/target/release/corevault /app/corevault
COPY config ./config
COPY migrations ./migrations

USER 10001:10001
ENV TZ=Africa/Lagos
ENV RUST_LOG=info,corevault=debug

EXPOSE 8080
ENTRYPOINT ["/app/corevault"]
