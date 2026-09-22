#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

FORMAT="${1:-html}"

if ! command -v cargo-llvm-cov &>/dev/null; then
    echo "cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov"
    exit 1
fi

case "$FORMAT" in
    html)
        cargo llvm-cov --all-features --workspace --html
        echo "Coverage report: target/llvm-cov/html/index.html"
        ;;
    lcov)
        cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
        echo "LCOV report: lcov.info"
        ;;
    text)
        cargo llvm-cov --all-features --workspace
        ;;
    *)
        echo "Usage: $0 [html|lcov|text]"
        exit 1
        ;;
esac
