#!/usr/bin/env python3
"""Fixture tests for extract-criterion.py's Criterion output parsing.

Run directly (no third-party deps -- stdlib unittest only):

    python3 benchmarks/scripts/test_extract_criterion.py

These tests exist because extract-criterion.py previously mis-parsed every
*direct* (ungrouped) `c.bench_function(name, ...)` benchmark as
group=<name>, bench_name="new", silently collapsing distinct benchmarks
together, and never distinguished a stale prior-run "base" estimate from a
fresh "new" one (#3979). Both on-disk layouts, and both failure fixtures
(stale-only output, malformed estimate), are covered here so a regression
in the parser is caught before it ships another vacuous-green benchmark job.
"""

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_MODULE_PATH = Path(__file__).parent / "extract-criterion.py"
_spec = importlib.util.spec_from_file_location("extract_criterion", _MODULE_PATH)
if _spec is None or _spec.loader is None:
    raise ImportError(f"could not load module spec from {_MODULE_PATH}")
extract_criterion = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(extract_criterion)


def _write_estimate(criterion_root: Path, parts: tuple, mean_ns: int = 1000) -> None:
    """Write a synthetic Criterion estimates.json at <criterion_root>/<parts>/estimates.json."""
    target_dir = criterion_root.joinpath(*parts)
    target_dir.mkdir(parents=True, exist_ok=True)
    (target_dir / "estimates.json").write_text(json.dumps({
        "mean": {
            "point_estimate": mean_ns,
            "confidence_interval": {"lower_bound": mean_ns - 10, "upper_bound": mean_ns + 10},
        }
    }))


class ParseEstimatesPathTests(unittest.TestCase):
    def test_direct_ungrouped_benchmark_is_two_level(self) -> None:
        # target/criterion/<name>/new/estimates.json -- a direct
        # `c.bench_function("incremental update single file", ...)` call.
        parsed = extract_criterion.parse_estimates_path(
            ("incremental update single file", "new", "estimates.json")
        )
        self.assertEqual(parsed, ("other", "incremental update single file"))

    def test_grouped_benchmark_is_three_level(self) -> None:
        # target/criterion/<group>/<name>/new/estimates.json -- a
        # `group.bench_function("rope_insertion", ...)` call.
        parsed = extract_criterion.parse_estimates_path(
            ("document_insertions", "rope_insertion", "new", "estimates.json")
        )
        self.assertEqual(parsed, ("document_insertions", "rope_insertion"))

    def test_parameterized_grouped_benchmark_is_four_level(self) -> None:
        # target/criterion/<group>/<function>/<param>/new/estimates.json --
        # a `group.bench_with_input(BenchmarkId::new(function, param), ...)`
        # call. The join-based reconstruction in parse_estimates_path must
        # fold the extra parameter component back into the bench name
        # (group, "function/param"), not silently drop or mis-group it.
        parsed = extract_criterion.parse_estimates_path(
            ("document_insertions", "rope_insertion", "10000", "new", "estimates.json")
        )
        self.assertEqual(parsed, ("document_insertions", "rope_insertion/10000"))

    def test_direct_benchmark_name_containing_slash_does_not_nest(self) -> None:
        # `c.bench_function("cpan/moose_oo_class", ...)` is a DIRECT call
        # (no `benchmark_group`) whose id string happens to contain a
        # literal "/". Verified against a real run (29186609858): Criterion
        # sanitizes that "/" to "_" and writes a single flat directory,
        # `target/criterion/cpan_moose_oo_class/new/estimates.json` -- it
        # does NOT nest like an explicit group would. An earlier version of
        # this test asserted the opposite (that it parses identically to an
        # explicit group) and was wrong; --expect-id in ci-nightly.yml made
        # the same mistake and had to be corrected to the flattened name
        # (#3979).
        parsed = extract_criterion.parse_estimates_path(
            ("cpan_moose_oo_class", "new", "estimates.json")
        )
        self.assertEqual(parsed, ("other", "cpan_moose_oo_class"))

    def test_stale_base_directory_is_rejected(self) -> None:
        # Criterion keeps a "base" (previous run) directory alongside "new"
        # once a benchmark has executed twice. A "base"-only estimate must
        # never be mistaken for evidence that *this* run executed.
        parsed = extract_criterion.parse_estimates_path(
            ("incremental update single file", "base", "estimates.json")
        )
        self.assertIsNone(parsed)

    def test_too_short_path_is_rejected(self) -> None:
        self.assertIsNone(extract_criterion.parse_estimates_path(("estimates.json",)))


class FindCriterionResultsTests(unittest.TestCase):
    def test_empty_criterion_dir_yields_zero_benchmarks(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            (base / "target" / "criterion").mkdir(parents=True)
            results, ids = extract_criterion.find_criterion_results(base)
            self.assertEqual(results, {})
            self.assertEqual(ids, set())

    def test_missing_criterion_dir_yields_zero_benchmarks(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            results, ids = extract_criterion.find_criterion_results(Path(tmp))
            self.assertEqual(results, {})
            self.assertEqual(ids, set())

    def test_direct_and_grouped_layouts_both_parse(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            _write_estimate(
                criterion_root, ("incremental update single file", "new"), mean_ns=209_000
            )
            _write_estimate(
                criterion_root, ("document_insertions", "rope_insertion", "new"), mean_ns=5_000
            )

            results, ids = extract_criterion.find_criterion_results(base)

            extracted_names = {
                name for cat in results.values() for name in cat if not name.startswith("_")
            }
            self.assertIn("incremental update single file", extracted_names)
            self.assertIn("rope_insertion", extracted_names)
            self.assertIn("incremental update single file", ids)
            self.assertIn("document_insertions/rope_insertion", ids)
            self.assertEqual(len(ids), 2)

    def test_stale_only_output_does_not_satisfy_extraction(self) -> None:
        # A benchmark directory with ONLY a "base" (prior-run) estimate and
        # no "new" must extract to zero benchmarks -- a stale artifact left
        # over from a previous CI attempt must not fake a fresh run.
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            _write_estimate(criterion_root, ("incremental update single file", "base"))

            results, ids = extract_criterion.find_criterion_results(base)
            self.assertEqual(results, {})
            self.assertEqual(ids, set())

    def test_malformed_estimate_is_skipped_not_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            bad_dir = criterion_root / "broken_benchmark" / "new"
            bad_dir.mkdir(parents=True)
            (bad_dir / "estimates.json").write_text("{not valid json")

            results, ids = extract_criterion.find_criterion_results(base)
            self.assertEqual(results, {})
            self.assertEqual(ids, set())


class BenchmarkIdTests(unittest.TestCase):
    def test_ungrouped_id_is_bare_name(self) -> None:
        self.assertEqual(
            extract_criterion.benchmark_id("other", "incremental update single file"),
            "incremental update single file",
        )

    def test_grouped_id_is_group_slash_name(self) -> None:
        self.assertEqual(
            extract_criterion.benchmark_id("document_insertions", "rope_insertion"),
            "document_insertions/rope_insertion",
        )


class CliFailClosedTests(unittest.TestCase):
    """Runs the real script as a subprocess so these tests exercise the exact
    exit-code contract CI depends on (--strict), not just the library
    functions above."""

    def _run(self, base: Path, output: Path, extra_args: "list[str]") -> subprocess.CompletedProcess:
        return subprocess.run(
            [
                sys.executable,
                str(_MODULE_PATH),
                "--base-path", str(base),
                "--output", str(output),
                *extra_args,
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_zero_output_fails_closed_under_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            (base / "target" / "criterion").mkdir(parents=True)
            output = base / "latest.json"

            non_strict = self._run(base, output, [])
            self.assertEqual(non_strict.returncode, 0)

            strict = self._run(base, output, ["--strict"])
            self.assertNotEqual(strict.returncode, 0)
            self.assertIn("vacuous pass", strict.stderr)

    def test_stale_output_fails_closed_under_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            _write_estimate(criterion_root, ("incremental update single file", "base"))
            output = base / "latest.json"

            strict = self._run(base, output, ["--strict"])
            self.assertNotEqual(strict.returncode, 0)

    def test_malformed_estimate_fails_closed_under_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            bad_dir = criterion_root / "broken_benchmark" / "new"
            bad_dir.mkdir(parents=True)
            (bad_dir / "estimates.json").write_text("{not valid json")
            output = base / "latest.json"

            strict = self._run(base, output, ["--strict"])
            self.assertNotEqual(strict.returncode, 0)

    def test_missing_expected_id_fails_closed_under_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            # A real benchmark DID run, so total > 0 -- but it isn't the one
            # we require, so this must still fail closed.
            _write_estimate(criterion_root, ("some_other_benchmark", "new"))
            output = base / "latest.json"

            strict = self._run(
                base, output,
                ["--strict", "--expect-id", "incremental update single file"],
            )
            self.assertNotEqual(strict.returncode, 0)
            self.assertIn("Missing expected benchmark IDs", strict.stdout + strict.stderr)

    def test_present_expected_id_passes_under_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            criterion_root = base / "target" / "criterion"
            _write_estimate(criterion_root, ("incremental update single file", "new"))
            output = base / "latest.json"

            strict = self._run(
                base, output,
                ["--strict", "--expect-id", "incremental update single file"],
            )
            self.assertEqual(strict.returncode, 0, strict.stderr)


if __name__ == "__main__":
    unittest.main()
