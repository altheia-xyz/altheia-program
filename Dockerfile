# Anchor + Solana + Rust toolchain for Altheia Identity Program
# Pinned versions; bump deliberately.

FROM rust:1.85-bookworm

# System deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl pkg-config libudev-dev libssl-dev build-essential ca-certificates \
    git python3 jq \
    && rm -rf /var/lib/apt/lists/*

# Solana CLI (Anza release)
ARG SOLANA_VERSION=2.3.0
RUN curl -sSfL "https://release.anza.xyz/v${SOLANA_VERSION}/install" | sh
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Anchor CLI pinned at the release tag (skips avm; avm from main needs edition2024 / Rust 1.85+)
ARG ANCHOR_VERSION=0.31.1
RUN cargo install --git https://github.com/coral-xyz/anchor --tag v${ANCHOR_VERSION} anchor-cli --locked

# Node + pnpm for tests
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g pnpm@9 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Default: drop into bash
CMD ["bash"]
