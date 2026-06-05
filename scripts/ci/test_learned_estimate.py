#!/usr/bin/env python3
"""Focused tests for scripts/ci/learned_estimate.py."""

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

from learned_estimate import estimate_for, main  # noqa: E402


class LearnedEstimateTests(unittest.TestCase):
    def test_estimate_for_reports_missing_lane_history(self) -> None:
        estimate = estimate_for("rust_small", {"lanes": {}})

        self.assertFalse(estimate["learned"])
        self.assertEqual("rust_small", estimate["lane"])
        self.assertIsNone(estimate["estimate"])
        self.assertEqual(0, estimate["samples"])
        self.assertEqual("no history entry for this lane", estimate["reason"])

    def test_estimate_for_uses_static_floor_until_lane_has_enough_samples(self) -> None:
        estimate = estimate_for(
            "rust_small",
            {
                "min_samples_for_learned": 7,
                "lanes": {
                    "rust_small": {
                        "learned": False,
                        "static_floor": 12.0,
                        "samples": 3,
                    }
                },
            },
        )

        self.assertFalse(estimate["learned"])
        self.assertEqual(12.0, estimate["estimate"])
        self.assertEqual(12.0, estimate["static_floor"])
        self.assertEqual(3, estimate["samples"])
        self.assertEqual("only 3 samples; need 7 to learn", estimate["reason"])

    def test_estimate_for_uses_higher_static_floor_over_learned_p50(self) -> None:
        estimate = estimate_for(
            "rust_small",
            {
                "lanes": {
                    "rust_small": {
                        "learned": True,
                        "static_floor": 20.0,
                        "p50": 10.0,
                        "p90": 18.0,
                        "p95": 24.0,
                        "samples": 9,
                    }
                }
            },
        )

        self.assertTrue(estimate["learned"])
        self.assertEqual(20.0, estimate["estimate"])
        self.assertEqual("static_floor (higher than learned)", estimate["estimate_source"])
        self.assertEqual(10.0, estimate["p50"])
        self.assertEqual(18.0, estimate["p90_warning"])
        self.assertEqual(24.0, estimate["p95_hard_planning"])

    def test_estimate_for_uses_learned_p50_when_it_exceeds_static_floor(self) -> None:
        estimate = estimate_for(
            "rust_small",
            {
                "lanes": {
                    "rust_small": {
                        "learned": True,
                        "static_floor": 10.0,
                        "p50": 20.0,
                        "p90": 30.0,
                        "p95": 35.0,
                        "samples": 12,
                    }
                }
            },
        )

        self.assertTrue(estimate["learned"])
        self.assertEqual(23.0, estimate["estimate"])
        self.assertEqual("p50 * 1.15", estimate["estimate_source"])
        self.assertEqual(12, estimate["samples"])

    def test_main_prints_advisory_json_for_missing_and_invalid_history(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            missing_history = root / "missing.json"
            invalid_history = root / "invalid.json"
            invalid_history.write_text("{", encoding="utf-8")

            old_argv = sys.argv
            try:
                sys.argv = [
                    "learned_estimate.py",
                    "--history",
                    str(missing_history),
                    "--lane",
                    "rust_small",
                ]
                missing_stdout = io.StringIO()
                with redirect_stdout(missing_stdout):
                    self.assertEqual(0, main())

                sys.argv = [
                    "learned_estimate.py",
                    "--history",
                    str(invalid_history),
                    "--lane",
                    "rust_small",
                ]
                invalid_stdout = io.StringIO()
                with redirect_stdout(invalid_stdout):
                    self.assertEqual(0, main())
            finally:
                sys.argv = old_argv

        missing = json.loads(missing_stdout.getvalue())
        invalid = json.loads(invalid_stdout.getvalue())
        self.assertFalse(missing["learned"])
        self.assertIn("not present", missing["reason"])
        self.assertFalse(invalid["learned"])
        self.assertIn("error", invalid)


if __name__ == "__main__":
    unittest.main()
