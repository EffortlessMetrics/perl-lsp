#!/usr/bin/env python3

"""Compatibility shim for `cargo xtask gates --list`."""

import subprocess
from pathlib import Path
import sys


if __name__ == "__main__":
    repo_root = Path(__file__).resolve().parents[1]
    raise SystemExit(
        subprocess.call(["cargo", "xtask", "gates", "--list", *sys.argv[1:]], cwd=repo_root)
    )
