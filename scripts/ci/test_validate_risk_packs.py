#!/usr/bin/env python3
"""Focused tests for scripts/ci/validate_risk_packs.py."""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from validate_risk_packs import main  # noqa: E402


class ValidateRiskPacksTests(unittest.TestCase):
    def run_validator(
        self,
        risk_packs_text: str,
        lanes_text: str,
        *,
        strict: bool,
    ) -> tuple[int, str]:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            risk_packs = root / "ci-risk-packs.toml"
            lanes = root / "ci-lanes.toml"
            risk_packs.write_text(risk_packs_text, encoding="utf-8")
            lanes.write_text(lanes_text, encoding="utf-8")

            old_argv = sys.argv
            try:
                sys.argv = [
                    "validate_risk_packs.py",
                    "--risk-packs",
                    str(risk_packs),
                    "--lanes",
                    str(lanes),
                ]
                if strict:
                    sys.argv.append("--strict")
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    status = main()
            finally:
                sys.argv = old_argv

        return status, stdout.getvalue()

    def test_strict_passes_when_risk_pack_references_known_lanes(self) -> None:
        status, output = self.run_validator(
            """
            [risk_pack.parser]
            lanes = ["rust_small"]
            deep_lanes = ["full_matrix"]
            paths = ["crates/perl-parser/**"]
            labels = ["full-ci"]
            """,
            """
            [lane.rust_small]
            [lane.full_matrix]
            """,
            strict=True,
        )

        self.assertEqual(0, status)
        self.assertIn("Risk packs in", output)
        self.assertIn("Lanes in", output)
        self.assertIn("All risk packs valid.", output)

    def test_strict_fails_on_unknown_lanes_missing_filters_and_bad_labels(self) -> None:
        status, output = self.run_validator(
            """
            [risk_pack.parser]
            lanes = ["missing_lane"]
            deep_lanes = ["missing_deep_lane"]
            labels = [1]
            """,
            """
            [lane.rust_small]
            """,
            strict=True,
        )

        self.assertEqual(1, status)
        self.assertIn("parser.lanes references unknown lane 'missing_lane'", output)
        self.assertIn(
            "parser.deep_lanes references unknown lane 'missing_deep_lane'",
            output,
        )
        self.assertIn("parser has neither `paths` nor `keywords`", output)
        self.assertIn("parser has non-string label: 1", output)

    def test_non_strict_reports_issues_without_failing(self) -> None:
        status, output = self.run_validator(
            """
            [risk_pack.docs]
            labels = [false]
            """,
            """
            [lane.docs_gate]
            """,
            strict=False,
        )

        self.assertEqual(0, status)
        self.assertIn("Issues (2):", output)
        self.assertIn("docs has neither `paths` nor `keywords`", output)
        self.assertIn("docs has non-string label: False", output)


if __name__ == "__main__":
    unittest.main()
