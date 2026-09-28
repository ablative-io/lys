# Native ARM build tools matching Rauthy v0.36.2's upstream Rust and Debian versions.
FROM rust:1.95.0-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends clang libpam0g-dev librust-libudev-sys-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /work
