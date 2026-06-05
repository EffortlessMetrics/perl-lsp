#!/usr/bin/env python3
"""Focused tests for scripts/ci/emit_ci_actuals.py."""

from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from emit_ci_actuals import collect_receipts, emit_actuals, main  # noqa: E402


class EmitCiActualsTests(unittest.TestCase):
    def test_collect_receipts_unwraps_gate_arrays_and_single_gate_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            receipts_dir = Path(tmp)
            (receipts_dir / "gates.json").write_text(
                json.dumps(
                    {
                        "gates": [
                            {
                                "gate_name": "pr_smoke",
                                "status": "pass",
                                "duration_ms": 120000,
                            },
                            "ignored",
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (receipts_dir / "single.json").write_text(
                json.dumps(
                    {
                        "gate_name": "rust_small",
                        "status": "pass",
                        "duration_ms": 60000,
                    }
                ),
                encoding="utf-8",
            )
            (receipts_dir / "invalid.json").write_text("{", encoding="utf-8")

            receipts = collect_receipts(receipts_dir)

        self.assertEqual(
            ["pr_smoke", "rust_small"],
            [receipt["gate_name"] for receipt in receipts],
        )
        self.assertTrue(all(receipt.get("_source_path") for receipt in receipts))

    def test_emit_actuals_computes_actual_and_estimated_lem(self) -> None:
        actuals = emit_actuals(
            receipts=[
                {
                    "gate_name": "pr_smoke",
                    "tier": "pr_fast",
                    "status": "pass",
                    "runner": "ubuntu_24_04",
                    "duration_ms": 120000,
                    "_source_path": "target/receipts/pr-smoke.json",
                },
                {
                    "gate_name": "missing_duration",
                    "status": "pass",
                    "_source_path": "target/receipts/missing-duration.json",
                },
            ],
            multipliers={"ubuntu_24_04": 2.0},
            lanes={"pr_smoke": {"base_lem": 5.0}},
            workflow="CI",
            sha="abc123",
            pr=42,
            runner_default="ubuntu_24_04",
        )

        self.assertEqual(1, actuals["schema_version"])
        self.assertEqual("abc123", actuals["sha"])
        self.assertEqual(42, actuals["pr"])
        self.assertEqual(4.0, actuals["totals"]["actual_lem"])
        self.assertEqual(5.0, actuals["totals"]["estimated_lem"])
        self.assertEqual(-1.0, actuals["totals"]["delta_lem"])
        self.assertEqual(2.0, actuals["jobs"][0]["actual_minutes"])
        self.assertEqual(4.0, actuals["jobs"][0]["actual_lem"])
        self.assertEqual(5.0, actuals["jobs"][0]["estimated_lem"])
        self.assertIsNone(actuals["jobs"][1]["actual_minutes"])
        self.assertIsNone(actuals["jobs"][1]["actual_lem"])
        self.assertIsNone(actuals["jobs"][1]["estimated_lem"])

    def test_main_writes_actuals_json_from_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            receipts_dir = root / "receipts"
            receipts_dir.mkdir()
            (receipts_dir / "receipt.json").write_text(
                json.dumps(
                    {
                        "gate_name": "pr_smoke",
                        "tier": "pr_fast",
                        "status": "pass",
                        "duration_ms": 60000,
                    }
                ),
                encoding="utf-8",
            )
            budget = root / "ci-budget.toml"
            budget.write_text(
                "[runner_multipliers]\nubuntu_24_04 = 3.0\n",
                encoding="utf-8",
            )
            lanes = root / "ci-lanes.toml"
            lanes.write_text("[lane.pr_smoke]\nbase_lem = 7.0\n", encoding="utf-8")
            json_out = root / "ci-actuals.json"

            old_argv = sys.argv
            try:
                sys.argv = [
                    "emit_ci_actuals.py",
                    "--receipts-dir",
                    str(receipts_dir),
                    "--budget",
                    str(budget),
                    "--lanes",
                    str(lanes),
                    "--json-out",
                    str(json_out),
                    "--workflow",
                    "CI",
                    "--sha",
                    "abc123",
                    "--pr",
                    "42",
                    "--runner-default",
                    "ubuntu_24_04",
                ]
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    self.assertEqual(0, main())
            finally:
                sys.argv = old_argv

            actuals = json.loads(json_out.read_text(encoding="utf-8"))

        self.assertEqual("CI", actuals["workflow"])
        self.assertIn('"jobs": 1', stdout.getvalue())
        self.assertEqual("abc123", actuals["sha"])
        self.assertEqual(42, actuals["pr"])
        self.assertEqual(3.0, actuals["totals"]["actual_lem"])
        self.assertEqual(7.0, actuals["totals"]["estimated_lem"])
        self.assertEqual("pr_smoke", actuals["jobs"][0]["gate_name"])


if __name__ == "__main__":
    unittest.main()
