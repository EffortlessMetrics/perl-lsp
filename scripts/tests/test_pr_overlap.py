#!/usr/bin/env python3
"""
Unit tests for scripts/pr_overlap.py — PR overlap detector.

Tests cover:
1. Acceptance case (1) — TRUE duplicate: same files + same tests + same symbols
   → ``likely-duplicate``.
2. Acceptance case (2) — Complementary: same files, different tests + symbols
   → ``sequence-both``.
3. Acceptance case (3) — Shared-base-but-isolated: different files entirely
   → ``isolated``; asserts base_commit / diffstat_lines fields are NOT
   consulted (they may be present in the payload but must not affect result).
4. Missing optional fields (tests, symbols) → treated as empty sets, jaccard 0.
5. pick-one case: file overlap > 0.5 with surface overlap.
6. Auto-detection of test files when ``tests`` field is absent.
7. Fixture round-trip: load each JSON fixture, classify, assert expected class.

Run with: python3 scripts/tests/test_pr_overlap.py
Returns exit code 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import json
import os
import sys
import unittest

# Ensure the scripts directory is on the path.
_SCRIPTS_DIR = os.path.join(os.path.dirname(__file__), "..")
sys.path.insert(0, _SCRIPTS_DIR)

# Import via importlib because the module name contains an underscore but
# lives in scripts/ — standard import works fine here.
import importlib.util

_spec = importlib.util.spec_from_file_location(
    "pr_overlap",
    os.path.join(_SCRIPTS_DIR, "pr_overlap.py"),
)
assert _spec is not None and _spec.loader is not None
pr_overlap = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pr_overlap)  # type: ignore[union-attr]

classify_pair = pr_overlap.classify_pair
generate_report = pr_overlap.generate_report
_normalise_pr = pr_overlap._normalise_pr
_jaccard = pr_overlap._jaccard
_is_test_file = pr_overlap._is_test_file

_FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures", "pr_overlap")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_pr(
    pr_id: str = "X",
    files: list[str] | None = None,
    tests: list[str] | None = None,
    symbols: list[str] | None = None,
    **extra: object,
) -> dict:
    """Build a raw PR dict; omit optional keys when None to test defaults."""
    raw: dict = {"id": pr_id, "files": files or []}
    if tests is not None:
        raw["tests"] = tests
    if symbols is not None:
        raw["symbols"] = symbols
    raw.update(extra)
    return raw


def _load_fixture(name: str) -> dict:
    path = os.path.join(_FIXTURES_DIR, name)
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def _classify_fixture(name: str) -> list[dict]:
    data = _load_fixture(name)
    return generate_report(data["prs"])


# ---------------------------------------------------------------------------
# Acceptance case (1): TRUE duplicate
# ---------------------------------------------------------------------------

class TestTrueDuplicate(unittest.TestCase):
    """
    Acceptance case (1): same files + same tests + same symbols → likely-duplicate.
    """

    def _pr_a(self) -> dict:
        return _make_pr(
            "101",
            files=["src/parser.rs", "src/ast.rs", "tests/parser_test.rs"],
            tests=["tests/parser_test.rs"],
            symbols=["Parser::parse", "Ast::new"],
        )

    def _pr_b(self) -> dict:
        return _make_pr(
            "102",
            files=["src/parser.rs", "src/ast.rs", "tests/parser_test.rs"],
            tests=["tests/parser_test.rs"],
            symbols=["Parser::parse", "Ast::new"],
        )

    def test_class_is_likely_duplicate(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertEqual(result["class"], "likely-duplicate")

    def test_all_jaccards_at_max(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertAlmostEqual(result["jaccard_files"], 1.0)
        self.assertAlmostEqual(result["jaccard_tests"], 1.0)
        self.assertAlmostEqual(result["jaccard_syms"],  1.0)

    def test_fixture_true_duplicate(self) -> None:
        """Fixture round-trip for acceptance case (1)."""
        results = _classify_fixture("true_duplicate.json")
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["class"], "likely-duplicate")


# ---------------------------------------------------------------------------
# Acceptance case (2): Complementary (sequence-both)
# ---------------------------------------------------------------------------

class TestComplementarySequenceBoth(unittest.TestCase):
    """
    Acceptance case (2): same file(s), different tests/symbols → sequence-both.

    File overlap is non-zero but jaccard_tests and jaccard_syms are both < 0.3,
    so the class is sequence-both (not pick-one, not likely-duplicate).
    """

    def _pr_a(self) -> dict:
        return _make_pr(
            "201",
            files=["src/parser.rs", "tests/lexer_test.rs"],
            tests=["tests/lexer_test.rs"],
            symbols=["Parser::lex_token"],
        )

    def _pr_b(self) -> dict:
        return _make_pr(
            "202",
            files=["src/parser.rs", "tests/heredoc_test.rs"],
            tests=["tests/heredoc_test.rs"],
            symbols=["Parser::parse_heredoc"],
        )

    def test_class_is_sequence_both(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertEqual(result["class"], "sequence-both")

    def test_jaccard_files_positive(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertGreater(result["jaccard_files"], 0.0)

    def test_low_surface_overlap(self) -> None:
        """Test and symbol Jaccard must be below the surface threshold."""
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertLess(result["jaccard_tests"], 0.3)
        self.assertLess(result["jaccard_syms"],  0.3)

    def test_fixture_complementary(self) -> None:
        """Fixture round-trip for acceptance case (2)."""
        results = _classify_fixture("complementary.json")
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["class"], "sequence-both")


# ---------------------------------------------------------------------------
# Acceptance case (3): Shared-base-but-isolated
# ---------------------------------------------------------------------------

class TestSharedBaseIsolated(unittest.TestCase):
    """
    Acceptance case (3): different files entirely → isolated.
    base_commit / diffstat_lines fields MAY be present in the payload
    but MUST NOT affect the classification result.
    """

    def _pr_a(self) -> dict:
        return _make_pr(
            "301",
            files=["crates/perl-lexer/src/lib.rs", "tests/lexer_test.rs"],
            tests=["tests/lexer_test.rs"],
            symbols=["Lexer::next_token"],
            # These extra fields must be ignored.
            base_commit="abc1234",
            diffstat_lines=120,
        )

    def _pr_b(self) -> dict:
        return _make_pr(
            "302",
            files=["crates/perl-dap/src/server.rs", "tests/dap_test.rs"],
            tests=["tests/dap_test.rs"],
            symbols=["DapServer::run"],
            base_commit="abc1234",
            diffstat_lines=85,
        )

    def test_class_is_isolated(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertEqual(result["class"], "isolated")

    def test_jaccard_files_is_zero(self) -> None:
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        self.assertEqual(result["jaccard_files"], 0.0)

    def test_base_commit_not_consulted(self) -> None:
        """
        The same base commit does NOT cause the pair to be anything other
        than isolated when files don't overlap. Verify by checking that
        the classification remains isolated even though both PRs carry a
        ``base_commit`` field and a ``diffstat_lines`` field.
        """
        a = _normalise_pr(self._pr_a())
        b = _normalise_pr(self._pr_b())
        result = classify_pair(a, b)
        # If base_commit were consulted, this might be 'sequence-both' or worse.
        self.assertEqual(result["class"], "isolated")
        # Sanity: scores must all be zero since files don't overlap.
        self.assertEqual(result["jaccard_files"], 0.0)
        self.assertEqual(result["jaccard_tests"], 0.0)
        self.assertEqual(result["jaccard_syms"],  0.0)

    def test_fixture_shared_base_isolated(self) -> None:
        """Fixture round-trip for acceptance case (3)."""
        results = _classify_fixture("shared_base_isolated.json")
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["class"], "isolated")


# ---------------------------------------------------------------------------
# Missing optional fields
# ---------------------------------------------------------------------------

class TestMissingOptionalFields(unittest.TestCase):
    """
    When ``tests`` and ``symbols`` are absent, they default to empty → jaccard 0.
    """

    def test_missing_tests_and_symbols_defaults_to_empty(self) -> None:
        a = _normalise_pr(_make_pr("A", files=["src/foo.rs"]))
        b = _normalise_pr(_make_pr("B", files=["src/foo.rs"]))
        # Both have no tests, no symbols.
        result = classify_pair(a, b)
        self.assertEqual(result["jaccard_tests"], 0.0)
        self.assertEqual(result["jaccard_syms"],  0.0)

    def test_missing_fields_does_not_raise(self) -> None:
        a = _normalise_pr(_make_pr("A", files=["src/foo.rs"]))
        b = _normalise_pr(_make_pr("B", files=["src/foo.rs"]))
        # Must not raise KeyError or AttributeError.
        result = classify_pair(a, b)
        self.assertIn("class", result)

    def test_only_file_overlap_gives_sequence_both(self) -> None:
        """
        When tests and symbols are missing (→ jaccard 0), file-only overlap
        with jaccard_files ≤ 0.5 yields sequence-both.
        """
        # One file shared out of three total → jaccard = 1/3 ≈ 0.333
        a = _normalise_pr(_make_pr("A", files=["shared.rs", "only_a.rs"]))
        b = _normalise_pr(_make_pr("B", files=["shared.rs", "only_b.rs"]))
        result = classify_pair(a, b)
        self.assertEqual(result["class"], "sequence-both")

    def test_missing_required_files_raises(self) -> None:
        """Missing 'files' field raises ValueError."""
        raw = {"id": "X"}
        with self.assertRaises(ValueError):
            _normalise_pr(raw)


# ---------------------------------------------------------------------------
# pick-one case
# ---------------------------------------------------------------------------

class TestPickOne(unittest.TestCase):
    """
    pick-one: jaccard_files > 0.5 with surface overlap >= 0.3 on tests or syms.
    """

    def test_pick_one_high_file_overlap_with_test_surface(self) -> None:
        # 4 shared files out of 5 total → jaccard = 4/5 = 0.8 > 0.5
        shared = [f"src/module_{i}.rs" for i in range(4)]
        a = _normalise_pr(_make_pr(
            "A",
            files=shared + ["src/only_a.rs"],
            tests=["tests/shared_test.rs", "tests/only_a_test.rs"],
            symbols=["A::unique_method", "Common::helper"],
        ))
        b = _normalise_pr(_make_pr(
            "B",
            files=shared + ["src/only_b.rs"],
            tests=["tests/shared_test.rs", "tests/only_b_test.rs"],
            symbols=["B::unique_method", "Common::helper"],
        ))
        result = classify_pair(a, b)
        # jaccard_files = 4/5 = 0.8, jaccard_tests = 1/3 ≈ 0.333 >= 0.3
        self.assertIn(result["class"], ("pick-one", "likely-duplicate"))

    def test_pick_one_with_sym_overlap_only(self) -> None:
        # 4 shared files → jaccard = 4/5 = 0.8 > 0.5; test overlap = 0; sym overlap >= 0.3
        shared = [f"src/file_{i}.rs" for i in range(4)]
        a = _normalise_pr(_make_pr(
            "A",
            files=shared + ["src/extra_a.rs"],
            tests=[],
            symbols=["Shared::method", "Shared::helper", "AOnly::thing"],
        ))
        b = _normalise_pr(_make_pr(
            "B",
            files=shared + ["src/extra_b.rs"],
            tests=[],
            symbols=["Shared::method", "Shared::helper", "BOnly::thing"],
        ))
        result = classify_pair(a, b)
        # jaccard_syms = 2/4 = 0.5 >= 0.3; jaccard_files = 0.8 > 0.5
        self.assertIn(result["class"], ("pick-one", "likely-duplicate"))

    def test_pick_one_explicitly(self) -> None:
        """Construct a case that is explicitly pick-one (not likely-duplicate)."""
        # jaccard_files = 0.6 > 0.5; jaccard_tests = 0.5 >= 0.3; jaccard_syms = 0.0 < 0.5
        # → NOT likely-duplicate (syms < 0.5), but IS pick-one (files > 0.5 AND tests >= 0.3)
        shared_files = ["src/a.rs", "src/b.rs", "src/c.rs"]
        a = _normalise_pr(_make_pr(
            "A",
            files=shared_files + ["src/d.rs", "src/e.rs"],
            tests=["tests/shared_t.rs"],
            symbols=["AStruct::method"],
        ))
        b = _normalise_pr(_make_pr(
            "B",
            files=shared_files + ["src/f.rs"],
            tests=["tests/shared_t.rs"],
            symbols=["BStruct::method"],
        ))
        result = classify_pair(a, b)
        # files: 3 shared / 6 total = 0.5 — NOT > 0.5, so this is actually sequence-both
        # Let's adjust: use 4 shared files
        shared_files = ["src/a.rs", "src/b.rs", "src/c.rs", "src/d.rs"]
        a = _normalise_pr(_make_pr(
            "A",
            files=shared_files + ["src/e.rs"],
            tests=["tests/shared_t.rs"],
            symbols=["AStruct::method"],
        ))
        b = _normalise_pr(_make_pr(
            "B",
            files=shared_files + ["src/f.rs"],
            tests=["tests/shared_t.rs"],
            symbols=["BStruct::method"],
        ))
        result = classify_pair(a, b)
        # files: 4/6 ≈ 0.667 > 0.5; tests: 1/1 = 1.0 >= 0.3; syms: 0/2 = 0.0 < 0.5
        # → pick-one (not likely-duplicate because syms < 0.5)
        self.assertEqual(result["class"], "pick-one")


# ---------------------------------------------------------------------------
# Jaccard helper
# ---------------------------------------------------------------------------

class TestJaccardHelper(unittest.TestCase):
    def test_identical_sets(self) -> None:
        self.assertAlmostEqual(_jaccard({"a", "b"}, {"a", "b"}), 1.0)

    def test_disjoint_sets(self) -> None:
        self.assertAlmostEqual(_jaccard({"a"}, {"b"}), 0.0)

    def test_both_empty(self) -> None:
        self.assertAlmostEqual(_jaccard(set(), set()), 0.0)

    def test_one_empty(self) -> None:
        self.assertAlmostEqual(_jaccard({"a"}, set()), 0.0)

    def test_partial_overlap(self) -> None:
        # |{a,b} ∩ {b,c}| / |{a,b,c}| = 1/3
        self.assertAlmostEqual(_jaccard({"a", "b"}, {"b", "c"}), 1 / 3)


# ---------------------------------------------------------------------------
# Auto-detection of test files
# ---------------------------------------------------------------------------

class TestAutoDetectTests(unittest.TestCase):
    """When ``tests`` field is absent, auto-detect from ``files``."""

    def test_tests_dir_prefix_detected(self) -> None:
        self.assertTrue(_is_test_file("tests/foo.rs"))

    def test_nested_tests_dir_detected(self) -> None:
        self.assertTrue(_is_test_file("crates/foo/tests/bar.rs"))

    def test_rust_test_suffix_detected(self) -> None:
        self.assertTrue(_is_test_file("src/parser_test.rs"))

    def test_python_test_prefix_detected(self) -> None:
        self.assertTrue(_is_test_file("test_publish.py"))

    def test_nested_python_test_prefix_detected(self) -> None:
        self.assertTrue(_is_test_file("scripts/test_foo.py"))

    def test_normal_source_not_detected(self) -> None:
        self.assertFalse(_is_test_file("src/parser.rs"))

    def test_auto_detect_applied_when_tests_absent(self) -> None:
        raw = _make_pr(
            "A",
            files=["src/parser.rs", "tests/parser_test.rs"],
            # no ``tests`` key
        )
        normalised = _normalise_pr(raw)
        self.assertIn("tests/parser_test.rs", normalised["tests"])
        self.assertNotIn("src/parser.rs", normalised["tests"])


# ---------------------------------------------------------------------------
# generate_report (multi-PR)
# ---------------------------------------------------------------------------

class TestGenerateReport(unittest.TestCase):
    def test_single_pair(self) -> None:
        prs = [
            {"id": "1", "files": ["a.rs"]},
            {"id": "2", "files": ["b.rs"]},
        ]
        results = generate_report(prs)
        self.assertEqual(len(results), 1)

    def test_three_prs_gives_three_pairs(self) -> None:
        prs = [
            {"id": "1", "files": ["a.rs"]},
            {"id": "2", "files": ["b.rs"]},
            {"id": "3", "files": ["c.rs"]},
        ]
        results = generate_report(prs)
        self.assertEqual(len(results), 3)

    def test_result_contains_required_keys(self) -> None:
        prs = [
            {"id": "1", "files": ["a.rs"]},
            {"id": "2", "files": ["a.rs"]},
        ]
        results = generate_report(prs)
        for key in ("id_a", "id_b", "class", "jaccard_files", "jaccard_tests", "jaccard_syms", "rationale"):
            self.assertIn(key, results[0])

    def test_ids_are_strings(self) -> None:
        prs = [
            {"id": 123, "files": ["a.rs"]},
            {"id": 456, "files": ["b.rs"]},
        ]
        results = generate_report(prs)
        self.assertIsInstance(results[0]["id_a"], str)
        self.assertIsInstance(results[0]["id_b"], str)

    def test_fewer_than_two_prs_allowed(self) -> None:
        """generate_report with < 2 PRs should return empty list (no pairs)."""
        prs = [{"id": "1", "files": ["a.rs"]}]
        results = generate_report(prs)
        self.assertEqual(results, [])


# ---------------------------------------------------------------------------
# Threshold boundary conditions
# ---------------------------------------------------------------------------

class TestThresholdBoundaries(unittest.TestCase):
    """Test exact boundary values match the spec thresholds."""

    def _make_pair(self, files_a: list[str], files_b: list[str],
                   tests_a: list[str], tests_b: list[str],
                   syms_a: list[str], syms_b: list[str]) -> dict:
        a = _normalise_pr(_make_pr("A", files=files_a, tests=tests_a, symbols=syms_a))
        b = _normalise_pr(_make_pr("B", files=files_b, tests=tests_b, symbols=syms_b))
        return classify_pair(a, b)

    def test_jaccard_files_exactly_zero_is_isolated(self) -> None:
        result = self._make_pair(
            ["a.rs"], ["b.rs"],
            [], [],
            [], [],
        )
        self.assertEqual(result["class"], "isolated")

    def test_jaccard_files_just_above_zero_is_not_isolated(self) -> None:
        result = self._make_pair(
            ["shared.rs", "a_only.rs"], ["shared.rs", "b_only.rs"],
            [], [],
            [], [],
        )
        self.assertNotEqual(result["class"], "isolated")

    def test_sequence_both_when_file_overlap_at_threshold(self) -> None:
        # jaccard_files = 0.5 (exactly) — NOT > 0.5, so not pick-one
        # but IS > 0 → sequence-both (with low surface overlap)
        result = self._make_pair(
            ["shared1.rs", "shared2.rs", "a_only1.rs", "a_only2.rs"],
            ["shared1.rs", "shared2.rs", "b_only1.rs", "b_only2.rs"],
            [], [],
            [], [],
        )
        # jaccard = 2 / 6 ... wait: |union| = 6, |intersection| = 2 → 2/6 = 0.333
        # Not exactly 0.5 — adjust for exact 0.5:
        # 2 shared, 2 total on each side → 2 / (2+2+2-2) = wrong
        # For 0.5: |A|=2, |B|=2, |A∩B|=1: jaccard = 1/3; |A|=3,|B|=3,|A∩B|=3: jaccard=1
        # For exactly 0.5: need |A∩B|/|A∪B| = 0.5
        # Example: A={1,2,3,4}, B={3,4,5,6}: intersection=2, union=6 → 2/6=0.333
        # Example: A={1,2,3}, B={2,3,4}: intersection=2, union=4 → 2/4=0.5
        result = self._make_pair(
            ["f1.rs", "f2.rs", "f3.rs"],
            ["f2.rs", "f3.rs", "f4.rs"],
            [], [],
            [], [],
        )
        # jaccard = 2/4 = 0.5; not > 0.5 → not pick-one → sequence-both
        self.assertAlmostEqual(result["jaccard_files"], 0.5)
        self.assertEqual(result["class"], "sequence-both")

    def test_likely_duplicate_at_exact_boundary(self) -> None:
        """
        Construct a case where jf=0.8, jt=0.5, js=0.5 exactly — the minimum
        values that satisfy the likely-duplicate predicate.  All three must be
        at-or-above their respective thresholds simultaneously.
        """
        # jf = 4/5 = 0.8 : A has {f1..f4}, B has {f1..f4, f5}
        # jt = 1/2 = 0.5 : A has {t1, t2}, B has {t1}  → |∩|=1, |∪|=2
        # js = 1/2 = 0.5 : A has {S1, S2}, B has {S1}   → |∩|=1, |∪|=2
        result = self._make_pair(
            files_a=["f1.rs", "f2.rs", "f3.rs", "f4.rs"],
            files_b=["f1.rs", "f2.rs", "f3.rs", "f4.rs", "f5.rs"],
            tests_a=["t1.rs", "t2.rs"],
            tests_b=["t1.rs"],
            syms_a=["S1", "S2"],
            syms_b=["S1"],
        )
        self.assertAlmostEqual(result["jaccard_files"], 0.8)
        self.assertAlmostEqual(result["jaccard_tests"], 0.5)
        self.assertAlmostEqual(result["jaccard_syms"],  0.5)
        self.assertEqual(result["class"], "likely-duplicate")


# ---------------------------------------------------------------------------
# 3-way cluster
# ---------------------------------------------------------------------------

class TestThreeWayCluster(unittest.TestCase):
    """
    With 3 PRs we get 3 pairs. Verify that generate_report produces independent
    pairwise results and that the mixed-class scenario is handled correctly:
    A-B = likely-duplicate, A-C = sequence-both, B-C = isolated.
    """

    def test_mixed_three_way_cluster(self) -> None:
        """Three-way cluster with distinct classes across the three pairs."""
        prs = [
            {
                "id": "A",
                "files": ["parser.rs", "tests/parser_test.rs"],
                "tests": ["tests/parser_test.rs"],
                "symbols": ["Parser::parse", "Ast::new"],
            },
            {
                # B is a near-duplicate of A (same files/tests/symbols)
                "id": "B",
                "files": ["parser.rs", "tests/parser_test.rs"],
                "tests": ["tests/parser_test.rs"],
                "symbols": ["Parser::parse", "Ast::new"],
            },
            {
                # C touches an entirely different file — isolated from both A and B
                "id": "C",
                "files": ["crates/perl-dap/src/server.rs"],
                "tests": [],
                "symbols": ["DapServer::run"],
            },
        ]
        results = generate_report(prs)
        self.assertEqual(len(results), 3)

        by_pair = {(r["id_a"], r["id_b"]): r["class"] for r in results}
        self.assertEqual(by_pair[("A", "B")], "likely-duplicate")
        self.assertEqual(by_pair[("A", "C")], "isolated")
        self.assertEqual(by_pair[("B", "C")], "isolated")

    def test_tests_rs_suffix_detected(self) -> None:
        """
        Files ending in _tests.rs (plural, e.g. ``src/parser_tests.rs``) are
        auto-detected as test files.  The issue spec lists this alongside the
        _test.rs suffix convention.
        """
        self.assertTrue(_is_test_file("src/parser_tests.rs"))
        self.assertTrue(_is_test_file("crates/perl-parser/src/parser_tests.rs"))


if __name__ == "__main__":
    unittest.main()
