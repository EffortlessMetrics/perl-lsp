#!/usr/bin/env python3
"""Extract benchmark results from Criterion's JSON files.

Usage:
    ./extract-criterion.py                    # Extract to latest.json
    ./extract-criterion.py --output out.json  # Specify output file
    ./extract-criterion.py --strict --expect-id "workspace_index/incremental update single file"
                                               # Fail closed unless every --expect-id
                                               # (and at least one benchmark overall)
                                               # was actually extracted (see #3979).

Criterion on-disk layout (both are real and both must parse correctly):
    target/criterion/<name>/new/estimates.json               (direct `c.bench_function(name, ...)`)
    target/criterion/<group>/<name>/new/estimates.json       (`group.bench_function(name, ...)`)
A direct `c.bench_function(name, ...)` whose `name` contains a literal "/"
does NOT nest -- Criterion sanitizes the "/" to "_" and still writes a single
flat directory (verified against a real run: `c.bench_function("cpan/moose_oo_class",
...)` produced `target/criterion/cpan_moose_oo_class/new/...`, not a nested
`cpan/moose_oo_class/`). Only an explicit `group.bench_function(name, ...)`
pair produces a true nested `<group>/<name>` directory.
Only the "new" (current-run) estimate is read; "base"/"change" directories hold
Criterion's own prior-run bookkeeping and must never be mistaken for this run's
result (see benchmarks/scripts/test_extract_criterion.py for fixture coverage).
"""

import json
import os
import sys
import argparse
from datetime import datetime, timezone
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


def benchmark_id(group: str, bench_name: str) -> str:
    """Build the canonical, unambiguous identifier used for --expect-id matching."""
    if group == "other":
        return bench_name
    return f"{group}/{bench_name}"


def parse_estimates_path(rel_path_parts: tuple) -> "tuple[str, str] | None":
    """Derive (group, bench_name) from an estimates.json path, relative to
    target/criterion/.

    Only accepts paths whose second-to-last component is "new" (the current
    run's sample). Criterion also writes "base"/"change" directories holding
    the *previous* run's data once a benchmark has run twice; treating those
    as valid would let a stale estimate from an earlier run satisfy a fresh
    "did this run produce results" check.

    A 3-part path (`<name>/new/estimates.json`) is a direct, ungrouped
    `c.bench_function(name, ...)` call -- there is no group, so `name` alone
    is the identifier. This is also what a direct call's `name` looks like on
    disk even when the *source* string contains a literal "/" (e.g.
    `c.bench_function("cpan/moose_oo_class", ...)`) -- Criterion sanitizes
    that "/" to "_" for the directory name rather than nesting, so it never
    produces more than 3 parts on its own.

    A 4+-part path (`<group>/<name>/new/estimates.json`) is an explicit
    `group.bench_function(name, ...)` call -- the only case that genuinely
    nests. The previous version of this parser assumed every path was (at
    least) 3 parts and always took parts[0]/parts[1] as group/name, which
    silently mis-parsed every direct benchmark as group=<name>, name="new"
    (#3979).
    """
    if len(rel_path_parts) < 3 or rel_path_parts[-2] != "new":
        return None

    if len(rel_path_parts) == 3:
        return "other", rel_path_parts[0]

    group = rel_path_parts[0]
    bench_name = "/".join(rel_path_parts[1:-2])
    return group, bench_name


def find_criterion_results(base_path: Path) -> "tuple[dict, set]":
    """Find and parse Criterion benchmark results.

    Returns a 2-tuple `(results, ids)`: `results` is a dict of
    category -> {bench_name: {...}}, and `ids` is the set of canonical
    benchmark_id() strings found, for --expect-id matching.
    """
    results = {}
    ids = set()

    criterion_path = base_path / "target" / "criterion"
    if not criterion_path.exists():
        return results, ids

    # Walk through criterion output looking for estimates.json files
    for root, _dirs, files in os.walk(criterion_path):
        if "estimates.json" not in files:
            continue

        estimates_path = Path(root) / "estimates.json"
        rel_path = estimates_path.relative_to(criterion_path)
        parsed = parse_estimates_path(rel_path.parts)
        if parsed is None:
            continue
        group, bench_name = parsed

        try:
            with open(estimates_path) as f:
                estimates = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            print(f"Warning: Could not parse {estimates_path}: {e}", file=sys.stderr)
            continue

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

        results.setdefault(category, {})[bench_name] = {
            "mean_ns": mean_ns,
            "low_ns": low_ns,
            "high_ns": high_ns,
            "unit": unit,
            "display": display,
        }
        ids.add(benchmark_id(group, bench_name))

    return results, ids


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
    parser.add_argument("--strict", action="store_true",
                        help="Exit non-zero when zero benchmarks were extracted, or "
                             "when any --expect-id was not found. A benchmark job "
                             "that ran zero benchmarks is a vacuous pass, not a real "
                             "result (see #3979) — use this flag in CI so that "
                             "condition fails the job instead of silently reporting "
                             "'Total benchmarks: 0'.")
    parser.add_argument("--expect-id", action="append", default=[],
                        metavar="ID",
                        help="A benchmark_id() (e.g. 'workspace_index/incremental "
                             "update single file', or a bare name when ungrouped) "
                             "that must be present for --strict to pass. May be "
                             "repeated. Proves specific declared Criterion targets "
                             "actually executed, not just that *something* did.")
    args = parser.parse_args()

    base_path = Path(args.base_path)
    output_path = Path(args.output)

    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Get environment info
    git_sha, git_dirty = get_git_info()
    rust_version = get_rust_version()

    # Extract results
    results, ids = find_criterion_results(base_path)

    if not results:
        print("Warning: No Criterion results found in target/criterion/", file=sys.stderr)
        print("Run 'cargo bench' first to generate results.", file=sys.stderr)

    # Add category markers
    for category in results:
        results[category]["_category"] = category

    # Build output structure
    output = {
        "version": "0.9.0",
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
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

    missing_ids = [expected for expected in args.expect_id if expected not in ids]
    if missing_ids:
        print("Missing expected benchmark IDs:", file=sys.stderr)
        for expected in missing_ids:
            print(f"  - {expected}", file=sys.stderr)

    if args.strict and (total == 0 or missing_ids):
        if total == 0:
            print(
                "Error: 0 benchmarks extracted — this is a vacuous pass, not a real "
                "benchmark run. Failing closed (--strict).",
                file=sys.stderr,
            )
        if missing_ids:
            print(
                "Error: one or more --expect-id benchmarks did not run. Failing "
                "closed (--strict).",
                file=sys.stderr,
            )
        sys.exit(1)


if __name__ == "__main__":
    main()
