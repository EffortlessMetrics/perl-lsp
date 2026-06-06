#!/usr/bin/env python3

"""Tests for the debt-report compatibility shim."""

from pathlib import Path
import runpy
import sys
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
DEBT_REPORT = REPO_ROOT / "scripts" / "debt-report.py"


class DebtReportShimTest(unittest.TestCase):
    def test_delegates_to_cargo_xtask_from_repo_root(self) -> None:
        exit_code, calls = self._run_wrapper(["--help", "--format", "json"], return_code=0)

        self.assertEqual(exit_code, 0)
        self.assertEqual(len(calls), 1)
        self.assertEqual(
            calls[0]["args"],
            [
                "cargo",
                "xtask",
                "debt-report",
                "--help",
                "--format",
                "json",
            ],
        )
        self.assertEqual(calls[0]["cwd"], REPO_ROOT)

    def test_propagates_cargo_exit_code(self) -> None:
        exit_code, calls = self._run_wrapper(["--summary"], return_code=37)

        self.assertEqual(exit_code, 37)
        self.assertEqual(calls[0]["args"], ["cargo", "xtask", "debt-report", "--summary"])

    def _run_wrapper(self, argv: list[str], return_code: int) -> tuple[int, list[dict[str, object]]]:
        calls: list[dict[str, object]] = []

        def fake_call(args: list[str], *, cwd: Path) -> int:
            calls.append({"args": args, "cwd": cwd})
            return return_code

        with mock.patch.object(sys, "argv", [str(DEBT_REPORT), *argv]):
            with mock.patch("subprocess.call", side_effect=fake_call):
                with self.assertRaises(SystemExit) as raised:
                    runpy.run_path(str(DEBT_REPORT), run_name="__main__")

        return int(raised.exception.code), calls


if __name__ == "__main__":
    unittest.main()
