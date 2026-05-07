#!/usr/bin/env python3
"""Validate policy/ci-risk-packs.toml against policy/ci-lanes.toml.

Checks:
  1. risk-pack TOML parses.
  2. Every lane referenced under `lanes` and `deep_lanes` exists in
     policy/ci-lanes.toml.
  3. Every risk pack declares at least `description` and `lanes` (deep_lanes,
     paths, keywords, labels are optional).
  4. `paths` entries do not include leading "./" or absolute paths and are
     plausible globs.
  5. No duplicate risk-pack ids.
  6. Risk-pack ids are snake_case.

Run:
    python3 scripts/ci/validate_risk_packs.py

Exits non-zero on any violation.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
import tomllib

REQUIRED_FIELDS = ("description", "lanes")
OPTIONAL_FIELDS = ("paths", "keywords", "deep_lanes", "labels")
SNAKE_CASE = re.compile(r"^[a-z][a-z0-9_]*$")


def load(path: pathlib.Path) -> dict:
    return tomllib.loads(path.read_text())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--risk-packs",
        type=pathlib.Path,
        default=pathlib.Path("policy/ci-risk-packs.toml"),
    )
    parser.add_argument(
        "--lanes",
        type=pathlib.Path,
        default=pathlib.Path("policy/ci-lanes.toml"),
    )
    args = parser.parse_args()

    errors: list[str] = []

    try:
        risk = load(args.risk_packs)
    except Exception as exc:
        print(f"FAIL: parse {args.risk_packs}: {exc}", file=sys.stderr)
        return 2

    try:
        lanes = load(args.lanes)
    except Exception as exc:
        print(f"FAIL: parse {args.lanes}: {exc}", file=sys.stderr)
        return 2

    lane_ids = set(lanes.get("lane", {}).keys())
    if not lane_ids:
        errors.append(f"{args.lanes}: no [lane.*] entries found")

    packs = risk.get("risk_pack", {})
    if not packs:
        errors.append(f"{args.risk_packs}: no [risk_pack.*] entries found")

    for pid, pack in packs.items():
        if not SNAKE_CASE.match(pid):
            errors.append(f"risk_pack.{pid}: id must be snake_case")

        for field in REQUIRED_FIELDS:
            if field not in pack:
                errors.append(f"risk_pack.{pid}: missing required field `{field}`")

        for field in pack:
            if field not in REQUIRED_FIELDS + OPTIONAL_FIELDS:
                errors.append(
                    f"risk_pack.{pid}: unknown field `{field}` "
                    f"(allowed: {sorted(set(REQUIRED_FIELDS + OPTIONAL_FIELDS))})"
                )

        for field in ("lanes", "deep_lanes"):
            for lane_id in pack.get(field, []):
                if lane_id not in lane_ids:
                    errors.append(
                        f"risk_pack.{pid}.{field}: unknown lane `{lane_id}` "
                        f"(not declared in {args.lanes})"
                    )

        for path in pack.get("paths", []):
            if path.startswith("./") or path.startswith("/"):
                errors.append(
                    f"risk_pack.{pid}.paths: `{path}` must be repo-relative without leading ./"
                )
            if path.strip() != path:
                errors.append(f"risk_pack.{pid}.paths: `{path}` has surrounding whitespace")

        for kw in pack.get("keywords", []):
            if not kw or kw != kw.lower():
                errors.append(
                    f"risk_pack.{pid}.keywords: `{kw}` must be lowercase non-empty"
                )

    if errors:
        print(f"FAIL: {len(errors)} risk-pack validation error(s):", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"OK: {len(packs)} risk pack(s) validated against "
        f"{len(lane_ids)} lane(s)."
    )
    for pid, pack in packs.items():
        print(
            f"  - {pid}: paths={len(pack.get('paths', []))} "
            f"keywords={len(pack.get('keywords', []))} "
            f"lanes={pack.get('lanes', [])} deep_lanes={pack.get('deep_lanes', [])}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
