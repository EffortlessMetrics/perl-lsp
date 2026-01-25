#!/usr/bin/env python3
"""Extract benchmark results from Criterion's JSON files.

Usage:
    ./extract-criterion.py                    # Extract to latest.json
    ./extract-criterion.py --output out.json  # Specify output file
"""

import json
import os
import sys
import argparse
from datetime import datetime
from pathlib import Path
import subprocess


def get_git_info():
    """Get current git SHA and dirty status."""
    try:
        sha = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            stderr=subprocess.DEVNULL
        ).decode().strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        sha = "unknown"

    try:
        subprocess.check_call(
            ["git", "diff", "--quiet"],
            stderr=subprocess.DEVNULL
        )
        dirty = False
    except (subprocess.CalledProcessError, FileNotFoundError):
        dirty = True

    return sha, dirty


def get_rust_version():
    """Get Rust compiler version."""
    try:
        output = subprocess.check_output(
            ["rustc", "--version"],
            stderr=subprocess.DEVNULL
        ).decode().strip()
        return output.split()[1]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"


def find_criterion_results(base_path: Path) -> dict:
    """Find and parse Criterion benchmark results."""
    results = {}

    criterion_path = base_path / "target" / "criterion"
    if not criterion_path.exists():
        return results

    # Walk through criterion output looking for estimates.json files
    for root, dirs, files in os.walk(criterion_path):
        if "estimates.json" in files:
            estimates_path = Path(root) / "estimates.json"
            try:
                with open(estimates_path) as f:
                    estimates = json.load(f)

                # Extract benchmark name from path
                # Path structure: target/criterion/<group>/<bench_name>/new/estimates.json
                rel_path = estimates_path.relative_to(criterion_path)
                parts = rel_path.parts

                if len(parts) >= 3:
                    group = parts[0]
                    bench_name = parts[1]

                    # Get mean value
                    mean = estimates.get("mean", {})
                    mean_ns = int(mean.get("point_estimate", 0))

                    # Get confidence interval
                    low_ns = int(mean.get("confidence_interval", {}).get("lower_bound", mean_ns))
                    high_ns = int(mean.get("confidence_interval", {}).get("upper_bound", mean_ns))

                    # Determine display unit
                    if mean_ns < 1000:
                        unit = "ns"
                        display = f"{mean_ns} ns"
                    elif mean_ns < 1_000_000:
                        unit = "us"
                        display = f"{mean_ns / 1000:.1f} us"
                    elif mean_ns < 1_000_000_000:
                        unit = "ms"
                        display = f"{mean_ns / 1_000_000:.1f} ms"
                    else:
                        unit = "s"
                        display = f"{mean_ns / 1_000_000_000:.2f} s"

                    # Categorize by group name
                    category = categorize_benchmark(group, bench_name)

                    if category not in results:
                        results[category] = {}

                    results[category][bench_name] = {
                        "mean_ns": mean_ns,
                        "low_ns": low_ns,
                        "high_ns": high_ns,
                        "unit": unit,
                        "display": display
                    }

            except (json.JSONDecodeError, KeyError) as e:
                print(f"Warning: Could not parse {estimates_path}: {e}", file=sys.stderr)
                continue

    return results


def categorize_benchmark(group: str, bench_name: str) -> str:
    """Categorize a benchmark based on its group and name."""
    group_lower = group.lower()
    bench_lower = bench_name.lower()

    if "parser" in group_lower or "parse" in bench_lower:
        return "parser"
    elif "lexer" in group_lower or "token" in bench_lower:
        return "lexer"
    elif "rope" in group_lower or "lsp" in group_lower or "position" in bench_lower:
        return "lsp"
    elif "index" in group_lower or "workspace" in group_lower or "symbol" in bench_lower:
        return "index"
    else:
        return "other"


def main():
    parser = argparse.ArgumentParser(description="Extract Criterion benchmark results")
    parser.add_argument("--output", "-o", default="benchmarks/results/latest.json",
                        help="Output JSON file")
    parser.add_argument("--base-path", "-b", default=".",
                        help="Repository base path")
    args = parser.parse_args()

    base_path = Path(args.base_path)
    output_path = Path(args.output)

    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Get environment info
    git_sha, git_dirty = get_git_info()
    rust_version = get_rust_version()

    # Extract results
    results = find_criterion_results(base_path)

    if not results:
        print("Warning: No Criterion results found in target/criterion/", file=sys.stderr)
        print("Run 'cargo bench' first to generate results.", file=sys.stderr)

    # Add category markers
    for category in results:
        results[category]["_category"] = category

    # Build output structure
    output = {
        "version": "0.9.0",
        "timestamp": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "git_sha": git_sha,
        "git_dirty": git_dirty,
        "environment": {
            "os": os.uname().sysname if hasattr(os, 'uname') else "unknown",
            "rust_version": rust_version,
            "extracted_from": "criterion"
        },
        "results": results
    }

    # Write output
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)

    print(f"Results extracted to {output_path}")

    # Print summary
    total = sum(len([k for k in v if not k.startswith("_")]) for v in results.values())
    print(f"Total benchmarks: {total}")
    for category, benchmarks in results.items():
        count = len([k for k in benchmarks if not k.startswith("_")])
        print(f"  {category}: {count}")


if __name__ == "__main__":
    main()
