#!/usr/bin/env python3
"""Validate trust-lane policy metadata in policy/trust-lanes.toml.

Checks:
  - The policy header matches the advisory trust-lane contract.
  - The class set matches PLSP-SPEC-0011.
  - Every class defines rank, boundaries, checks, widening triggers, receipt
    paths, and support-claim impact.
  - Receipt paths resolve to existing repo files or directories.

Reports issues; --strict fails on any.
"""
from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path
from typing import Any


EXPECTED_CLASSES = {
    "parser_fixture_only",
    "parser_runtime_fix",
    "provider_receipt",
    "provider_live_cutover",
    "support_claim_change",
    "subprocess_seam",
    "real_workspace_receipt",
    "release_proof",
    "dependency_update",
    "docs_status_only",
}

REQUIRED_LIST_FIELDS = {
    "required_checks",
    "optional_checks",
    "skipped_by_policy_checks",
    "widening_triggers",
    "receipt_paths",
}

NON_EMPTY_LIST_FIELDS = {
    "required_checks",
    "widening_triggers",
    "receipt_paths",
}

REQUIRED_STRING_FIELDS = {
    "claim_boundary",
    "support_claim_impact",
}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as f:
        return tomllib.load(f)


def is_non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def validate_string_list(
    issues: list[str],
    class_id: str,
    field: str,
    value: object,
) -> None:
    if not isinstance(value, list):
        issues.append(f"{class_id}.{field} must be a list")
        return
    if field in NON_EMPTY_LIST_FIELDS and not value:
        issues.append(f"{class_id}.{field} must not be empty")
    for index, item in enumerate(value):
        if not is_non_empty_string(item):
            issues.append(f"{class_id}.{field}[{index}] must be a non-empty string")


def validate_receipt_paths(
    issues: list[str],
    root: Path,
    class_id: str,
    receipt_paths: object,
) -> None:
    if not isinstance(receipt_paths, list):
        return
    for index, item in enumerate(receipt_paths):
        if not isinstance(item, str):
            continue
        path = root / item
        if not path.exists():
            issues.append(
                f"{class_id}.receipt_paths[{index}] does not resolve: {item}"
            )


def validate_class(
    issues: list[str],
    root: Path,
    class_id: str,
    class_doc: object,
) -> int | None:
    if not isinstance(class_doc, dict):
        issues.append(f"class.{class_id} must be a table")
        return None

    rank = class_doc.get("risk_rank")
    if not isinstance(rank, int):
        issues.append(f"{class_id}.risk_rank must be an integer")
    elif rank <= 0 or rank > 100:
        issues.append(f"{class_id}.risk_rank must be in 1..=100")

    for field in REQUIRED_STRING_FIELDS:
        if not is_non_empty_string(class_doc.get(field)):
            issues.append(f"{class_id}.{field} must be a non-empty string")

    for field in REQUIRED_LIST_FIELDS:
        validate_string_list(issues, class_id, field, class_doc.get(field))

    validate_receipt_paths(issues, root, class_id, class_doc.get("receipt_paths"))
    return rank if isinstance(rank, int) else None


def validate_policy(policy_path: Path) -> list[str]:
    root = repo_root()
    doc = load_toml(policy_path)
    issues: list[str] = []

    if doc.get("schema_version") != 1:
        issues.append("schema_version must be 1")
    if doc.get("policy") != "trust-lanes":
        issues.append('policy must be "trust-lanes"')
    if doc.get("status") != "advisory":
        issues.append('status must be "advisory"')

    for field in ["owner", "updated", "classification_rule", "enforcement_boundary"]:
        if not is_non_empty_string(doc.get(field)):
            issues.append(f"{field} must be a non-empty string")

    spec = doc.get("spec")
    if not is_non_empty_string(spec):
        issues.append("spec must be a non-empty string")
    elif not (root / spec).exists():
        issues.append(f"spec path does not resolve: {spec}")

    classes = doc.get("class")
    if not isinstance(classes, dict):
        issues.append("class must be a table")
        return issues

    actual_classes = set(classes)
    missing = sorted(EXPECTED_CLASSES - actual_classes)
    extra = sorted(actual_classes - EXPECTED_CLASSES)
    for class_id in missing:
        issues.append(f"missing trust-lane class: {class_id}")
    for class_id in extra:
        issues.append(f"unknown trust-lane class: {class_id}")

    ranks: dict[int, str] = {}
    for class_id in sorted(actual_classes & EXPECTED_CLASSES):
        rank = validate_class(issues, root, class_id, classes[class_id])
        if rank is None:
            continue
        if rank in ranks:
            issues.append(
                f"{class_id}.risk_rank duplicates {ranks[rank]}.risk_rank ({rank})"
            )
        ranks[rank] = class_id

    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--trust-lanes",
        type=Path,
        default=Path("policy/trust-lanes.toml"),
    )
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    issues = validate_policy(args.trust_lanes)
    class_count = len(load_toml(args.trust_lanes).get("class", {}))
    print(f"Trust-lane classes in {args.trust_lanes}: {class_count}")

    if issues:
        print(f"Issues ({len(issues)}):")
        for issue in issues:
            print(f"  - {issue}")
    else:
        print("All trust-lane classes valid.")

    return 1 if args.strict and issues else 0


if __name__ == "__main__":
    sys.exit(main())
