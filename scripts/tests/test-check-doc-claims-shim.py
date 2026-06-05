#!/usr/bin/env python3

"""Tests for the check-doc-claims compatibility shim."""

from pathlib import Path
import runpy
import sys
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECK_DOC_CLAIMS = REPO_ROOT / "scripts" / "check-doc-claims.py"


class CheckDocClaimsShimTest(unittest.TestCase):
    def test_delegates_to_cargo_xtask_from_repo_root(self) -> None:
        exit_code, calls = self._run_wrapper(["--help", "--fixture", "demo"], return_code=0)

        self.assertEqual(exit_code, 0)
        self.assertEqual(len(calls), 1)
        self.assertEqual(
            calls[0]["args"],
            [
                "cargo",
                "xtask",
                "doc-claims",
                "--help",
                "--fixture",
                "demo",
            ],
        )
        self.assertEqual(calls[0]["cwd"], REPO_ROOT)

    def test_propagates_cargo_exit_code(self) -> None:
        exit_code, calls = self._run_wrapper(["--check"], return_code=37)

        self.assertEqual(exit_code, 37)
        self.assertEqual(calls[0]["args"], ["cargo", "xtask", "doc-claims", "--check"])

    def _run_wrapper(self, argv: list[str], return_code: int) -> tuple[int, list[dict[str, object]]]:
        calls: list[dict[str, object]] = []

        def fake_call(args: list[str], *, cwd: Path) -> int:
            calls.append({"args": args, "cwd": cwd})
            return return_code

        with mock.patch.object(sys, "argv", [str(CHECK_DOC_CLAIMS), *argv]):
            with mock.patch("subprocess.call", side_effect=fake_call):
                with self.assertRaises(SystemExit) as raised:
                    runpy.run_path(str(CHECK_DOC_CLAIMS), run_name="__main__")

        return int(raised.exception.code), calls


if __name__ == "__main__":
    unittest.main()
