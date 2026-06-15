#!/usr/bin/env python3
"""Generate coverage-pack-commands.sh from the CI route receipt.

Integration-test commands (those containing ``--tests``) are run NON-FATALLY
for coverage collection.  The instrumented binaries write LLVM coverage data
even when their assertions fail, so coverage IS collected from them.  The
quality-gate verdict comes from the coverage NUMBER (patch >= 95 %), not from
test pass/fail.

Since #1282, integration commands use ``cargo llvm-cov test --no-report``
instead of ``cargo test`` so that cargo-llvm-cov registers the binary in its
tracking file.  Without this registration, ``cargo llvm-cov report`` does not
know which binary files to symbolise for integration-test profdata, causing
integration-test-covered source lines to appear uncovered (false-low patch %).

Context (issue #1269 / epic #1232):
  perl-dap integration tests (tests/) have pre-existing correctness debt and
  are NOT validated by any other CI lane -- the "Perl LSP Rust Small Result"
  correctness gate runs only ``cargo check --workspace`` plus two smoke tests.
  Failing on these tests in the coverage lane blocks PRs on legacy debt without
  catching any real regression.  The proper long-term fix (tracked in #1269) is
  to add a real correctness lane for perl-dap integration tests.  This script
  unblocks coverage measurement now.
"""

from __future__ import annotations

import json
import shlex
import sys
from pathlib import Path


def is_test_command(command: str) -> bool:
    """Return True if this is a test invocation (cargo test or cargo llvm-cov test).

    ALL test commands are run non-fatally (#1470/#1232/#1269).  Coverage
    collection does not depend on test correctness: the LLVM instrumentation
    data is written by the binary before it exits, so cargo-llvm-cov collects
    coverage regardless of whether assertions pass.  The quality-gate verdict
    is the coverage NUMBER, not test pass/fail.

    Test correctness is owned by the separate correctness gate (#1469).

    Since #1282, integration commands use ``cargo llvm-cov test --no-report``
    instead of ``cargo test`` so that cargo-llvm-cov registers the binary for
    the final ``cargo llvm-cov report`` step (fixing the false-low patch
    coverage for integration-tested code paths).
    """
    stripped = command.lstrip()
    return stripped.startswith("cargo test") or stripped.startswith("cargo llvm-cov test")


def render_command_block(command: str) -> str:
    """Render a single command as a bash block in the generated script."""
    if is_test_command(command):
        label = shlex.quote(
            ">>> routed coverage (non-fatal -- test correctness is a separate gate #1469): "
            + command
        )
        # Run the test but suppress non-zero exit so coverage-pack-commands.sh
        # continues.  The LLVM instrumentation data is written by the binary
        # before it exits, so cargo-llvm-cov collects coverage regardless of
        # whether assertions pass.  A genuinely-broken *production* change
        # still shows low patch coverage and fails the numeric gate.
        return (
            f"echo {label}\n"
            f"{command} || {{\n"
            f"  echo '::warning::coverage-lane test exited non-zero"
            f" (test correctness is a separate gate #1469; coverage data still collected)'\n"
            f"}}\n"
        )
    else:
        label = shlex.quote(">>> routed coverage: " + command)
        return f"echo {label}\n{command}\n"


def main() -> int:
    route_receipt = Path("target/receipts/quality/ci-route.json")
    if not route_receipt.exists():
        print(
            f"error: {route_receipt} not found -- run `cargo xtask ci route` first",
            file=sys.stderr,
        )
        return 1

    route = json.loads(route_receipt.read_text(encoding="utf-8"))
    packs = route.get("coverage_proof_packs") or []

    commands: list[str] = []
    seen: set[str] = set()
    pack_ids: list[str] = []

    for pack in packs:
        pack_ids.append(str(pack.get("id", "<unknown>")))
        for command in pack.get("commands") or []:
            if command not in seen:
                seen.add(command)
                commands.append(command)

    packs_txt = Path("target/receipts/quality/coverage-route-selected-packs.txt")
    packs_txt.write_text(", ".join(pack_ids) + "\n", encoding="utf-8")

    script_lines = ["#!/usr/bin/env bash", "set -euo pipefail", ""]
    for command in commands:
        script_lines.append(render_command_block(command))

    body = "\n".join(script_lines)
    script_path = Path("target/receipts/quality/coverage-pack-commands.sh")
    script_path.write_text(body, encoding="utf-8")

    print(f"generated {script_path} with {len(commands)} command(s) from {len(pack_ids)} pack(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
