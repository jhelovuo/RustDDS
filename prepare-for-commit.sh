#!/bin/bash

# Run formatter
echo cargo +nightly fmt
cargo +nightly fmt

# Run linter
echo cargo +nightly clippy --tests --examples
cargo +nightly clippy --tests --examples

# Run linter with all features, including security
echo cargo +nightly clippy --tests --examples --all-features
cargo +nightly clippy --tests --examples --all-features