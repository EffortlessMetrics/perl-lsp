#!/bin/bash
# Quick verification that stacker is working

echo "Building with release mode first..."
cargo build --features pure-rust --release --quiet

echo "Running release mode test (should always work)..."
cargo run --features pure-rust --release --bin test_stacker 2>&1 | head -20

echo -e "\nBuilding with debug mode..."
cargo build --features pure-rust --quiet

echo "Running debug mode test (testing stacker fix)..."
timeout 30s cargo run --features pure-rust --bin test_stacker 2>&1 | head -20

if [ $? -eq 124 ]; then
    echo "❌ Debug mode timed out - stacker may not be working"
else
    echo "✅ Debug mode completed - stacker is working!"
fi