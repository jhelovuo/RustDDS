#!/bin/bash

# Run formatter
echo cargo +nightly fmt
cargo +nightly fmt

# Run linter
echo cargo +nightly clippy --tests --examples
cargo +nightly clippy --tests --examples

# Run linter without default features (=without security in the security branch)
echo cargo +nightly clippy --no-default-features --tests --examples
cargo +nightly clippy --no-default-features --tests --examples