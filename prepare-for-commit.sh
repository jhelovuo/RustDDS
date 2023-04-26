#!/bin/bash

# Run formatter
cargo +nightly fmt

# Run linter
cargo +nightly clippy