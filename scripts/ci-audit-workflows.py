#!/usr/bin/env python3

"""Compatibility shim for `cargo xtask ci-audit-workflows`."""

import subprocess
import sys
from pathlib import Path


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    return subprocess.call(["cargo", "xtask", "ci-audit-workflows", *sys.argv[1:]], cwd=repo_root)


if __name__ == "__main__":
    raise SystemExit(main())
