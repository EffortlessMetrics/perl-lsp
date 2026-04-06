# perl-lsp Docker image — Perl runtime + perllsp LSP server
#
# Stage 1: Build the perllsp binary from source
# Stage 2: Minimal runtime image with Perl installed
#
# Usage (stdio LSP transport):
#   docker run --rm -i -v "${PWD}:/workspace" ghcr.io/effortlessmetrics/perl-lsp:latest-perl
#
# Build locally:
#   docker build -f .docker/perl-lsp/Dockerfile -t perl-lsp:local .

# ─────────────────────────────────────────────────────────────────────────────
# Stage 1: builder
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests first for layer-cache efficiency.
# We copy the full tree because the workspace has 130+ crates; selective
# copy would be fragile and rarely faster.
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ ./.cargo/
COPY crates/ ./crates/
COPY xtask/ ./xtask/

# Build the public perllsp binary in release mode.
RUN cargo build -p perllsp --release --bin perllsp

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2: runtime
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Perl runtime + common CPAN modules useful for real projects.
# cpanminus bootstraps further module installs inside the container.
RUN apt-get update && apt-get install -y --no-install-recommends \
    perl \
    perl-modules-5.36 \
    libmoose-perl \
    cpanminus \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled LSP server binary.
COPY --from=builder /build/target/release/perllsp /usr/local/bin/perllsp

RUN chmod +x /usr/local/bin/perllsp

# Non-root user for safety.
RUN useradd --create-home --uid 10001 --shell /bin/bash appuser

# Default workspace mount point — callers bind-mount their Perl project here.
RUN mkdir -p /workspace && chown appuser:appuser /workspace

WORKDIR /workspace
USER appuser

# LSP communicates via stdin/stdout — no port exposure needed.
# Pass --help to see available options.
ENTRYPOINT ["perllsp"]
