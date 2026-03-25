#!/usr/bin/env python3

"""Compatibility shim for `cargo xtask doc-claims`."""

import subprocess
from pathlib import Path


if __name__ == "__main__":
    repo_root = Path(__file__).resolve().parents[1]
    raise SystemExit(subprocess.call(["cargo", "xtask", "doc-claims"], cwd=repo_root))
