#!/usr/bin/env python3
"""Focused tests for scripts/ci/receipts-to-junit.py."""

from __future__ import annotations

import importlib.util
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
import xml.etree.ElementTree as ET
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("receipts-to-junit.py")
SPEC = importlib.util.spec_from_file_location("receipts_to_junit", SCRIPT_PATH)
assert SPEC is not None
receipts_to_junit = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(receipts_to_junit)


class ReceiptsToJunitTests(unittest.TestCase):
    def test_gate_receipt_converts_pass_fail_skip_and_error_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            receipt = Path(tmp) / "gate.json"
            receipt.write_text(
                json.dumps(
                    {
                        "gates": [
                            {
                                "gate_name": "fmt",
                                "status": "pass",
                                "duration_ms": 1250,
                            },
                            {
                                "gate_name": "coverage",
                                "status": "fail",
                                "command": "rtk cargo xtask quality-gate",
                                "exit_code": 1,
                            },
                            {"gate_name": "docs", "status": "skipped"},
                            {"gate_name": "ripr", "status": "timeout"},
                        ]
                    }
                ),
                encoding="utf-8",
            )

            root, total, failures, errors, skipped = receipts_to_junit.receipts_to_junit(
                receipt,
                "proof-gates",
            )

        self.assertEqual((4, 1, 1, 1), (total, failures, errors, skipped))
        suite = root.find("testsuite")
        self.assertIsNotNone(suite)
        assert suite is not None
        self.assertEqual("proof-gates", suite.get("name"))
        fmt = suite.find("./testcase[@name='fmt']")
        coverage = suite.find("./testcase[@name='coverage']")
        docs = suite.find("./testcase[@name='docs']")
        ripr = suite.find("./testcase[@name='ripr']")
        self.assertIsNotNone(fmt)
        assert fmt is not None
        self.assertEqual("1.25", fmt.get("time"))
        self.assertIsNotNone(coverage)
        assert coverage is not None
        failure = coverage.find("failure")
        self.assertIsNotNone(failure)
        assert failure is not None
        self.assertEqual("fail", failure.get("type"))
        self.assertIn("Command: rtk cargo xtask quality-gate", failure.text or "")
        self.assertIn("Exit code: 1", failure.text or "")
        self.assertIsNotNone(docs)
        assert docs is not None
        self.assertIsNotNone(docs.find("skipped"))
        self.assertIsNotNone(ripr)
        assert ripr is not None
        self.assertIsNotNone(ripr.find("error"))

    def test_ux_receipt_failure_includes_repair_context(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            receipt = Path(tmp) / "ux.json"
            receipt.write_text(
                json.dumps(
                    {
                        "result": "fail",
                        "failure_class": "assertion",
                        "first_failing_test": "inline_completion_context",
                        "panic_location": "crates/perl-lsp-ux-tests/tests/foo.rs:42",
                        "canonical_repro": "rtk cargo test -p perl-lsp-ux-tests",
                        "route": "ux-scenario",
                    }
                ),
                encoding="utf-8",
            )

            root, total, failures, errors, skipped = receipts_to_junit.receipts_to_junit(
                receipt,
                "ux-regression",
            )

        self.assertEqual((1, 1, 0, 0), (total, failures, errors, skipped))
        failure = root.find("./testsuite/testcase/failure")
        self.assertIsNotNone(failure)
        assert failure is not None
        self.assertEqual("assertion", failure.get("type"))
        self.assertIn("First failing test: inline_completion_context", failure.text or "")
        self.assertIn("Canonical repro: rtk cargo test -p perl-lsp-ux-tests", failure.text or "")
        self.assertIn("Route: ux-scenario", failure.text or "")

    def test_missing_input_emits_skipped_testcase(self) -> None:
        root, total, failures, errors, skipped = receipts_to_junit.receipts_to_junit(
            Path("missing-receipt-dir"),
            "missing-suite",
        )

        self.assertEqual((1, 0, 0, 1), (total, failures, errors, skipped))
        skipped_node = root.find("./testsuite/testcase/skipped")
        self.assertIsNotNone(skipped_node)
        assert skipped_node is not None
        self.assertEqual("No receipt files found", skipped_node.get("message"))

    def test_unrecognized_and_invalid_json_files_emit_parse_errors(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "invalid.json").write_text("{", encoding="utf-8")
            (directory / "unknown.json").write_text("[]", encoding="utf-8")

            root, total, failures, errors, skipped = receipts_to_junit.receipts_to_junit(
                directory,
                "parse-suite",
            )

        self.assertEqual((2, 0, 2, 0), (total, failures, errors, skipped))
        testcase_names = {
            testcase.get("name") for testcase in root.findall("./testsuite/testcase")
        }
        self.assertEqual({"parse-invalid.json", "parse-unknown.json"}, testcase_names)
        error_types = {
            error.get("type") for error in root.findall("./testsuite/testcase/error")
        }
        self.assertEqual({"JSONDecodeError", "UnrecognizedFormat"}, error_types)

    def test_main_writes_junit_xml_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root_dir = Path(tmp)
            receipt = root_dir / "receipt.json"
            output = root_dir / "out" / "junit.xml"
            receipt.write_text(
                json.dumps({"gates": [{"gate_name": "fmt", "status": "passed"}]}),
                encoding="utf-8",
            )

            old_argv = receipts_to_junit.sys.argv
            try:
                receipts_to_junit.sys.argv = [
                    "receipts-to-junit.py",
                    "--input",
                    str(receipt),
                    "--output",
                    str(output),
                    "--suite-name",
                    "ci-gates",
                ]
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    status = receipts_to_junit.main()
            finally:
                receipts_to_junit.sys.argv = old_argv

            tree = ET.parse(output)

        self.assertEqual(0, status)
        self.assertEqual("ci-gates", tree.getroot().find("testsuite").get("name"))

if __name__ == "__main__":
    unittest.main()
