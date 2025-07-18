#!/usr/bin/env python3
"""Optimized benchmark runner - builds once, tests many"""

import subprocess
import os
import glob
import statistics
from pathlib import Path
from datetime import datetime

# Configuration
CRATES_DIR = "/home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs"
PARSER_BIN = "/home/steven/code/tree-sitter-perl/target/release/bench_parser"
# Get all files
all_original = glob.glob("/home/steven/code/tree-sitter-perl/benchmark_tests/*.pl")
all_fuzzed = glob.glob("/home/steven/code/tree-sitter-perl/benchmark_tests/fuzzed/*.pl")

# Sample fuzzed files (every 5th file to get a representative sample)
sampled_fuzzed = all_fuzzed[::5]  # Take every 5th file

TEST_FILES = all_original + sampled_fuzzed
print(f"Testing {len(all_original)} original files and {len(sampled_fuzzed)} sampled fuzzed files (out of {len(all_fuzzed)} total)")

def build_parser(features):
    """Build parser with given features"""
    print(f"Building with features: {features}")
    os.chdir(CRATES_DIR)
    result = subprocess.run(
        ["cargo", "build", "--release", "--features", features],
        capture_output=True,
        text=True
    )
    return result.returncode == 0

def run_parser(file_path):
    """Run parser on a file and return timing"""
    try:
        result = subprocess.run(
            [PARSER_BIN, file_path],
            capture_output=True,
            text=True,
            timeout=2
        )
        
        if result.returncode == 0 and "duration_us=" in result.stdout:
            for part in result.stdout.strip().split():
                if part.startswith("duration_us="):
                    return int(part.split("=")[1])
    except subprocess.TimeoutExpired:
        pass
    return None

def main():
    print("🚀 Optimized Perl Parser Benchmark")
    print("=" * 60)
    
    results = {"c": {}, "rust": {}}
    
    # Test C parser
    print("\n📦 Testing C Parser...")
    if build_parser("c-scanner test-utils"):
        for file_path in TEST_FILES:
            filename = os.path.basename(file_path)
            print(f"  Testing {filename}...", end="", flush=True)
            time_us = run_parser(file_path)
            results["c"][filename] = time_us
            print(f" {time_us}µs" if time_us else " FAILED")
    else:
        print("  ❌ Build failed!")
        return
    
    # Test Rust parser
    print("\n📦 Testing Rust Parser...")
    if build_parser("pure-rust test-utils"):
        for file_path in TEST_FILES:
            filename = os.path.basename(file_path)
            print(f"  Testing {filename}...", end="", flush=True)
            time_us = run_parser(file_path)
            results["rust"][filename] = time_us
            print(f" {time_us}µs" if time_us else " FAILED")
    else:
        print("  ❌ Build failed!")
        return
    
    # Analysis
    print("\n📊 Results Summary")
    print("=" * 60)
    print(f"{'File':<30} {'Size':>8} {'C (µs)':>10} {'Rust (µs)':>10} {'Speedup':>10}")
    print("-" * 70)
    
    speedups = []
    c_times = []
    rust_times = []
    
    for filename in sorted(results["c"].keys()):
        # Find the actual file path
        matching_files = [f for f in TEST_FILES if os.path.basename(f) == filename]
        if matching_files:
            file_path = matching_files[0]
            size = os.path.getsize(file_path)
        else:
            size = 0
        
        c_time = results["c"].get(filename)
        rust_time = results["rust"].get(filename)
        
        if c_time and rust_time:
            speedup = c_time / rust_time
            speedups.append(speedup)
            c_times.append(c_time)
            rust_times.append(rust_time)
            speedup_str = f"{speedup:.2f}x"
        else:
            speedup_str = "N/A"
        
        c_str = str(c_time) if c_time else "FAIL"
        rust_str = str(rust_time) if rust_time else "FAIL"
        
        print(f"{filename:<30} {size:>8} {c_str:>10} {rust_str:>10} {speedup_str:>10}")
    
    # Statistics
    if speedups:
        print("\n⚡ Performance Statistics:")
        print(f"  Average speedup: {statistics.mean(speedups):.2f}x")
        print(f"  Median speedup:  {statistics.median(speedups):.2f}x")
        print(f"  Min speedup:     {min(speedups):.2f}x")
        print(f"  Max speedup:     {max(speedups):.2f}x")
        
        avg_c = statistics.mean(c_times)
        avg_rust = statistics.mean(rust_times)
        print(f"\n  Average C time:    {avg_c:.0f}µs")
        print(f"  Average Rust time: {avg_rust:.0f}µs")
        
        if statistics.mean(speedups) > 1:
            print(f"\n🏆 Rust parser is {statistics.mean(speedups):.2f}x faster on average!")
        else:
            print(f"\n🏆 C parser is {1/statistics.mean(speedups):.2f}x faster on average!")
    
    # Success rates
    c_success = sum(1 for v in results["c"].values() if v is not None)
    rust_success = sum(1 for v in results["rust"].values() if v is not None)
    total = len(results["c"])
    
    print(f"\n✅ Success Rates:")
    print(f"  C Parser:    {c_success}/{total} ({c_success/total*100:.0f}%)")
    print(f"  Rust Parser: {rust_success}/{total} ({rust_success/total*100:.0f}%)")
    
    # Separate fuzzed vs original
    fuzzed_speedups = []
    original_speedups = []
    for filename in results["c"].keys():
        c_time = results["c"].get(filename)
        rust_time = results["rust"].get(filename)
        if c_time and rust_time:
            speedup = c_time / rust_time
            if "fuzz" in filename:
                fuzzed_speedups.append(speedup)
            else:
                original_speedups.append(speedup)
    
    if original_speedups:
        print(f"\n📁 Original Files:")
        print(f"  Count: {len(original_speedups)}")
        print(f"  Average speedup: {statistics.mean(original_speedups):.2f}x")
    
    if fuzzed_speedups:
        print(f"\n🎲 Fuzzed Files:")
        print(f"  Count: {len(fuzzed_speedups)}")
        print(f"  Average speedup: {statistics.mean(fuzzed_speedups):.2f}x")

if __name__ == "__main__":
    main()