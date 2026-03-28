#!/usr/bin/env python3
"""Extract failed test names from cargo test JSON output.

Parses cargo test --format json output (with -Z unstable-options)
and prints names of failed tests, one per line.

Usage:
    extract-failed-tests.py <json-output-file>
    cargo test -Z unstable-options --format json | extract-failed-tests.py
"""

import sys
import json


def extract_failed_tests(lines):
    """Parse JSON lines and return list of failed test names."""
    failed = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
            if event.get("type") == "test" and event.get("event") == "failed":
                name = event.get("name")
                if name:
                    failed.append(name)
        except json.JSONDecodeError:
            continue
    return failed


def main():
    if len(sys.argv) > 1:
        with open(sys.argv[1], "r") as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    for test in extract_failed_tests(lines):
        print(test)


if __name__ == "__main__":
    main()
