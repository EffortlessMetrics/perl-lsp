#!/usr/bin/env python3
"""Compare benchmark results against baseline.

Usage:
    ./compare.py                              # Compare latest vs committed baseline
    ./compare.py baseline.json current.json   # Compare two specific files
    ./compare.py --fail-on-regression         # Exit non-zero if regression found

Supports both the detailed format (with metadata/benchmarks structure)
and the simplified format (with results structure).
"""

import json
import sys
import argparse
from pathlib import Path
from typing import Optional, Tuple, Dict, Any

# ANSI colors
RED = '\033[0;31m'
GREEN = '\033[0;32m'
YELLOW = '\033[1;33m'
BLUE = '\033[0;34m'
NC = '\033[0m'  # No Color

# Configuration
REGRESSION_THRESHOLD = 20  # Percentage slower that triggers regression
IMPROVEMENT_THRESHOLD = 10  # Percentage faster that counts as improvement


def load_json(path: Path) -> dict:
    """Load JSON file."""
    with open(path) as f:
        return json.load(f)


def extract_mean_ns(bench_data: dict) -> Optional[int]:
    """Extract mean nanoseconds from benchmark data, handling both formats."""
    # Format 1: Direct mean_ns key (simplified format)
    if "mean_ns" in bench_data:
        return int(bench_data["mean_ns"])

    # Format 2: Nested mean.nanoseconds (detailed format)
    if "mean" in bench_data:
        mean = bench_data["mean"]
        if isinstance(mean, dict):
            if "nanoseconds" in mean:
                return int(mean["nanoseconds"])
            if "point_estimate" in mean:
                return int(mean["point_estimate"])

    return None


def get_display_value(ns: int) -> str:
    """Format nanoseconds as human-readable string."""
    if ns < 1000:
        return f"{ns}ns"
    elif ns < 1_000_000:
        return f"{ns / 1000:.1f}us"
    elif ns < 1_000_000_000:
        return f"{ns / 1_000_000:.1f}ms"
    else:
        return f"{ns / 1_000_000_000:.2f}s"


def extract_benchmarks(data: dict) -> Dict[str, Dict[str, Any]]:
    """Extract benchmark data from either format, returning category->name->data structure."""
    results = {}

    # Format 1: Simplified format with top-level "results"
    if "results" in data:
        for category, benchmarks in data["results"].items():
            if isinstance(benchmarks, dict):
                results[category] = {}
                for name, values in benchmarks.items():
                    if name.startswith("_"):
                        continue
                    if isinstance(values, dict):
                        results[category][name] = values

    # Format 2: Detailed format with "benchmarks" containing category keys
    elif "benchmarks" in data:
        benchmarks = data["benchmarks"]
        # Check if it's category-keyed or flat
        for key, value in benchmarks.items():
            if isinstance(value, dict):
                # Check if this is a category (contains nested benchmarks)
                if any(isinstance(v, dict) and ("mean" in v or "mean_ns" in v)
                       for v in value.values()):
                    results[key] = {}
                    for name, bench_data in value.items():
                        if isinstance(bench_data, dict) and ("mean" in bench_data or "mean_ns" in bench_data):
                            results[key][name] = bench_data
                # Otherwise it's a flat benchmark
                elif "mean" in value or "mean_ns" in value:
                    if "other" not in results:
                        results["other"] = {}
                    results["other"][key] = value

    return results


def compare_benchmarks(baseline_file: Path, current_file: Path,
                       fail_on_regression: bool = False) -> int:
    """Compare two benchmark files and print report."""
    baseline_data = load_json(baseline_file)
    current_data = load_json(current_file)

    # Extract metadata
    baseline_version = (baseline_data.get("metadata", {}).get("version") or
                        baseline_data.get("version", "unknown"))
    baseline_sha = (baseline_data.get("metadata", {}).get("git_sha") or
                    baseline_data.get("git_sha", "unknown"))
    current_sha = (current_data.get("metadata", {}).get("git_sha") or
                   current_data.get("git_sha", "unknown"))
    current_timestamp = (current_data.get("metadata", {}).get("date") or
                         current_data.get("timestamp", "unknown"))

    # Header
    print(f"{BLUE}========================================")
    print("Benchmark Comparison")
    print(f"========================================{NC}")
    print()
    print(f"Baseline: v{baseline_version} ({baseline_sha})")
    print(f"Current:  {current_sha} @ {current_timestamp}")
    print(f"Regression threshold: {REGRESSION_THRESHOLD}%")
    print()

    # Extract benchmarks
    baseline_benchmarks = extract_benchmarks(baseline_data)
    current_benchmarks = extract_benchmarks(current_data)

    # Counters
    regressions = 0
    improvements = 0
    unchanged = 0
    missing = 0

    # Compare each category
    for category in sorted(set(baseline_benchmarks.keys()) | set(current_benchmarks.keys())):
        if category not in baseline_benchmarks:
            continue

        print(f"{BLUE}{category}:{NC}")

        base_cat = baseline_benchmarks.get(category, {})
        curr_cat = current_benchmarks.get(category, {})

        for bench_name in sorted(base_cat.keys()):
            base_bench = base_cat[bench_name]
            curr_bench = curr_cat.get(bench_name)

            base_ns = extract_mean_ns(base_bench)
            if base_ns is None:
                continue

            if curr_bench is None:
                missing += 1
                print(f"  {bench_name:30s}  {YELLOW}MISSING{NC}")
                continue

            curr_ns = extract_mean_ns(curr_bench)
            if curr_ns is None:
                missing += 1
                print(f"  {bench_name:30s}  {YELLOW}NO DATA{NC}")
                continue

            # Calculate percentage change
            pct_change = ((curr_ns - base_ns) * 100) / base_ns

            # Format times
            base_display = get_display_value(base_ns)
            curr_display = get_display_value(curr_ns)

            # Determine status
            if pct_change > REGRESSION_THRESHOLD:
                status = "REGRESSION"
                color = RED
                regressions += 1
            elif pct_change < -IMPROVEMENT_THRESHOLD:
                status = "FASTER"
                color = GREEN
                improvements += 1
            else:
                status = "OK"
                color = NC
                unchanged += 1

            # Format sign
            sign = "+" if pct_change >= 0 else ""

            print(f"  {bench_name:25s}  {base_display:>10s} -> {curr_display:<10s}  "
                  f"({sign}{pct_change:.1f}%)  [{color}{status}{NC}]")

        print()

    # Summary
    print(f"{BLUE}========================================")
    print("Summary")
    print(f"========================================{NC}")
    print()
    print(f"  Regressions:  {RED}{regressions}{NC}")
    print(f"  Improvements: {GREEN}{improvements}{NC}")
    print(f"  Unchanged:    {unchanged}")
    if missing > 0:
        print(f"  Missing:      {YELLOW}{missing}{NC}")
    print()

    # Final status
    if regressions > 0:
        print(f"{RED}STATUS: REGRESSION DETECTED{NC}")
        return 1 if fail_on_regression else 0
    else:
        print(f"{GREEN}STATUS: PASS{NC}")
        return 0


def find_latest_baseline(base_path: Path) -> Optional[Path]:
    """Find the most recent baseline file."""
    baselines_dir = base_path / "benchmarks" / "baselines"
    if not baselines_dir.exists():
        return None

    baseline_files = sorted(baselines_dir.glob("*.json"), reverse=True)
    return baseline_files[0] if baseline_files else None


def main():
    global REGRESSION_THRESHOLD

    parser = argparse.ArgumentParser(description="Compare benchmark results")
    parser.add_argument("baseline", nargs="?", help="Baseline JSON file")
    parser.add_argument("current", nargs="?", help="Current results JSON file")
    parser.add_argument("--fail-on-regression", "-f", action="store_true",
                        help="Exit non-zero if regression detected")
    parser.add_argument("--threshold", "-t", type=int, default=REGRESSION_THRESHOLD,
                        help=f"Regression threshold percentage (default: {REGRESSION_THRESHOLD})")
    args = parser.parse_args()

    REGRESSION_THRESHOLD = args.threshold

    # Determine paths
    repo_root = Path(__file__).parent.parent.parent

    baseline_file = Path(args.baseline) if args.baseline else find_latest_baseline(repo_root)
    current_file = Path(args.current) if args.current else repo_root / "benchmarks" / "results" / "latest.json"

    if baseline_file is None:
        print(f"{RED}Error: No baseline file found{NC}", file=sys.stderr)
        print("Run 'just bench-baseline' to create one.", file=sys.stderr)
        sys.exit(1)

    if not baseline_file.exists():
        print(f"{RED}Error: Baseline file not found: {baseline_file}{NC}", file=sys.stderr)
        sys.exit(1)

    if not current_file.exists():
        print(f"{RED}Error: Current results file not found: {current_file}{NC}", file=sys.stderr)
        print("Run 'just bench' first to generate results.", file=sys.stderr)
        sys.exit(1)

    sys.exit(compare_benchmarks(baseline_file, current_file, args.fail_on_regression))


if __name__ == "__main__":
    main()
