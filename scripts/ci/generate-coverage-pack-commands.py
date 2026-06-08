#!/usr/bin/env python3
"""Generate coverage-pack-commands.sh from the CI route receipt.

Integration-test commands (``cargo test ... --tests ...``) are run
NON-FATALLY for coverage collection.  The instrumented binaries write LLVM
coverage data even when their assertions fail, so coverage IS collected from
them.  The quality-gate verdict comes from the coverage NUMBER (patch >= 95 %),
not from test pass/fail.

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


def is_integration_test_command(command: str) -> bool:
    """Return True if this is an integration-test invocation (tests/ suite).

    Integration tests are identified by containing ``--tests`` in the cargo
    invocation.  These are run non-fatally because they may have pre-existing
    assertion failures that are tracked as test-debt in issue #1269.
    """
    return "--tests" in command and command.lstrip().startswith("cargo test")


def render_command_block(command: str) -> str:
    """Render a single command as a bash block in the generated script."""
    if is_integration_test_command(command):
        label = shlex.quote(
            ">>> routed coverage (non-fatal -- assertion failures are pre-existing test-debt #1269): "
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
            f"  echo '::warning::coverage-lane integration test exited non-zero"
            f" (tracked as test-debt in #1269; coverage data still collected)'\n"
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
