#!/usr/bin/env python3
"""Validate features.toml invariants for truth surface integrity.

This script enforces rules that prevent the feature catalog from
accidentally misrepresenting the project's capabilities:

1. GA + advertised => tests must be non-empty (no claiming untested features)
2. No duplicate feature IDs
3. advertised=true + counts_in_coverage=true is the headline metric

Exit code 0 = all invariants pass
Exit code 1 = invariant violations found
"""

from __future__ import annotations

import pathlib
import sys
from typing import Any

try:
    import tomllib
except ImportError:
    import tomli as tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
FEATURES_TOML = ROOT / "features.toml"


def check_invariants(features: list[dict[str, Any]]) -> list[str]:
    """Check all invariants and return list of violations."""
    violations: list[str] = []
    seen_ids: set[str] = set()

    for i, f in enumerate(features):
        fid = f.get("id", f"<unnamed feature #{i}>")
        advertised = f.get("advertised", False)
        maturity = f.get("maturity", "planned")
        tests = f.get("tests", [])
        counts_in_coverage = f.get("counts_in_coverage", True)

        # Rule 1: No duplicate IDs
        if fid in seen_ids:
            violations.append(f"DUPLICATE_ID: {fid!r} appears more than once")
        seen_ids.add(fid)

        # Rule 2: GA + advertised => tests must be non-empty
        # This prevents claiming "GA" status for untested features
        if advertised and maturity == "ga" and not tests:
            # Exception: features with counts_in_coverage=false are protocol/plumbing
            # and can be GA without tests (they're not in the headline metric)
            if counts_in_coverage is not False:
                violations.append(
                    f"UNTESTED_GA: {fid!r} is advertised+GA but has no tests. "
                    f"Either add tests or set counts_in_coverage=false (if it's protocol plumbing)."
                )

        # Rule 3: Warn if advertised but not counting in coverage
        # (This is allowed but worth noting)
        if advertised and counts_in_coverage is False and maturity == "ga":
            # This is fine - just informational
            pass

    return violations


def main() -> int:
    data = tomllib.loads(FEATURES_TOML.read_text(encoding="utf-8"))
    features = data.get("feature", [])

    violations = check_invariants(features)

    if violations:
        print("FEATURE INVARIANT VIOLATIONS:", file=sys.stderr)
        print("=" * 50, file=sys.stderr)
        for v in violations:
            print(f"  - {v}", file=sys.stderr)
        print("=" * 50, file=sys.stderr)
        print(f"\n{len(violations)} violation(s) found.", file=sys.stderr)
        print("\nTo fix:", file=sys.stderr)
        print("  - DUPLICATE_ID: Remove duplicate entries from features.toml", file=sys.stderr)
        print("  - UNTESTED_GA: Either add tests[] entries or set counts_in_coverage=false", file=sys.stderr)
        return 1

    # Print summary
    total = len(features)
    ga_advertised = sum(
        1 for f in features
        if f.get("advertised") and f.get("maturity") == "ga"
    )
    headline_features = sum(
        1 for f in features
        if f.get("advertised")
        and f.get("maturity") == "ga"
        and f.get("counts_in_coverage", True) is not False
    )

    print(f"Feature invariants OK: {total} features, {ga_advertised} GA+advertised, {headline_features} in headline metric")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
