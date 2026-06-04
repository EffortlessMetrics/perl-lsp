#!/usr/bin/env python3
"""Focused tests for scripts/ci/aggregate_lane_history.py."""

from __future__ import annotations

import importlib.util
import io
import json
import os
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("aggregate_lane_history.py")
SPEC = importlib.util.spec_from_file_location("aggregate_lane_history", SCRIPT_PATH)
assert SPEC is not None
aggregate_lane_history = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(aggregate_lane_history)


class AggregateLaneHistoryTests(unittest.TestCase):
    def test_percentile_uses_linear_interpolation(self) -> None:
        self.assertEqual(0.0, aggregate_lane_history.percentile([], 95))
        self.assertEqual(42.0, aggregate_lane_history.percentile([42.0], 50))
        self.assertEqual(25.0, aggregate_lane_history.percentile([10.0, 20.0, 30.0, 40.0], 50))
        self.assertEqual(37.0, aggregate_lane_history.percentile([10.0, 20.0, 30.0, 40.0], 90))

    def test_collect_actuals_filters_old_invalid_and_incomplete_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            actuals = Path(tmp)
            fresh = actuals / "fresh" / "ci-actuals.json"
            fresh.parent.mkdir()
            fresh.write_text(
                json.dumps(
                    {
                        "jobs": [
                            {"gate_name": "rust-small", "actual_lem": 120},
                            {"lane_id": "ripr", "actual_lem": 42.5},
                            {"lane_id": "missing-actual"},
                            {"lane_id": "bad-actual", "actual_lem": "slow"},
                        ]
                    }
                ),
                encoding="utf-8",
            )
            old = actuals / "old.json"
            old.write_text(
                json.dumps({"jobs": [{"lane_id": "old", "actual_lem": 1}]}),
                encoding="utf-8",
            )
            old_time = time.time() - 3 * 86400
            os.utime(old, (old_time, old_time))
            (actuals / "invalid.json").write_text("{", encoding="utf-8")
            (actuals / "array.json").write_text("[]", encoding="utf-8")

            samples = aggregate_lane_history.collect_actuals(
                actuals_dir=actuals,
                window_days=1,
            )

        self.assertEqual({"ripr": [42.5], "rust-small": [120.0]}, samples)

    def test_static_floors_reads_lane_base_lem_values(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            lanes = Path(tmp) / "ci-lanes.toml"
            lanes.write_text(
                """
[lane.rust-small]
base_lem = 20

[lane.docs]
base_lem = 2.5

[lane.no-floor]
label = "No floor"
""",
                encoding="utf-8",
            )

            floors = aggregate_lane_history.static_floors(lanes)

        self.assertEqual({"docs": 2.5, "rust-small": 20.0}, floors)

    def test_build_history_includes_policy_lanes_without_samples(self) -> None:
        history = aggregate_lane_history.build_history(
            samples={"rust-small": [10, 20, 30, 40, 50]},
            floors={"docs": 3, "rust-small": 15},
            window_days=14,
        )

        self.assertEqual(1, history["schema_version"])
        self.assertEqual(2, history["lane_count"])
        self.assertFalse(history["lanes"]["docs"]["learned"])
        self.assertEqual(0, history["lanes"]["docs"]["samples"])
        self.assertTrue(history["lanes"]["rust-small"]["learned"])
        self.assertEqual(30, history["lanes"]["rust-small"]["p50"])
        self.assertEqual(46, history["lanes"]["rust-small"]["p90"])
        self.assertEqual(48, history["lanes"]["rust-small"]["p95"])
        self.assertEqual(30, history["lanes"]["rust-small"]["mean"])

    def test_main_writes_history_and_summary_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            actuals = root / "actuals"
            actuals.mkdir()
            (actuals / "ci-actuals.json").write_text(
                json.dumps({"jobs": [{"lane_id": "rust-small", "actual_lem": 12}]}),
                encoding="utf-8",
            )
            lanes = root / "ci-lanes.toml"
            lanes.write_text(
                """
[lane.rust-small]
base_lem = 10

[lane.docs]
base_lem = 2
""",
                encoding="utf-8",
            )
            output = root / "history.json"

            old_argv = sys.argv
            try:
                sys.argv = [
                    "aggregate_lane_history.py",
                    "--actuals-dir",
                    str(actuals),
                    "--window-days",
                    "14",
                    "--output",
                    str(output),
                    "--static-lanes",
                    str(lanes),
                ]
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    status = aggregate_lane_history.main()
            finally:
                sys.argv = old_argv

            history = json.loads(output.read_text(encoding="utf-8"))
            printed = json.loads(stdout.getvalue())

        self.assertEqual(0, status)
        self.assertEqual(2, history["lane_count"])
        self.assertEqual(1, history["lanes"]["rust-small"]["samples"])
        self.assertEqual({"lanes": 2, "learned": 0, "window_days": 14}, printed)


if __name__ == "__main__":
    unittest.main()
