#!/usr/bin/env python3
"""Format benchmark results for display.

Usage:
    ./format-results.py results.json              # Pretty print
    ./format-results.py results.json --markdown   # Markdown table
    ./format-results.py results.json --receipt    # Receipt format

Supports both the detailed format (with metadata/benchmarks structure)
and the simplified format (with results structure).
"""

import json
import sys
import argparse
from datetime import datetime
from pathlib import Path
from typing import Dict, Any, Optional


def format_duration(ns: int) -> str:
    """Format nanoseconds as human-readable duration."""
    if ns < 1000:
        return f"{ns}ns"
    elif ns < 1_000_000:
        return f"{ns / 1000:.1f}us"
    elif ns < 1_000_000_000:
        return f"{ns / 1_000_000:.1f}ms"
    else:
        return f"{ns / 1_000_000_000:.2f}s"


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


def extract_benchmarks(data: dict) -> Dict[str, Dict[str, Any]]:
    """Extract benchmark data from either format."""
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

    # Format 2: Detailed format with "benchmarks"
    elif "benchmarks" in data:
        benchmarks = data["benchmarks"]
        for key, value in benchmarks.items():
            if isinstance(value, dict):
                # Check if this is a category
                if any(isinstance(v, dict) and ("mean" in v or "mean_ns" in v)
                       for v in value.values()):
                    results[key] = {}
                    for name, bench_data in value.items():
                        if isinstance(bench_data, dict) and ("mean" in bench_data or "mean_ns" in bench_data):
                            results[key][name] = bench_data
                # Or a flat benchmark
                elif "mean" in value or "mean_ns" in value:
                    if "other" not in results:
                        results["other"] = {}
                    results["other"][key] = value

    return results


def load_results(path: str) -> dict:
    """Load JSON results file."""
    with open(path) as f:
        return json.load(f)


def print_pretty(data: dict) -> None:
    """Pretty print benchmark results."""
    # Extract metadata
    timestamp = data.get("timestamp") or data.get("metadata", {}).get("date", "unknown")
    git_sha = data.get("git_sha") or data.get("metadata", {}).get("git_sha", "unknown")
    rust_version = (data.get("environment", {}).get("rust_version") or
                    data.get("metadata", {}).get("rust", {}).get("version", "unknown"))
    os_name = (data.get("environment", {}).get("os") or
               data.get("metadata", {}).get("machine", {}).get("os", "unknown"))

    print("=" * 60)
    print(f"Benchmark Results - {timestamp}")
    print("=" * 60)
    print()
    print(f"Git SHA:      {git_sha}")
    print(f"Rust Version: {rust_version}")
    print(f"OS:           {os_name}")
    print()

    results = extract_benchmarks(data)
    for category in sorted(results.keys()):
        benchmarks = results[category]
        print(f"\n{category.upper()}:")
        print("-" * 40)

        for name in sorted(benchmarks.keys()):
            values = benchmarks[name]
            ns = extract_mean_ns(values)
            if ns is not None:
                display = format_duration(ns)
                print(f"  {name:30s}  {display}")


def print_markdown(data: dict) -> None:
    """Print benchmark results as markdown table."""
    # Extract metadata
    timestamp = data.get("timestamp") or data.get("metadata", {}).get("date", "unknown")
    git_sha = data.get("git_sha") or data.get("metadata", {}).get("git_sha", "unknown")
    rust_version = (data.get("environment", {}).get("rust_version") or
                    data.get("metadata", {}).get("rust", {}).get("version", "unknown"))

    print("## Benchmark Results")
    print()
    print(f"- **Git SHA:** `{git_sha}`")
    print(f"- **Timestamp:** {timestamp}")
    print(f"- **Rust Version:** {rust_version}")
    print()

    results = extract_benchmarks(data)
    for category in sorted(results.keys()):
        benchmarks = results[category]
        print(f"### {category.title()}")
        print()
        print("| Benchmark | Time | Status |")
        print("|-----------|------|--------|")

        for name in sorted(benchmarks.keys()):
            values = benchmarks[name]
            ns = extract_mean_ns(values)
            if ns is not None:
                display = format_duration(ns)
                target = values.get("target_range", "-")
                meets = values.get("meets_target")
                status = "OK" if meets else ("FAIL" if meets is False else "-")
                print(f"| {name} | {display} | {status} |")

        print()


def print_receipt(data: dict) -> None:
    """Print benchmark results in receipt format."""
    # Extract metadata
    timestamp_raw = data.get("timestamp") or data.get("metadata", {}).get("date")
    if timestamp_raw:
        try:
            dt = datetime.fromisoformat(timestamp_raw.replace("Z", "+00:00"))
            timestamp = dt.strftime("%Y-%m-%d %H:%M:%S")
        except (ValueError, AttributeError):
            timestamp = timestamp_raw
    else:
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    git_sha = data.get("git_sha") or data.get("metadata", {}).get("git_sha", "unknown")
    version = data.get("version") or data.get("metadata", {}).get("version", "unknown")

    print("=" * 50)
    print(f"BENCHMARK RECEIPT - {timestamp}")
    print("=" * 50)
    print()
    print(f"Run ID:   bench-{datetime.now().strftime('%Y%m%d-%H%M%S')}")
    print(f"Git SHA:  {git_sha}")
    print(f"Version:  {version}")
    print()

    results = extract_benchmarks(data)
    total_benchmarks = 0
    passed = 0
    failed = 0

    for category in sorted(results.keys()):
        benchmarks = results[category]
        print(f"{category.upper()} BENCHMARKS:")
        for name in sorted(benchmarks.keys()):
            values = benchmarks[name]
            ns = extract_mean_ns(values)
            if ns is not None:
                display = format_duration(ns)
                meets = values.get("meets_target")
                status_char = ""
                if meets is True:
                    status_char = " [OK]"
                    passed += 1
                elif meets is False:
                    status_char = " [FAIL]"
                    failed += 1
                print(f"  {name:35s} {display:>10s}{status_char}")
                total_benchmarks += 1
        print()

    print("SUMMARY:")
    print(f"  Total benchmarks:  {total_benchmarks}")
    if passed > 0 or failed > 0:
        print(f"  Passed targets:    {passed}")
        print(f"  Failed targets:    {failed}")
    print()
    print("STATUS: COMPLETE")
    print("=" * 50)


def main():
    parser = argparse.ArgumentParser(description="Format benchmark results")
    parser.add_argument("file", help="JSON results file")
    parser.add_argument("--markdown", "-m", action="store_true",
                        help="Output as markdown")
    parser.add_argument("--receipt", "-r", action="store_true",
                        help="Output as receipt format")
    args = parser.parse_args()

    if not Path(args.file).exists():
        print(f"Error: File not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    data = load_results(args.file)

    if args.markdown:
        print_markdown(data)
    elif args.receipt:
        print_receipt(data)
    else:
        print_pretty(data)


if __name__ == "__main__":
    main()
