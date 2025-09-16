# --- Builder ---
FROM rust:1.89-slim-bookworm AS builder

ARG SOLANA_VERSION=2.1.0
ARG ANCHOR_VERSION=0.31.1

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:/usr/local/bin:${PATH}"

# deps (минимальный набор для anchor + solana)
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev git python3 \
    libudev-dev ca-certificates wget gnupg bzip2 xz-utils \
    protobuf-compiler libprotobuf-dev curl && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

# Solana CLI (через Anza installer)
RUN curl -sSfL https://release.anza.xyz/stable/install | sh -s - v${SOLANA_VERSION}
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Anchor CLI
RUN cargo install anchor-cli@${ANCHOR_VERSION} --locked --force

WORKDIR /project

# Подготовка Cargo deps
COPY Cargo.toml ./
COPY ./w3b2-bridge-program/Cargo.toml ./w3b2-bridge-program/
COPY ./w3b2-connector/Cargo.toml ./w3b2-connector/
RUN mkdir -p w3b2-bridge-program/src w3b2-connector/src && \
    touch w3b2-bridge-program/src/lib.rs && \
    touch w3b2-connector/src/main.rs && \
    cargo fetch

# Копируем код
COPY . .

# Сборка артефактов
RUN anchor build -- --verbose && \
    cargo build --release --workspace

COPY deploy.sh /project/deploy.sh
RUN chmod +x /project/deploy.sh