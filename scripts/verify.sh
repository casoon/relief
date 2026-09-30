#!/usr/bin/env bash
# Prüft lokal, was die CI beim Release prüft (vor jedem Push).
set -euo pipefail

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p relief-cdp
