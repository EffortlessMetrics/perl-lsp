#!/usr/bin/env python3
"""Validate risk packs in policy/ci-risk-packs.toml.

Checks:
  - Every `lanes` and `deep_lanes` entry references a real lane in
    policy/ci-lanes.toml.
  - Every risk pack has a non-empty `paths` or `keywords` filter.
  - Every label referenced is a string (no nested structure mistakes).

Reports issues; --strict fails on any.
"""
from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path
from typing import Any


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--risk-packs", type=Path, default=Path("policy/ci-risk-packs.toml")
    )
    p.add_argument("--lanes", type=Path, default=Path("policy/ci-lanes.toml"))
    p.add_argument("--strict", action="store_true")
    args = p.parse_args()

    risk_packs: dict[str, Any] = (
        tomllib.loads(args.risk_packs.read_text(encoding="utf-8"))
        .get("risk_pack", {})
    )
    lane_ids: set[str] = set(
        tomllib.loads(args.lanes.read_text(encoding="utf-8"))
        .get("lane", {})
        .keys()
    )

    issues: list[str] = []
    print(f"Risk packs in {args.risk_packs}: {len(risk_packs)}")
    print(f"Lanes in {args.lanes}: {len(lane_ids)}")
    print()

    for pack_id, pack in risk_packs.items():
        for lane in pack.get("lanes", []):
            if lane not in lane_ids:
                issues.append(
                    f"{pack_id}.lanes references unknown lane '{lane}'"
                )
        for lane in pack.get("deep_lanes", []):
            if lane not in lane_ids:
                issues.append(
                    f"{pack_id}.deep_lanes references unknown lane '{lane}'"
                )
        if not pack.get("paths") and not pack.get("keywords"):
            issues.append(f"{pack_id} has neither `paths` nor `keywords`")
        for label in pack.get("labels", []):
            if not isinstance(label, str):
                issues.append(f"{pack_id} has non-string label: {label!r}")

    if issues:
        print(f"Issues ({len(issues)}):")
        for i in issues:
            print(f"  - {i}")
    else:
        print("All risk packs valid.")

    return 1 if args.strict and issues else 0


if __name__ == "__main__":
    sys.exit(main())
