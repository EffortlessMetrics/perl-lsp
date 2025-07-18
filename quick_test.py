#!/usr/bin/env python3
"""Quick test of parser benchmarking"""

import subprocess
import os

def test_parser(features, file_path):
    """Test a single parser configuration"""
    # Build
    os.chdir("/home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs")
    build_result = subprocess.run(
        ["cargo", "build", "--release", "--features", features],
        capture_output=True,
        text=True
    )
    
    if build_result.returncode != 0:
        print(f"Build failed for {features}")
        return None
    
    # Run
    parser_bin = "/home/steven/code/tree-sitter-perl/target/release/bench_parser"
    run_result = subprocess.run(
        [parser_bin, file_path],
        capture_output=True,
        text=True
    )
    
    print(f"\n{features}:")
    print(f"  stdout: {run_result.stdout.strip()}")
    print(f"  stderr: {run_result.stderr.strip()[:100]}")
    
    if "duration_us=" in run_result.stdout:
        for part in run_result.stdout.strip().split():
            if part.startswith("duration_us="):
                return int(part.split("=")[1])
    return None

# Test files
test_file = "/home/steven/code/tree-sitter-perl/benchmark_tests/simple.pl"

print(f"Testing with: {test_file}")
print("=" * 60)

c_time = test_parser("c-scanner test-utils", test_file)
rust_time = test_parser("pure-rust test-utils", test_file)

print("\nResults:")
print(f"  C parser:    {c_time}µs")
print(f"  Rust parser: {rust_time}µs")

if c_time and rust_time:
    speedup = c_time / rust_time
    print(f"  Speedup:     {speedup:.2f}x")
    if speedup > 1:
        print(f"  Rust is {speedup:.2f}x faster!")
    else:
        print(f"  C is {1/speedup:.2f}x faster!")