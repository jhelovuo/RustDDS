#!/bin/bash

# Run formatter
echo cargo +nightly fmt
cargo +nightly fmt

# Run linter
echo cargo +nightly clippy
cargo +nightly clippy

# Run linter without default features (=without security in the security branch)
echo cargo +nightly clippy --no-default-features
cargo +nightly clippy --no-default-features

# Run linter on examples
#echo cargo +nightly clippy --examples
#cargo +nightly clippy --examples