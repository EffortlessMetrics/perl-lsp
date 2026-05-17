#!/usr/bin/env python3
"""Compatibility shim for the Rust non-Rust policy schema validator.

The schema validation logic lives in `cargo xtask non-rust validate-policy` so
file-policy checks share the same Rust implementation. This wrapper preserves
older automation that still invokes the historical Python script path.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--allowlist",
        type=pathlib.Path,
        default=pathlib.Path("policy/non-rust-allowlist.toml"),
    )
    parser.add_argument(
        "--debt",
        type=pathlib.Path,
        default=pathlib.Path("policy/non-rust-debt.toml"),
    )
    args = parser.parse_args()

    command = [
        "cargo",
        "xtask",
        "non-rust",
        "validate-policy",
        "--allowlist",
        str(args.allowlist),
        "--debt",
        str(args.debt),
    ]
    env = os.environ.copy()
    env.setdefault("CARGO_TARGET_DIR", "/tmp/perl-lsp-target")
    return subprocess.run(command, check=False, env=env).returncode


if __name__ == "__main__":
    sys.exit(main())
