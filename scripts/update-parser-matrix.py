#!/usr/bin/env python3

"""Compatibility shim for `cargo xtask parser-matrix`."""

import subprocess
import sys
from pathlib import Path


if __name__ == "__main__":
    repo_root = Path(__file__).resolve().parents[1]
    raise SystemExit(
        subprocess.call(["cargo", "xtask", "parser-matrix", *sys.argv[1:]], cwd=repo_root)
    )
