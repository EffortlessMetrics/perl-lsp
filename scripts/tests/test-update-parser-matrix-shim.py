#!/usr/bin/env python3

"""Tests for the update-parser-matrix compatibility shim."""

from pathlib import Path
import runpy
import sys
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
UPDATE_PARSER_MATRIX = REPO_ROOT / "scripts" / "update-parser-matrix.py"


class UpdateParserMatrixShimTest(unittest.TestCase):
    def test_delegates_to_cargo_xtask_parser_matrix_from_repo_root(self) -> None:
        exit_code, calls = self._run_wrapper(
            ["--report", "target/parser-audit.json", "--output", "docs/reference/PARSER_FEATURE_MATRIX.md"],
            return_code=0,
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(len(calls), 1)
        self.assertEqual(
            calls[0]["args"],
            [
                "cargo",
                "xtask",
                "parser-matrix",
                "--report",
                "target/parser-audit.json",
                "--output",
                "docs/reference/PARSER_FEATURE_MATRIX.md",
            ],
        )
        self.assertEqual(calls[0]["cwd"], REPO_ROOT)

    def test_propagates_cargo_exit_code(self) -> None:
        exit_code, calls = self._run_wrapper(["--output", "matrix.md"], return_code=37)

        self.assertEqual(exit_code, 37)
        self.assertEqual(
            calls[0]["args"],
            ["cargo", "xtask", "parser-matrix", "--output", "matrix.md"],
        )

    def _run_wrapper(self, argv: list[str], return_code: int) -> tuple[int, list[dict[str, object]]]:
        calls: list[dict[str, object]] = []

        def fake_call(args: list[str], *, cwd: Path) -> int:
            calls.append({"args": args, "cwd": cwd})
            return return_code

        with mock.patch.object(sys, "argv", [str(UPDATE_PARSER_MATRIX), *argv]):
            with mock.patch("subprocess.call", side_effect=fake_call):
                with self.assertRaises(SystemExit) as raised:
                    runpy.run_path(str(UPDATE_PARSER_MATRIX), run_name="__main__")

        return int(raised.exception.code), calls


if __name__ == "__main__":
    unittest.main()
