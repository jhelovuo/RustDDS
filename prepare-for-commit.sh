#!/bin/bash

# Run formatter
echo cargo +nightly fmt
cargo +nightly fmt

# Run linter
echo cargo +nightly clippy --tests --examples
cargo +nightly clippy --tests --examples

echo cargo +nightly clippy --tests --examples --features security
cargo +nightly clippy --tests --examples --features security

echo cargo +nightly clippy --tests --examples --features rtps_proxy
cargo +nightly clippy --tests --examples --features rtps_proxy

# Run linter with all features, including security and rtps_proxy
echo cargo +nightly clippy --tests --examples --all-features
cargo +nightly clippy --tests --examples --all-features