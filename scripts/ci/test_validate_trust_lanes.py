#!/usr/bin/env python3
"""Focused tests for scripts/ci/validate_trust_lanes.py."""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from validate_trust_lanes import EXPECTED_CLASSES, main  # noqa: E402


class ValidateTrustLanesTests(unittest.TestCase):
    def run_validator(self, policy_text: str, *, strict: bool) -> tuple[int, str]:
        with tempfile.TemporaryDirectory() as tmp:
            policy = Path(tmp) / "trust-lanes.toml"
            policy.write_text(policy_text, encoding="utf-8")

            old_argv = sys.argv
            try:
                sys.argv = [
                    "validate_trust_lanes.py",
                    "--trust-lanes",
                    str(policy),
                ]
                if strict:
                    sys.argv.append("--strict")
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    status = main()
            finally:
                sys.argv = old_argv

        return status, stdout.getvalue()

    def valid_policy_text(self) -> str:
        header = """
schema_version = 1
policy = "trust-lanes"
owner = "EffortlessMetrics"
status = "advisory"
updated = "2026-06-01"
spec = "policy/trust-lanes.toml"
classification_rule = "Use the strongest claim made by the PR."
enforcement_boundary = "Advisory metadata only."
"""
        classes = []
        for rank, class_id in enumerate(sorted(EXPECTED_CLASSES), start=1):
            classes.append(
                f"""
[class.{class_id}]
risk_rank = {rank}
claim_boundary = "Boundary for {class_id}."
required_checks = ["focused proof"]
optional_checks = []
skipped_by_policy_checks = []
widening_triggers = ["claim widens"]
receipt_paths = ["policy/trust-lanes.toml"]
support_claim_impact = "No support claim impact."
"""
            )
        return header + "\n".join(classes)

    def test_strict_passes_when_policy_matches_contract(self) -> None:
        status, output = self.run_validator(self.valid_policy_text(), strict=True)

        self.assertEqual(0, status)
        self.assertIn("Trust-lane classes in", output)
        self.assertIn("All trust-lane classes valid.", output)

    def test_strict_fails_on_missing_class_invalid_fields_and_bad_receipt(self) -> None:
        policy_text = self.valid_policy_text()
        policy_text = policy_text.replace("[class.docs_status_only]", "[class.unknown_lane]", 1)
        policy_text = policy_text.replace("schema_version = 1", "schema_version = 2", 1)
        policy_text = policy_text.replace('owner = "EffortlessMetrics"', 'owner = ""', 1)
        policy_text = policy_text.replace(
            'receipt_paths = ["policy/trust-lanes.toml"]',
            'receipt_paths = ["does/not/exist.md"]',
            1,
        )
        policy_text = policy_text.replace(
            'required_checks = ["focused proof"]',
            "required_checks = []",
            1,
        )

        status, output = self.run_validator(policy_text, strict=True)

        self.assertEqual(1, status)
        self.assertIn("schema_version must be 1", output)
        self.assertIn("owner must be a non-empty string", output)
        self.assertIn("missing trust-lane class: docs_status_only", output)
        self.assertIn("unknown trust-lane class: unknown_lane", output)
        self.assertIn("dependency_update.required_checks must not be empty", output)
        self.assertIn(
            "dependency_update.receipt_paths[0] does not resolve: does/not/exist.md",
            output,
        )

    def test_non_strict_reports_issues_without_failing(self) -> None:
        policy_text = self.valid_policy_text().replace(
            'status = "advisory"',
            'status = "blocking"',
            1,
        )

        status, output = self.run_validator(policy_text, strict=False)

        self.assertEqual(0, status)
        self.assertIn("Issues (1):", output)
        self.assertIn('status must be "advisory"', output)


if __name__ == "__main__":
    unittest.main()
