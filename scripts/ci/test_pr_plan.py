#!/usr/bin/env python3
"""Focused tests for scripts/ci/pr_plan.py."""

from __future__ import annotations

import importlib.util
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("pr_plan.py")
SPEC = importlib.util.spec_from_file_location("pr_plan", SCRIPT_PATH)
assert SPEC is not None
pr_plan = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(pr_plan)


class PrPlanTests(unittest.TestCase):
    def test_path_matches_glob_handles_recursive_and_single_segment_patterns(self) -> None:
        self.assertTrue(pr_plan.path_matches_glob("docs/foo/bar.md", "docs/**"))
        self.assertTrue(
            pr_plan.path_matches_glob(
                "crates/perl-lsp-rs-core/src/providers/foo.rs",
                "crates/perl-lsp-rs-core/src/providers/**",
            )
        )
        self.assertTrue(pr_plan.path_matches_glob("docs/foo.md", "docs/*.md"))
        self.assertFalse(pr_plan.path_matches_glob("docs/nested/foo.md", "docs/*.md"))
        self.assertTrue(pr_plan.path_matches_glob("README.md", "README*"))

    def test_docs_only_requires_every_changed_file_to_be_docs(self) -> None:
        self.assertTrue(pr_plan.docs_only(["docs/a.md", "README.md"]))
        self.assertFalse(pr_plan.docs_only([]))
        self.assertFalse(pr_plan.docs_only(["docs/a.md", "crates/foo/src/lib.rs"]))

    def test_classify_areas_matches_path_and_keyword_risk_packs(self) -> None:
        risk_packs = {
            "parser": {"paths": ["crates/perl-parser/**"], "keywords": []},
            "coverage": {"paths": [], "keywords": ["coverage"]},
            "unmatched": {"paths": ["vscode-extension/**"], "keywords": ["release"]},
        }

        selected, areas = pr_plan.classify_areas(
            ["crates/perl-parser/src/lib.rs", "docs/coverage-plan.md"],
            risk_packs,
        )

        self.assertEqual(["parser", "coverage"], selected)
        self.assertEqual(["coverage", "parser"], areas)

    def test_select_lanes_keeps_docs_only_changes_on_docs_gate(self) -> None:
        lanes = {
            "docs_gate": {"base_lem": 2, "blocking": True},
            "rust_small": {"default_pr": True, "base_lem": 10, "blocking": True},
            "ripr_advisory": {
                "default_pr": True,
                "base_lem": 5,
                "paths": ["crates/**/*.rs"],
            },
        }

        selected, skipped = pr_plan.select_lanes(
            files=["docs/status.md"],
            labels=[],
            risk_pack_ids=[],
            risk_packs={},
            lanes=lanes,
        )

        self.assertEqual(["docs_gate"], [lane["id"] for lane in selected])
        self.assertEqual([], skipped)

    def test_select_lanes_reports_path_filtered_default_lane_when_it_does_not_match(self) -> None:
        lanes = {
            "rust_small": {"default_pr": True, "base_lem": 10, "blocking": True},
            "ripr_advisory": {
                "default_pr": True,
                "base_lem": 5,
                "paths": ["crates/**/*.rs"],
            },
        }

        selected, skipped = pr_plan.select_lanes(
            files=["scripts/ci/pr_plan.py"],
            labels=[],
            risk_pack_ids=[],
            risk_packs={},
            lanes=lanes,
        )

        self.assertEqual(["rust_small"], [lane["id"] for lane in selected])
        self.assertEqual(["ripr_advisory"], [lane["id"] for lane in skipped])
        self.assertEqual("paths-filter-no-match", skipped[0]["skipped_reason"])

    def test_apply_learned_estimates_uses_recent_p50_or_static_floor(self) -> None:
        lanes = [
            {"id": "rust_small", "base_lem": 10},
            {"id": "slow_lane", "base_lem": 100},
            {"id": "not_learned", "base_lem": 5},
        ]
        history = {
            "lanes": {
                "rust_small": {"learned": True, "p50": 20, "static_floor": 15},
                "slow_lane": {"learned": True, "p50": 20, "static_floor": 80},
                "not_learned": {"learned": False, "p50": 50, "static_floor": 5},
            }
        }

        delta, learned_count = pr_plan.apply_learned_estimates(lanes, history)

        self.assertEqual(-7.0, delta)
        self.assertEqual(2, learned_count)
        self.assertEqual(23.0, lanes[0]["base_lem"])
        self.assertEqual("learned (p50 * 1.15)", lanes[0]["learned_source"])
        self.assertEqual(80.0, lanes[1]["base_lem"])
        self.assertEqual("static_floor", lanes[1]["learned_source"])
        self.assertEqual(5, lanes[2]["base_lem"])

    def test_main_writes_plan_summary_and_trust_lane_for_pr_plan_helper(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            budget = root / "ci-budget.toml"
            budget.write_text(
                """
[budget]
default_limit_lem = 35
elevated_limit_lem = 75
hard_limit_lem = 125
linux_minute_rate_usd = 0.008
""",
                encoding="utf-8",
            )
            lanes = root / "ci-lanes.toml"
            lanes.write_text(
                """
[lane.docs_gate]
base_lem = 2
blocking = true

[lane.rust_small]
default_pr = true
base_lem = 10
blocking = true

[lane.ripr_advisory]
default_pr = true
base_lem = 5
paths = ["crates/**/*.rs"]
""",
                encoding="utf-8",
            )
            risk_packs = root / "ci-risk-packs.toml"
            risk_packs.write_text("", encoding="utf-8")
            trust_lanes = root / "trust-lanes.toml"
            trust_lanes.write_text(
                """
schema_version = 1
policy = "trust-lanes"
status = "advisory"

[class.docs_status_only]
risk_rank = 1
claim_boundary = "Docs, status, policy, and CI planning only."
required_checks = ["docs"]
""",
                encoding="utf-8",
            )
            output = root / "ci-plan.json"
            summary = root / "summary.md"

            old_argv = sys.argv
            old_changed_files = pr_plan.changed_files
            try:
                pr_plan.changed_files = lambda _base, _head: ["scripts/ci/pr_plan.py"]
                sys.argv = [
                    "pr_plan.py",
                    "--base",
                    "origin/main",
                    "--head",
                    "HEAD",
                    "--labels-json",
                    "[]",
                    "--budget",
                    str(budget),
                    "--lanes",
                    str(lanes),
                    "--risk-packs",
                    str(risk_packs),
                    "--trust-lanes",
                    str(trust_lanes),
                    "--json-out",
                    str(output),
                    "--summary",
                    str(summary),
                ]
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    status = pr_plan.main()
            finally:
                sys.argv = old_argv
                pr_plan.changed_files = old_changed_files

            plan = json.loads(output.read_text(encoding="utf-8"))
            printed = json.loads(stdout.getvalue())
            summary_text = summary.read_text(encoding="utf-8")

        self.assertEqual(0, status)
        self.assertEqual(["scripts/ci/pr_plan.py"], plan["changed"]["files"])
        self.assertEqual("docs_status_only", plan["trust_lanes"]["strongest_class"]["id"])
        self.assertEqual(["rust_small"], [lane["id"] for lane in plan["selection"]["lanes"]])
        self.assertEqual(["ripr_advisory"], [lane["id"] for lane in plan["selection"]["skipped_lanes"]])
        self.assertEqual({"estimated_lem": 10.0, "band": "default", "lanes": 1}, printed)
        self.assertIn("## Trust lane (advisory)", summary_text)
        self.assertIn("`docs_status_only`", summary_text)
        self.assertIn("`ripr_advisory` | paths-filter-no-match", summary_text)


if __name__ == "__main__":
    unittest.main()
