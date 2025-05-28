
# Use the official Rust image as a base
FROM rust:latest

# Install system dependencies
RUN apt-get update && apt-get install -y \
    jq \
    curl \
    git \
    pkg-config \
    libssl-dev \
    clang \
    llvm \
    cmake \
    make \
    build-essential \
    && apt-get clean

# Install cargo tools
RUN rustup component add rustfmt clippy

# Install cargo-deny
RUN cargo install cargo-deny

# Install cargo-geiger
RUN cargo install cargo-geiger --locked --force

# Install cargo-fuzz
RUN cargo install cargo-fuzz

# Set working directory
WORKDIR /usr/src/app

# Default command
CMD [ "bash" ]
