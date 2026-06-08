#!/usr/bin/env python3
"""Tests for generate-coverage-pack-commands.py.

Verifies that:
- Integration-test commands (--tests) are wrapped non-fatally in the
  generated bash script.
- Non-integration commands are emitted with strict error-exit semantics.
- The generated script still has ``set -euo pipefail`` at the top.
- pack-ids and commands are deduplicated correctly.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


def _load_module():
    """Load the generate-coverage-pack-commands module."""
    spec_path = (
        Path(__file__).parent / "generate-coverage-pack-commands.py"
    )
    spec = importlib.util.spec_from_file_location(
        "generate_coverage_pack_commands", spec_path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"Could not load {spec_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)  # type: ignore[union-attr]
    return module


gen = _load_module()


class IsIntegrationTestCommandTests(unittest.TestCase):
    """Unit tests for the is_integration_test_command predicate."""

    def test_cargo_test_with_tests_flag_is_integration(self) -> None:
        cmd = "cargo test -p perl-dap --tests --profile agent --locked -- --test-threads=1"
        self.assertTrue(gen.is_integration_test_command(cmd))

    def test_cargo_test_lib_is_not_integration(self) -> None:
        cmd = "cargo test --workspace --lib --profile agent --locked"
        self.assertFalse(gen.is_integration_test_command(cmd))

    def test_cargo_check_is_not_integration(self) -> None:
        cmd = "cargo check --workspace --all-targets --profile agent --locked"
        self.assertFalse(gen.is_integration_test_command(cmd))

    def test_cargo_test_specific_bin_without_tests_flag_is_not_integration(self) -> None:
        cmd = "cargo test -p xtask --bin xtask --profile agent --locked ci_route -- --nocapture"
        self.assertFalse(gen.is_integration_test_command(cmd))

    def test_python_unittest_is_not_integration(self) -> None:
        cmd = "python -m unittest scripts/ci/test_route_codecov_packs.py"
        self.assertFalse(gen.is_integration_test_command(cmd))


class RenderCommandBlockTests(unittest.TestCase):
    """Unit tests for render_command_block output."""

    def test_integration_test_command_wrapped_non_fatally(self) -> None:
        """Integration test command must use '|| {' to suppress exit code."""
        cmd = "cargo test -p perl-dap --tests --profile agent --locked -- --test-threads=1"
        block = gen.render_command_block(cmd)
        # Must contain the command itself.
        self.assertIn(cmd, block)
        # Must contain '|| {' or '|| {' to suppress non-zero exit.
        self.assertIn("|| {", block)
        # Must reference the test-debt tracking issue.
        self.assertIn("#1269", block)

    def test_non_integration_command_uses_direct_invocation(self) -> None:
        """Non-integration commands must NOT be wrapped with '|| {'."""
        cmd = "cargo test --workspace --lib --profile agent --locked"
        block = gen.render_command_block(cmd)
        self.assertIn(cmd, block)
        self.assertNotIn("|| {", block)

    def test_generated_block_contains_echo_label(self) -> None:
        cmd = "cargo check --workspace --all-targets --profile agent --locked"
        block = gen.render_command_block(cmd)
        self.assertIn("echo ", block)

    def test_integration_block_contains_github_warning_annotation(self) -> None:
        """GitHub Actions ::warning:: annotation must appear for integration failures."""
        cmd = "cargo test -p perl-parser --tests --profile agent --locked -- --test-threads=1"
        block = gen.render_command_block(cmd)
        self.assertIn("::warning::", block)


class GenerateScriptTests(unittest.TestCase):
    """Integration tests for the full generate-coverage-pack-commands flow."""

    def _make_route_receipt(self, packs: list[dict]) -> dict:
        return {
            "schema_version": "ci_route.v1",
            "coverage_proof_packs": packs,
        }

    def _run_generate(self, receipt: dict) -> str:
        """Run main() with a fake route receipt and return the generated script."""
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            receipt_dir = tmp / "target" / "receipts" / "quality"
            receipt_dir.mkdir(parents=True)
            (receipt_dir / "ci-route.json").write_text(
                json.dumps(receipt), encoding="utf-8"
            )
            # Patch Path to resolve relative paths under tmpdir.
            import os
            old_cwd = os.getcwd()
            try:
                os.chdir(tmpdir)
                result = gen.main()
                script_path = receipt_dir / "coverage-pack-commands.sh"
                self.assertEqual(0, result, "main() should return 0 on success")
                return script_path.read_text(encoding="utf-8")
            finally:
                os.chdir(old_cwd)

    def test_script_has_set_euo_pipefail_header(self) -> None:
        receipt = self._make_route_receipt([
            {
                "id": "patch-coverage-rust-focused",
                "commands": ["cargo test --workspace --lib --profile agent --locked"],
            }
        ])
        script = self._run_generate(receipt)
        self.assertIn("set -euo pipefail", script)

    def test_integration_test_non_fatal_in_generated_script(self) -> None:
        """The key property: integration tests must use '|| {' in generated script."""
        receipt = self._make_route_receipt([
            {
                "id": "patch-coverage-rust-focused",
                "commands": [
                    "cargo test --workspace --lib --profile agent --locked",
                    "cargo test -p perl-dap --tests --profile agent --locked -- --test-threads=1",
                ],
            }
        ])
        script = self._run_generate(receipt)
        # Integration test command must be non-fatal.
        self.assertIn("cargo test -p perl-dap --tests", script)
        self.assertIn("|| {", script)
        # Warning annotation for observability.
        self.assertIn("::warning::", script)

    def test_lib_test_command_is_fatal_in_generated_script(self) -> None:
        """Library tests (not --tests) must remain strictly fatal."""
        receipt = self._make_route_receipt([
            {
                "id": "patch-coverage-rust-focused",
                "commands": [
                    "cargo test --workspace --lib --profile agent --locked",
                ],
            }
        ])
        script = self._run_generate(receipt)
        # No non-fatal wrapper for lib tests.
        self.assertNotIn("|| {", script)

    def test_commands_are_deduplicated_across_packs(self) -> None:
        """The same command appearing in two packs must appear once in the script.

        We count occurrences of the command as a standalone line (not as
        part of the echo label), by checking lines that start with the command
        text rather than with 'echo'.
        """
        shared_cmd = "cargo test --workspace --lib --profile agent --locked"
        receipt = self._make_route_receipt([
            {"id": "pack-a", "commands": [shared_cmd]},
            {"id": "pack-b", "commands": [shared_cmd]},
        ])
        script = self._run_generate(receipt)
        # Count only lines where the command is the actual invocation (not echo label).
        invocation_lines = [
            line for line in script.splitlines()
            if line.strip() == shared_cmd
        ]
        self.assertEqual(1, len(invocation_lines), f"command must not be duplicated; got lines: {invocation_lines}")

    def test_empty_packs_produces_valid_script(self) -> None:
        receipt = self._make_route_receipt([])
        script = self._run_generate(receipt)
        self.assertIn("set -euo pipefail", script)

    def test_missing_receipt_returns_nonzero(self) -> None:
        """main() must return 1 when the route receipt is missing."""
        import os
        with tempfile.TemporaryDirectory() as tmpdir:
            old_cwd = os.getcwd()
            try:
                os.chdir(tmpdir)
                result = gen.main()
                self.assertEqual(1, result)
            finally:
                os.chdir(old_cwd)


if __name__ == "__main__":
    unittest.main()
