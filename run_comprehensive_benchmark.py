#!/usr/bin/env python3
"""
Comprehensive benchmark runner for C vs Rust Perl parsers
"""

import subprocess
import os
import glob
import json
import statistics
from pathlib import Path
from datetime import datetime

# Parser binaries
C_PARSER = "/home/steven/code/tree-sitter-perl/target/release/bench_parser"
RUST_PARSER = C_PARSER  # Same binary, different features

# Test directories - start with a smaller subset for testing
TEST_DIRS = [
    "/home/steven/code/tree-sitter-perl/benchmark_tests/*.pl",
    # Comment out fuzzed files for initial test
    # "/home/steven/code/tree-sitter-perl/benchmark_tests/fuzzed/*.pl"
]

# Results directory
RESULTS_DIR = Path(f"/home/steven/code/tree-sitter-perl/benchmark_results/comprehensive_{datetime.now().strftime('%Y%m%d_%H%M%S')}")
RESULTS_DIR.mkdir(parents=True, exist_ok=True)

def run_parser(parser_type, file_path):
    """Run parser and return timing info"""
    try:
        # Build the right parser first
        if parser_type == "c":
            build_cmd = ["cargo", "build", "--release", "--features", "c-scanner test-utils"]
        else:
            build_cmd = ["cargo", "build", "--release", "--features", "pure-rust test-utils"]
        
        # Change to crates directory for build
        os.chdir("/home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs")
        subprocess.run(build_cmd, capture_output=True, timeout=60)
        
        # Run the parser
        result = subprocess.run(
            [C_PARSER, file_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if result.returncode == 0 and "status=success" in result.stdout:
            # Parse output: status=success error=false duration_us=123
            for part in result.stdout.strip().split():
                if part.startswith("duration_us="):
                    return int(part.split("=")[1])
        return None
    except subprocess.TimeoutExpired:
        return None
    except Exception as e:
        print(f"Error running {parser_type} parser on {file_path}: {e}")
        return None

def get_file_size(path):
    """Get file size in bytes"""
    try:
        return os.path.getsize(path)
    except:
        return 0

def categorize_size(size):
    """Categorize file size"""
    if size < 100:
        return "tiny"
    elif size < 1000:
        return "small"
    elif size < 10000:
        return "medium"
    elif size < 100000:
        return "large"
    else:
        return "huge"

def main():
    print("🚀 Comprehensive Perl Parser Benchmark")
    print("=" * 60)
    
    # Collect all test files
    test_files = []
    for pattern in TEST_DIRS:
        test_files.extend(glob.glob(pattern))
    
    print(f"Found {len(test_files)} test files")
    
    # Results storage
    results = []
    size_categories = {"tiny": [], "small": [], "medium": [], "large": [], "huge": []}
    
    # Process each file
    for i, file_path in enumerate(test_files):
        filename = os.path.basename(file_path)
        filesize = get_file_size(file_path)
        category = categorize_size(filesize)
        
        print(f"\r[{i+1}/{len(test_files)}] Testing {filename}...", end="", flush=True)
        
        # Run both parsers
        c_time = run_parser("c", file_path)
        rust_time = run_parser("rust", file_path)
        
        # Calculate speedup
        speedup = None
        if c_time and rust_time:
            speedup = c_time / rust_time
        
        # Store results
        result = {
            "file": filename,
            "path": file_path,
            "size": filesize,
            "category": category,
            "c_time": c_time,
            "rust_time": rust_time,
            "speedup": speedup,
            "c_success": c_time is not None,
            "rust_success": rust_time is not None
        }
        results.append(result)
        
        # Add to category stats
        if speedup:
            size_categories[category].append(speedup)
    
    print("\n")
    
    # Generate report
    print("\n📊 Performance Summary")
    print("=" * 60)
    
    # Success rates
    c_success = sum(1 for r in results if r["c_success"])
    rust_success = sum(1 for r in results if r["rust_success"])
    both_success = sum(1 for r in results if r["c_success"] and r["rust_success"])
    
    print(f"✅ Success Rates:")
    print(f"   C Parser:    {c_success}/{len(results)} ({c_success/len(results)*100:.1f}%)")
    print(f"   Rust Parser: {rust_success}/{len(results)} ({rust_success/len(results)*100:.1f}%)")
    print(f"   Both:        {both_success}/{len(results)} ({both_success/len(results)*100:.1f}%)")
    
    # Performance comparison (only for successful parses)
    valid_speedups = [r["speedup"] for r in results if r["speedup"]]
    if valid_speedups:
        print(f"\n⚡ Performance (based on {len(valid_speedups)} comparable files):")
        print(f"   Average speedup: {statistics.mean(valid_speedups):.2f}x")
        print(f"   Median speedup:  {statistics.median(valid_speedups):.2f}x")
        print(f"   Min speedup:     {min(valid_speedups):.2f}x")
        print(f"   Max speedup:     {max(valid_speedups):.2f}x")
        
        if statistics.mean(valid_speedups) > 1:
            print(f"\n   🏆 Rust parser is {statistics.mean(valid_speedups):.2f}x faster on average!")
        else:
            print(f"\n   🏆 C parser is {1/statistics.mean(valid_speedups):.2f}x faster on average!")
    
    # Performance by file size
    print("\n📏 Performance by File Size:")
    for category in ["tiny", "small", "medium", "large", "huge"]:
        if size_categories[category]:
            avg_speedup = statistics.mean(size_categories[category])
            count = len(size_categories[category])
            print(f"   {category.capitalize():8} ({count:3} files): {avg_speedup:.2f}x speedup")
    
    # Find problematic files
    print("\n⚠️  Files with parsing differences:")
    diff_count = 0
    for r in results:
        if r["c_success"] != r["rust_success"]:
            diff_count += 1
            if diff_count <= 10:  # Show first 10
                status = "C✓ Rust✗" if r["c_success"] else "C✗ Rust✓"
                print(f"   {r['file']:40} [{status}]")
    if diff_count > 10:
        print(f"   ... and {diff_count - 10} more files")
    
    # Save detailed results
    csv_path = RESULTS_DIR / "results.csv"
    with open(csv_path, "w") as f:
        f.write("file,size,category,c_time_us,rust_time_us,speedup,c_success,rust_success\n")
        for r in results:
            f.write(f"{r['file']},{r['size']},{r['category']},{r['c_time'] or 'N/A'},{r['rust_time'] or 'N/A'},")
            f.write(f"{r['speedup'] or 'N/A'},{r['c_success']},{r['rust_success']}\n")
    
    # Save summary
    summary_path = RESULTS_DIR / "summary.json"
    summary = {
        "total_files": len(results),
        "c_success_rate": c_success / len(results),
        "rust_success_rate": rust_success / len(results),
        "average_speedup": statistics.mean(valid_speedups) if valid_speedups else None,
        "median_speedup": statistics.median(valid_speedups) if valid_speedups else None,
        "timestamp": datetime.now().isoformat()
    }
    with open(summary_path, "w") as f:
        json.dump(summary, f, indent=2)
    
    print(f"\n💾 Results saved to: {RESULTS_DIR}")
    print(f"   - results.csv: Detailed results for each file")
    print(f"   - summary.json: Overall summary statistics")
    
    # Generate recommendation
    print("\n🎯 Recommendation:")
    if rust_success / len(results) >= 0.95 and statistics.mean(valid_speedups) >= 0.8:
        print("   ✅ The Rust parser is ready to be the default!")
        print("   - High success rate (95%+)")
        print("   - Competitive performance")
    else:
        print("   ⚠️  The Rust parser needs more work before becoming default")
        if rust_success / len(results) < 0.95:
            print(f"   - Success rate is only {rust_success/len(results)*100:.1f}%")
        if valid_speedups and statistics.mean(valid_speedups) < 0.8:
            print(f"   - Performance is {(1-statistics.mean(valid_speedups))*100:.1f}% slower")

if __name__ == "__main__":
    main()