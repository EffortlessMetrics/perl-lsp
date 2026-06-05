#!/usr/bin/env python3
"""Tests for scripts/ci/ci_classify.py.

Covers the pure classify_one() function with offline fixtures.
No live GitHub API calls.  No external dependencies beyond stdlib.

Run with:
    python3 scripts/ci/test_ci_classify.py
    python3 -m unittest scripts.ci.test_ci_classify  (from repo root)

Exit code 0 on all-pass, non-zero on any failure.
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# ---------------------------------------------------------------------------
# Path setup: add scripts/ci to sys.path so we can import ci_classify directly.
# ---------------------------------------------------------------------------
_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from ci_classify import (  # noqa: E402
    CLASS_COVERAGE_ARTIFACT,
    CLASS_EXPECTED_PATH_SKIP,
    CLASS_INFRA_ISSUE,
    CLASS_POLICY_MISMATCH,
    CLASS_PRODUCT_DEFECT,
    CLASS_REVIEW_GATE,
    CLASS_UNKNOWN,
    classify_one,
    filter_failing,
    load_check_runs,
)

# Fixtures directory (sibling to this test file).
FIXTURES_DIR = _HERE / "fixtures"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _check(
    name: str,
    conclusion: str = "failure",
    *,
    required: bool = True,
    quarantine: bool = False,
    run_ci: bool = True,
    is_latest: bool = True,
) -> dict:
    return {
        "name": name,
        "conclusion": conclusion,
        "required": required,
        "quarantine": quarantine,
        "run_ci": run_ci,
        "is_latest": is_latest,
    }


def _cls(name: str, **kwargs: object) -> str:
    cls, _ = classify_one(_check(name, **kwargs))
    return cls


# ---------------------------------------------------------------------------
# Core classification unit tests (4 required by spec)
# ---------------------------------------------------------------------------


class TestProductDefect(unittest.TestCase):
    """Spec requirement: product_defect — gate fails, quarantine=false, core gate."""

    def test_lsp_shard_failure(self) -> None:
        cls, rationale = classify_one(
            _check("CI Gate shard (lsp)", "failure", quarantine=False, required=True)
        )
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)
        self.assertIn("product gate", rationale)

    def test_merge_blocking_failure(self) -> None:
        cls, _ = classify_one(
            _check("CI Gate (Merge-Blocking)", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)

    def test_ux_regression_tests(self) -> None:
        cls, _ = classify_one(
            _check("UX Regression Tests", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)

    def test_lsp_memory_smoke(self) -> None:
        cls, _ = classify_one(
            _check("LSP Memory Smoke", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)

    def test_pr_smoke(self) -> None:
        cls, _ = classify_one(_check("pr-smoke", "failure", quarantine=False))
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)

    def test_corpus_gate_failure(self) -> None:
        cls, _ = classify_one(
            _check("CI Gate shard (corpus)", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_PRODUCT_DEFECT)


class TestInfraIssue(unittest.TestCase):
    """Spec requirement: infra_issue — conclusion=cancelled or timed_out."""

    def test_cancelled_gate(self) -> None:
        cls, rationale = classify_one(
            _check("CI Gate shard (corpus)", "cancelled", quarantine=False)
        )
        self.assertEqual(cls, CLASS_INFRA_ISSUE)
        self.assertIn("cancelled", rationale)

    def test_timed_out_compile(self) -> None:
        cls, rationale = classify_one(
            _check("Compile All Targets", "timed_out", quarantine=False)
        )
        self.assertEqual(cls, CLASS_INFRA_ISSUE)
        self.assertIn("timed_out", rationale)

    def test_cancelled_takes_priority_over_product(self) -> None:
        """Infra cancellation should win over product_defect classification."""
        cls, _ = classify_one(
            _check("CI Gate shard (lsp)", "cancelled", quarantine=False, required=True)
        )
        self.assertEqual(cls, CLASS_INFRA_ISSUE)

    def test_quarantine_beats_timed_out_for_coverage_gate(self) -> None:
        """Quarantined coverage gate that times out → coverage_artifact, not infra_issue.

        A quarantined gate cannot meaningfully be 'retried' — the quarantine is the
        authoritative policy signal.  Routing as infra_issue would incorrectly suggest
        a retry is safe.
        """
        cls, _ = classify_one(
            _check("security_audit", "timed_out", quarantine=True)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)

    def test_quarantine_beats_cancelled_for_non_coverage_gate(self) -> None:
        """Quarantined non-coverage gate that is cancelled → expected_path_skip, not infra_issue."""
        cls, _ = classify_one(
            _check("some-quarantined-gate", "cancelled", quarantine=True)
        )
        self.assertEqual(cls, CLASS_EXPECTED_PATH_SKIP)


class TestPolicyMismatch(unittest.TestCase):
    """Spec requirement: policy_mismatch — mechanical-correctness gate, quarantine=false."""

    def test_fmt_gate(self) -> None:
        cls, rationale = classify_one(
            _check("fmt", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)
        self.assertIn("mechanical-correctness", rationale)

    def test_conflict_markers(self) -> None:
        cls, _ = classify_one(
            _check("check_conflict_markers", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)

    def test_conflict_markers_alt_name(self) -> None:
        """conflict-markers (GitHub job name variant) should also match."""
        cls, _ = classify_one(
            _check("conflict-markers", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)

    def test_publish_manifest_check(self) -> None:
        cls, _ = classify_one(
            _check("publish_manifest_check", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)

    def test_layer_check(self) -> None:
        cls, _ = classify_one(
            _check("layer_check", "failure", quarantine=False)
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)


class TestExpectedPathSkip(unittest.TestCase):
    """Spec requirement: expected_path_skip — quarantine=true or required=false for non-coverage gates."""

    def test_windows_required_false(self) -> None:
        """Windows Required with required=false is policy-sanctioned path skip."""
        cls, rationale = classify_one(
            _check("Windows Required", "failure", required=False, quarantine=False)
        )
        self.assertEqual(cls, CLASS_EXPECTED_PATH_SKIP)
        self.assertIn("required=false", rationale)

    def test_quarantined_non_coverage_gate(self) -> None:
        """A quarantined gate that is not a measurement/quality gate → expected_path_skip."""
        cls, rationale = classify_one(
            _check("some-experimental-check", "failure", quarantine=True)
        )
        self.assertEqual(cls, CLASS_EXPECTED_PATH_SKIP)
        self.assertIn("quarantine=true", rationale)

    def test_quarantined_product_gate_is_expected_path_skip(self) -> None:
        """A quarantined product gate must never reach product_defect — expected_path_skip wins."""
        cls, _ = classify_one(
            _check("CI Gate shard (lsp)", "failure", quarantine=True)
        )
        self.assertEqual(cls, CLASS_EXPECTED_PATH_SKIP)


class TestReviewGate(unittest.TestCase):
    """Spec requirement: review_gate — draft or superseded-SHA skip."""

    def test_draft_pr_check_run_ci_false(self) -> None:
        cls, rationale = classify_one(
            _check("draft-pr-check", "failure", run_ci=False)
        )
        self.assertEqual(cls, CLASS_REVIEW_GATE)
        self.assertIn("draft", rationale)

    def test_preflight_latest_not_latest(self) -> None:
        cls, rationale = classify_one(
            _check("preflight-latest-check", "failure", is_latest=False)
        )
        self.assertEqual(cls, CLASS_REVIEW_GATE)
        self.assertIn("superseded", rationale)

    def test_draft_pr_check_run_ci_true_falls_through(self) -> None:
        """draft-pr-check with run_ci=True should NOT classify as review_gate."""
        # If run_ci=true there's a real failure — falls through to product/unknown.
        cls, _ = classify_one(
            _check("draft-pr-check", "failure", run_ci=True, quarantine=False)
        )
        self.assertNotEqual(cls, CLASS_REVIEW_GATE)

    def test_preflight_latest_is_latest_falls_through(self) -> None:
        """preflight-latest-check with is_latest=True should NOT classify as review_gate."""
        cls, _ = classify_one(
            _check("preflight-latest-check", "failure", is_latest=True, quarantine=False)
        )
        self.assertNotEqual(cls, CLASS_REVIEW_GATE)


class TestCoverageArtifact(unittest.TestCase):
    """coverage_artifact — measurement/quality gate whose threshold or tooling drifted."""

    def test_coverage_skipped(self) -> None:
        cls, _ = classify_one(
            {"name": "coverage-baseline-check", "conclusion": "skipped"}
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)

    def test_coverage_keyword_skipped(self) -> None:
        cls, _ = classify_one(
            {"name": "coverage-drift", "conclusion": "neutral"}
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)

    def test_spec_acceptance_3_security_audit_quarantined(self) -> None:
        """Spec acceptance test 3: security_audit quarantine=true → coverage_artifact.

        Rationale per issue #907: security_audit is quarantined because cargo-audit
        breaks on CVSS 4.0 — environmental tooling issue, not a product regression.
        Routing: log & ignore; do not block merge.
        """
        cls, rationale = classify_one(
            _check("security_audit", "failure", quarantine=True)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)
        self.assertIn("measurement/quality gate", rationale)

    def test_spec_acceptance_8_parser_corpus_ratchet_required_false(self) -> None:
        """Spec acceptance test 8: parser_corpus_ratchet required=false → coverage_artifact.

        Rationale per issue #907: corpus ratchet with required=false indicates baseline
        drift, not a product regression.  Routing: log & ignore; do not block merge.
        """
        cls, rationale = classify_one(
            _check("parser_corpus_ratchet", "failure", required=False)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)
        self.assertIn("measurement/quality gate", rationale)

    def test_mutation_subset_required_false_is_coverage_artifact(self) -> None:
        """mutation-subset with required=false → coverage_artifact (measurement gate)."""
        cls, _ = classify_one(
            _check("mutation-subset", "failure", quarantine=False, required=False)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)

    def test_quarantined_mutation_is_coverage_artifact(self) -> None:
        """mutation quarantined → coverage_artifact, not expected_path_skip."""
        cls, _ = classify_one(
            _check("mutation", "failure", quarantine=True)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)

    def test_quarantined_fuzz_is_coverage_artifact(self) -> None:
        """fuzz quarantined → coverage_artifact, not expected_path_skip."""
        cls, _ = classify_one(
            _check("fuzz", "failure", quarantine=True)
        )
        self.assertEqual(cls, CLASS_COVERAGE_ARTIFACT)


class TestUnknown(unittest.TestCase):
    """unknown — no pattern matches."""

    def test_unknown_check(self) -> None:
        cls, rationale = classify_one(
            _check("some-new-experimental-gate", "failure", quarantine=False, required=True)
        )
        self.assertEqual(cls, CLASS_UNKNOWN)
        self.assertIn("no classification pattern matched", rationale)

    def test_empty_name(self) -> None:
        cls, _ = classify_one(
            {"name": "", "conclusion": "failure"}
        )
        self.assertEqual(cls, CLASS_UNKNOWN)


# ---------------------------------------------------------------------------
# Missing-field robustness tests
# ---------------------------------------------------------------------------


class TestMissingFieldGraceful(unittest.TestCase):
    """classify_one must not raise when optional fields are absent."""

    def test_minimal_input(self) -> None:
        """Only name + conclusion present — all defaults apply."""
        cls, rationale = classify_one({"name": "fmt", "conclusion": "failure"})
        # fmt with defaults (quarantine=false, required=true) → policy_mismatch
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)
        self.assertIsInstance(rationale, str)
        self.assertTrue(rationale)

    def test_no_conclusion(self) -> None:
        """Missing conclusion — treat as empty string, no infra match."""
        cls, _ = classify_one({"name": "some-check"})
        # No conclusion, no quarantine, no pattern → unknown
        self.assertEqual(cls, CLASS_UNKNOWN)

    def test_no_name(self) -> None:
        """Missing name — should still return a class without raising."""
        cls, rationale = classify_one({"conclusion": "cancelled"})
        # conclusion=cancelled → infra_issue
        self.assertEqual(cls, CLASS_INFRA_ISSUE)

    def test_extra_unknown_fields_ignored(self) -> None:
        """Extra fields in the input must not cause errors."""
        cls, _ = classify_one(
            {
                "name": "fmt",
                "conclusion": "failure",
                "extra_field": "ignored",
                "another": 42,
            }
        )
        self.assertEqual(cls, CLASS_POLICY_MISMATCH)


# ---------------------------------------------------------------------------
# filter_failing tests
# ---------------------------------------------------------------------------


class TestFilterFailing(unittest.TestCase):
    def test_filters_success(self) -> None:
        checks = [
            {"name": "a", "conclusion": "success"},
            {"name": "b", "conclusion": "failure"},
        ]
        result = filter_failing(checks)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "b")

    def test_cancelled_included(self) -> None:
        checks = [{"name": "a", "conclusion": "cancelled"}]
        self.assertEqual(len(filter_failing(checks)), 1)

    def test_timed_out_included(self) -> None:
        checks = [{"name": "a", "conclusion": "timed_out"}]
        self.assertEqual(len(filter_failing(checks)), 1)

    def test_skipped_excluded(self) -> None:
        checks = [{"name": "a", "conclusion": "skipped"}]
        self.assertEqual(len(filter_failing(checks)), 0)

    def test_empty_list(self) -> None:
        self.assertEqual(filter_failing([]), [])


# ---------------------------------------------------------------------------
# Fixture-based integration tests
# ---------------------------------------------------------------------------


class TestFixtures(unittest.TestCase):
    """Load realistic fixture JSON files and verify expected classifications."""

    def _load_fixture(self, filename: str) -> list[dict]:
        path = FIXTURES_DIR / filename
        return json.loads(path.read_text(encoding="utf-8"))

    def test_product_defect_fixture(self) -> None:
        checks = self._load_fixture("product_defect.json")
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_PRODUCT_DEFECT,
                f"Expected product_defect for {check.get('name')!r}, got {cls!r}",
            )

    def test_infra_issue_fixture(self) -> None:
        checks = self._load_fixture("infra_issue.json")
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_INFRA_ISSUE,
                f"Expected infra_issue for {check.get('name')!r}, got {cls!r}",
            )

    def test_policy_mismatch_fixture(self) -> None:
        checks = self._load_fixture("policy_mismatch.json")
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_POLICY_MISMATCH,
                f"Expected policy_mismatch for {check.get('name')!r}, got {cls!r}",
            )

    def test_expected_path_skip_fixture(self) -> None:
        checks = self._load_fixture("expected_path_skip.json")
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_EXPECTED_PATH_SKIP,
                f"Expected expected_path_skip for {check.get('name')!r}, got {cls!r}",
            )

    def test_review_gate_fixture(self) -> None:
        checks = self._load_fixture("review_gate.json")
        # review_gate checks have conclusion=failure so they appear in filter_failing
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_REVIEW_GATE,
                f"Expected review_gate for {check.get('name')!r}, got {cls!r}",
            )

    def test_coverage_artifact_fixture(self) -> None:
        """Spec acceptance tests 3 and 8: coverage/quality gates are coverage_artifact."""
        checks = self._load_fixture("coverage_artifact.json")
        failing = filter_failing(checks)
        self.assertGreater(len(failing), 0)
        for check in failing:
            cls, _ = classify_one(check)
            self.assertEqual(
                cls,
                CLASS_COVERAGE_ARTIFACT,
                f"Expected coverage_artifact for {check.get('name')!r}, got {cls!r}",
            )

    def test_mixed_fixture_has_expected_classes(self) -> None:
        checks = self._load_fixture("mixed.json")
        failing = filter_failing(checks)
        classes = {check.get("name"): classify_one(check)[0] for check in failing}

        self.assertEqual(classes.get("fmt"), CLASS_POLICY_MISMATCH)
        self.assertEqual(classes.get("CI Gate shard (lsp)"), CLASS_PRODUCT_DEFECT)
        # cancelled → infra
        self.assertEqual(classes.get("CI Gate shard (corpus)"), CLASS_INFRA_ISSUE)
        # security_audit quarantine=true → coverage_artifact (measurement gate, not path skip)
        self.assertEqual(classes.get("security_audit"), CLASS_COVERAGE_ARTIFACT)
        # draft → review_gate
        self.assertEqual(classes.get("draft-pr-check"), CLASS_REVIEW_GATE)
        # unknown
        self.assertEqual(classes.get("some-new-experimental-check"), CLASS_UNKNOWN)


# ---------------------------------------------------------------------------
# load_check_runs tests
# ---------------------------------------------------------------------------


class TestLoadCheckRuns(unittest.TestCase):
    def test_load_from_file(self) -> None:
        data = [{"name": "fmt", "conclusion": "failure"}]
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as f:
            json.dump(data, f)
            fname = f.name
        try:
            result = load_check_runs(fname)
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0]["name"], "fmt")
        finally:
            os.unlink(fname)

    def test_load_github_envelope(self) -> None:
        """GitHub API envelopes with check_runs key are unwrapped."""
        data = {
            "check_runs": [
                {"name": "a", "conclusion": "failure"},
                {"name": "b", "conclusion": "success"},
            ]
        }
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as f:
            json.dump(data, f)
            fname = f.name
        try:
            result = load_check_runs(fname)
            self.assertEqual(len(result), 2)
        finally:
            os.unlink(fname)


if __name__ == "__main__":
    unittest.main()
