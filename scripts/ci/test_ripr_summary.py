#!/usr/bin/env python3
"""Focused tests for scripts/ci/ripr_summary.py."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from ripr_summary import classify, collect_findings, main, render  # noqa: E402


class RiprSummaryTests(unittest.TestCase):
    def test_collect_findings_accepts_supported_report_shapes(self) -> None:
        self.assertEqual(
            [{"classification": "exposed"}],
            collect_findings([{"classification": "exposed"}, "ignored"]),
        )
        self.assertEqual(
            [{"classification": "weakly_exposed"}],
            collect_findings({"findings": [{"classification": "weakly_exposed"}]}),
        )
        self.assertEqual(
            [{"classification": "static_unknown"}],
            collect_findings({"results": [{"classification": "static_unknown"}]}),
        )
        self.assertEqual([], collect_findings({"findings": "not a list"}))

    def test_classify_uses_schema_aliases_and_static_unknown_default(self) -> None:
        self.assertEqual("exposed", classify({"classification": "Exposed"}))
        self.assertEqual("weakly_exposed", classify({"class": "Weakly_Exposed"}))
        self.assertEqual("reachable_unrevealed", classify({"category": "Reachable_Unrevealed"}))
        self.assertEqual("infection_unknown", classify({"severity": "Infection_Unknown"}))
        self.assertEqual("static_unknown", classify({}))

    def test_render_sorts_and_escapes_actionable_findings(self) -> None:
        summary = render(
            [
                {
                    "classification": "exposed",
                    "location": {"file": "crates\\foo|bar.rs", "line": 12},
                    "related_tests": ["proof|one", "proof_two", "proof_three", "proof_four"],
                },
                {
                    "severity": "custom_severity",
                    "path": "scripts\\ci\\ripr_summary.py",
                    "tests": "manual|proof",
                },
            ]
        )

        self.assertIn("| `exposed` | 1 |", summary)
        self.assertIn("| `custom_severity` | 1 |", summary)
        self.assertIn(
            "| `exposed` | crates/foo\\|bar.rs:12 | proof\\|one, proof_two, proof_three (+1) |",
            summary,
        )
        self.assertIn(
            "| `custom_severity` | scripts/ci/ripr_summary.py | manual\\|proof |",
            summary,
        )

    def test_render_no_findings_names_advisory_rollout(self) -> None:
        summary = render([])

        self.assertIn("No oracle-gap findings on the changed Rust diff.", summary)
        self.assertIn("ripr is advisory in this rollout", summary)

    def test_main_appends_missing_and_invalid_report_summaries(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            summary = root / "summary.md"
            missing_report = root / "missing.json"
            invalid_report = root / "invalid.json"
            invalid_report.write_text("{", encoding="utf-8")

            old_argv = sys.argv
            try:
                sys.argv = [
                    "ripr_summary.py",
                    "--report",
                    str(missing_report),
                    "--summary",
                    str(summary),
                ]
                self.assertEqual(0, main())
                sys.argv = [
                    "ripr_summary.py",
                    "--report",
                    str(invalid_report),
                    "--summary",
                    str(summary),
                ]
                self.assertEqual(0, main())
            finally:
                sys.argv = old_argv

            text = summary.read_text(encoding="utf-8")

        self.assertIn("Report file empty or missing.", text)
        self.assertIn("Could not parse report:", text)


if __name__ == "__main__":
    unittest.main()
