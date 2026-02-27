FROM rust:slim

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ ./.cargo/
COPY crates/*/Cargo.toml ./crates/*/

# Create dummy main files to allow dependency compilation
RUN find . -name "Cargo.toml" -path "./crates/*" -exec dirname {} \; | \
    xargs -I {} sh -c 'mkdir -p {}/src && echo "fn main() {}" > {}/src/main.rs && echo "" > {}/src/lib.rs'

# Build dependencies (this layer will be cached)
RUN cargo build --release || true

# Copy the actual source code
COPY . .

# Set default concurrency environment
ENV RUST_TEST_THREADS=2
ENV OMP_NUM_THREADS=1
ENV OPENBLAS_NUM_THREADS=1
ENV MKL_NUM_THREADS=1
ENV NUMEXPR_NUM_THREADS=1

# Build the project
RUN cargo build --release

RUN useradd --create-home --uid 10001 appuser
USER appuser