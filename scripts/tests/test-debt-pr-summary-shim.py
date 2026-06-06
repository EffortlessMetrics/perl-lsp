#!/usr/bin/env python3

"""Tests for the debt-pr-summary compatibility shim."""

from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
import runpy
import sys
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
DEBT_PR_SUMMARY = REPO_ROOT / "scripts" / "debt-pr-summary.py"


class DebtPrSummaryShimTest(unittest.TestCase):
    def test_formats_summary_json_from_stdin(self) -> None:
        payload = """
        {
          "summary": {
            "quarantined_tests": {
              "count": 2,
              "budget": 3,
              "status": "ok",
              "expired": 1
            },
            "known_issues": {"count": 4, "budget": 5, "status": "warn"},
            "technical_debt": {"count": 6, "budget": 7, "status": "fail"}
          }
        }
        """

        exit_code, stdout, stderr, calls = self._run_wrapper(payload, return_code=0)

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls, [])
        self.assertEqual(stderr, "")
        self.assertIn("| Category | Count | Budget | Status |", stdout)
        self.assertIn("| Quarantined Tests | 2 | 3 | ok |", stdout)
        self.assertIn("| Known Issues | 4 | 5 | warn |", stdout)
        self.assertIn("| Technical Debt | 6 | 7 | fail |", stdout)
        self.assertIn("**Warning:** 1 expired quarantine(s) need attention!", stdout)

    def test_invalid_json_from_stdin_fails_with_error(self) -> None:
        exit_code, stdout, stderr, calls = self._run_wrapper("{not json", return_code=0)

        self.assertEqual(exit_code, 1)
        self.assertEqual(stdout, "")
        self.assertEqual(calls, [])
        self.assertIn("Error: Invalid JSON input:", stderr)

    def test_delegates_to_cargo_xtask_summary_when_stdin_is_empty(self) -> None:
        exit_code, stdout, stderr, calls = self._run_wrapper("", return_code=37)

        self.assertEqual(exit_code, 37)
        self.assertEqual(stdout, "")
        self.assertEqual(stderr, "")
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]["args"], ["cargo", "xtask", "debt-report", "--summary"])
        self.assertEqual(calls[0]["cwd"], REPO_ROOT)

    def _run_wrapper(
        self,
        stdin: str,
        return_code: int,
    ) -> tuple[int, str, str, list[dict[str, object]]]:
        calls: list[dict[str, object]] = []

        def fake_call(args: list[str], *, cwd: Path) -> int:
            calls.append({"args": args, "cwd": cwd})
            return return_code

        stdout = StringIO()
        stderr = StringIO()

        with mock.patch.object(sys, "argv", [str(DEBT_PR_SUMMARY)]):
            with mock.patch.object(sys, "stdin", StringIO(stdin)):
                with mock.patch("subprocess.call", side_effect=fake_call):
                    with redirect_stdout(stdout), redirect_stderr(stderr):
                        with self.assertRaises(SystemExit) as raised:
                            runpy.run_path(str(DEBT_PR_SUMMARY), run_name="__main__")

        return int(raised.exception.code), stdout.getvalue(), stderr.getvalue(), calls


if __name__ == "__main__":
    unittest.main()
