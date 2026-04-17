#!/usr/bin/env python3
"""
Update flaky test tracker in .ci/debt-ledger.yaml.

This script runs as a post-hook after `just test-full` in nightly CI only.
It detects flaky failures by matching test output against failure_pattern
from debt-ledger entries, increments failure_count, and updates last_failed_at.

This script is informational only and does not fail CI.
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import yaml


def load_debt_ledger(path: Path) -> dict:
    """Load the debt-ledger.yaml file."""
    with open(path, "r") as f:
        return yaml.safe_load(f)


def save_debt_ledger(path: Path, data: dict) -> None:
    """Save the debt-ledger.yaml file, preserving formatting."""
    with open(path, "w") as f:
        # Use yaml.dump with default flow style for simplicity
        # The original file has custom formatting, but we preserve structure
        yaml.dump(data, f, default_flow_style=False, sort_keys=False, allow_unicode=True)


def update_flaky_tracker(
    debt_ledger: dict,
    test_results: dict,
    dry_run: bool = False,
) -> tuple[int, int]:
    """
    Update flaky test tracker based on test results.

    Args:
        debt_ledger: The parsed debt-ledger.yaml data
        test_results: JSON test results from test run
        dry_run: If True, don't write changes

    Returns:
        Tuple of (updated_count, total_flaky_tests)
    """
    flaky_tests = debt_ledger.get("flaky_tests", [])
    if not flaky_tests:
        return 0, 0

    # Build a map of failure_pattern -> flaky_test entries
    pattern_to_tests = {}
    for entry in flaky_tests:
        pattern = entry.get("failure_pattern", "")
        if pattern:
            if pattern not in pattern_to_tests:
                pattern_to_tests[pattern] = []
            pattern_to_tests[pattern].append(entry)

    # Collect failed tests from test results
    # The test results format from `just test-full` is expected to be
    # a JSON object with a "failures" key containing a list of failed test names
    failed_tests = test_results.get("failures", [])
    if isinstance(failed_tests, list) and failed_tests and isinstance(failed_tests[0], dict):
        # Alternative format: list of dicts with "name" key
        failed_tests = [f.get("name", "") for f in failed_tests]

    updated_count = 0
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # Check each failed test against failure_patterns
    for failed_test in failed_tests:
        for pattern, entries in pattern_to_tests.items():
            if pattern in str(failed_test):
                for entry in entries:
                    entry["failure_count"] = entry.get("failure_count", 0) + 1
                    entry["last_failed_at"] = now
                    updated_count += 1

    if not dry_run and updated_count > 0:
        # Ensure flaky_tests is updated in the ledger
        debt_ledger["flaky_tests"] = flaky_tests

    return updated_count, len(flaky_tests)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Update flaky test tracker in debt-ledger.yaml",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--input",
        type=str,
        help="Path to JSON test results file (from `just test-full`)",
    )
    parser.add_argument(
        "--ledger",
        type=str,
        default=".ci/debt-ledger.yaml",
        help="Path to debt-ledger.yaml (default: .ci/debt-ledger.yaml)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be updated without making changes",
    )

    args = parser.parse_args()

    # If no input file provided, just show help
    if args.input is None:
        parser.print_help()
        return 0

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        return 1

    ledger_path = Path(args.ledger)
    if not ledger_path.exists():
        print(f"Error: Ledger file not found: {ledger_path}", file=sys.stderr)
        return 1

    # Load test results
    try:
        with open(input_path, "r") as f:
            test_results = json.load(f)
    except json.JSONDecodeError as e:
        print(f"Error: Failed to parse JSON from {input_path}: {e}", file=sys.stderr)
        return 1

    # Load debt ledger
    debt_ledger = load_debt_ledger(ledger_path)

    # Update flaky tracker
    updated, total = update_flaky_tracker(debt_ledger, test_results, dry_run=args.dry_run)

    if args.dry_run:
        if updated > 0:
            print(f"Would update {updated} flaky test(s) out of {total} tracked")
        else:
            print("No flaky tests would be updated")
    else:
        if updated > 0:
            save_debt_ledger(ledger_path, debt_ledger)
            print(f"Updated {updated} flaky test(s) out of {total} tracked")
        else:
            print("No flaky tests needed updating")

    return 0


if __name__ == "__main__":
    sys.exit(main())
