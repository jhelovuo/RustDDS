#!/bin/bash

# Run tests with default features (no security)
cargo test

# Run tests with the security feature
cargo test --features security