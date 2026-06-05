#!/usr/bin/env python3
"""
Convert CI receipt JSON files to JUnit XML format for Codecov Test Analytics.

Supports both single files and directories containing multiple receipt files.
Handles gate receipts with structured gate metadata and UX regression receipts.
"""

import argparse
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


def parse_receipt(data: Any) -> Optional[List[Dict[str, Any]]]:
    """
    Parse receipt data and extract testcases.
    Returns a list of testcase dicts or None if not a valid receipt.
    """
    if not isinstance(data, dict):
        return None

    testcases = []

    # Handle gate receipts with a 'gates' array
    if "gates" in data and isinstance(data["gates"], list):
        for gate in data["gates"]:
            if isinstance(gate, dict):
                testcases.append(gate)
        return testcases if testcases else None

    # Handle UX regression receipt
    if "result" in data or "failure_class" in data:
        return [data]

    return None


def gate_to_testcase(
    gate: Dict[str, Any], classname: str
) -> Tuple[ET.Element, bool]:
    """
    Convert a gate receipt to a JUnit testcase element.
    Returns (element, is_passed).
    """
    name = gate.get("gate_name") or gate.get("name") or "unnamed"
    duration_ms = gate.get("duration_ms", 0)
    duration_sec = duration_ms / 1000.0
    status = str(gate.get("status", "unknown")).lower()

    testcase = ET.Element("testcase")
    testcase.set("classname", classname)
    testcase.set("name", name)
    testcase.set("time", str(duration_sec))

    # Determine pass/skip/fail
    is_passed = False
    if status in ("pass", "passed"):
        is_passed = True
    elif status in ("skip", "skipped"):
        skipped = ET.SubElement(testcase, "skipped")
        skipped.set("message", "Skipped")
    else:
        # Treat as failure or error
        is_error = status in ("timeout", "error")
        tag = "error" if is_error else "failure"
        failure = ET.SubElement(testcase, tag)
        failure.set("type", status)

        # Build diagnostic message
        diag_lines = []
        diag_lines.append(f"Gate: {name}")
        diag_lines.append(f"Status: {status}")
        if "command" in gate:
            diag_lines.append(f"Command: {gate['command']}")
        if "exit_code" in gate:
            diag_lines.append(f"Exit code: {gate['exit_code']}")
        failure.text = "\n".join(diag_lines)

    return testcase, is_passed


def ux_to_testcase(receipt: Dict[str, Any], classname: str) -> Tuple[ET.Element, bool]:
    """
    Convert a UX regression receipt to a JUnit testcase element.
    Returns (element, is_passed).
    """
    testcase = ET.Element("testcase")
    testcase.set("classname", classname)
    testcase.set("name", "ux-regression")
    testcase.set("time", "0.0")

    result = receipt.get("result", "").lower()
    is_passed = result in ("pass", "passed", "success", "ok")
    is_skipped = result in ("skip", "skipped")

    if is_skipped:
        skipped = ET.SubElement(testcase, "skipped")
        skipped.set("message", "Skipped")
    elif not is_passed:
        failure = ET.SubElement(testcase, "failure")
        failure.set("type", receipt.get("failure_class", "unknown"))

        # Build diagnostic message
        diag_lines = []
        if receipt.get("failure_class"):
            diag_lines.append(f"Failure class: {receipt['failure_class']}")
        if receipt.get("first_failing_test"):
            diag_lines.append(f"First failing test: {receipt['first_failing_test']}")
        if receipt.get("panic_location"):
            diag_lines.append(f"Panic location: {receipt['panic_location']}")
        if receipt.get("canonical_repro"):
            diag_lines.append(f"Canonical repro: {receipt['canonical_repro']}")
        if receipt.get("route"):
            diag_lines.append(f"Route: {receipt['route']}")
        failure.text = "\n".join(diag_lines)

    return testcase, is_passed


def receipts_to_junit(
    input_path: Path, suite_name: str
) -> Tuple[ET.Element, int, int, int, int]:
    """
    Convert receipt file(s) to JUnit XML structure.
    Returns (root_element, total_tests, failures, errors, skipped).
    """
    root = ET.Element("testsuites")

    if not input_path.exists():
        # No files found, emit a skipped testcase
        testsuite = ET.SubElement(root, "testsuite")
        testsuite.set("name", suite_name)
        testsuite.set("tests", "1")
        testsuite.set("failures", "0")
        testsuite.set("skipped", "1")
        testsuite.set("errors", "0")
        testsuite.set("time", "0.0")

        testcase = ET.SubElement(testsuite, "testcase")
        testcase.set("classname", suite_name)
        testcase.set("name", "no-files-found")
        testcase.set("time", "0.0")
        skipped = ET.SubElement(testcase, "skipped")
        skipped.set("message", "No receipt files found")

        return root, 1, 0, 0, 1

    # Determine if input is file or directory
    json_files = []
    if input_path.is_file():
        json_files = [input_path]
    elif input_path.is_dir():
        json_files = sorted(input_path.glob("*.json"))

    if not json_files:
        # No files found, emit a skipped testcase
        testsuite = ET.SubElement(root, "testsuite")
        testsuite.set("name", suite_name)
        testsuite.set("tests", "1")
        testsuite.set("failures", "0")
        testsuite.set("skipped", "1")
        testsuite.set("errors", "0")
        testsuite.set("time", "0.0")

        testcase = ET.SubElement(testsuite, "testcase")
        testcase.set("classname", suite_name)
        testcase.set("name", "no-files-found")
        testcase.set("time", "0.0")
        skipped = ET.SubElement(testcase, "skipped")
        skipped.set("message", "No JSON files found")

        return root, 1, 0, 0, 1

    all_testcases = []
    total_time = 0.0

    for json_file in json_files:
        try:
            with open(json_file, "r", encoding="utf-8") as f:
                data = json.load(f)

            testcases = parse_receipt(data)
            if testcases is None:
                # Not a recognized receipt format, emit an error testcase
                testcase = ET.Element("testcase")
                testcase.set("classname", suite_name)
                testcase.set("name", f"parse-{json_file.name}")
                testcase.set("time", "0.0")
                error = ET.SubElement(testcase, "error")
                error.set("type", "UnrecognizedFormat")
                error.text = f"Receipt at {json_file} does not match known format"
                all_testcases.append(testcase)
                continue

            # Convert each testcase
            for tc_data in testcases:
                # Detect if this is a UX receipt or gate receipt
                if "result" in tc_data or "failure_class" in tc_data:
                    testcase, is_passed = ux_to_testcase(tc_data, suite_name)
                else:
                    testcase, is_passed = gate_to_testcase(tc_data, suite_name)

                duration_sec = float(testcase.get("time", 0))
                total_time += duration_sec

                all_testcases.append(testcase)

        except json.JSONDecodeError as e:
            # JSON parse error, emit an error testcase
            testcase = ET.Element("testcase")
            testcase.set("classname", suite_name)
            testcase.set("name", f"parse-{json_file.name}")
            testcase.set("time", "0.0")
            error = ET.SubElement(testcase, "error")
            error.set("type", "JSONDecodeError")
            error.text = f"Failed to parse {json_file}: {e}"
            all_testcases.append(testcase)

        except Exception as e:
            # General error, emit an error testcase
            testcase = ET.Element("testcase")
            testcase.set("classname", suite_name)
            testcase.set("name", f"parse-{json_file.name}")
            testcase.set("time", "0.0")
            error = ET.SubElement(testcase, "error")
            error.set("type", type(e).__name__)
            error.text = f"Error processing {json_file}: {e}"
            all_testcases.append(testcase)

    # Build testsuite
    testsuite = ET.SubElement(root, "testsuite")
    testsuite.set("name", suite_name)
    total_tests = len(all_testcases)
    total_failures = sum(1 for tc in all_testcases if tc.find("failure") is not None)
    total_errors = sum(1 for tc in all_testcases if tc.find("error") is not None)
    total_skipped = sum(1 for tc in all_testcases if tc.find("skipped") is not None)

    testsuite.set("tests", str(total_tests))
    testsuite.set("failures", str(total_failures))
    testsuite.set("errors", str(total_errors))
    testsuite.set("skipped", str(total_skipped))
    testsuite.set("time", str(total_time))

    for testcase in all_testcases:
        testsuite.append(testcase)

    return root, total_tests, total_failures, total_errors, total_skipped


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert CI receipt JSON files to JUnit XML for Codecov Test Analytics.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python3 receipts-to-junit.py --input receipt.json --output junit.xml --suite-name pr-fast
  python3 receipts-to-junit.py --input target/receipts/shards --output junit.xml --suite-name gate-meta
        """,
    )

    parser.add_argument(
        "--input",
        required=True,
        type=Path,
        help="Input: single JSON file or directory containing *.json receipt files",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Output JUnit XML file path",
    )
    parser.add_argument(
        "--suite-name",
        required=True,
        help="Test suite name (e.g., 'pr-fast', 'gate-meta', 'ux-regression')",
    )

    args = parser.parse_args()

    # Create output directory if needed
    args.output.parent.mkdir(parents=True, exist_ok=True)

    # Convert receipts to JUnit
    # Convert receipts to JUnit
    root, total_tests, total_failures, total_errors, total_skipped = receipts_to_junit(
        args.input, args.suite_name
    )

    # Write XML
    tree = ET.ElementTree(root)
    tree.write(args.output, encoding="utf-8", xml_declaration=True)

    print(f"Generated JUnit XML: {args.output}")
    print(f"  Suite: {args.suite_name}")
    print(f"  Tests: {total_tests}, Failures: {total_failures}, Errors: {total_errors}, Skipped: {total_skipped}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
