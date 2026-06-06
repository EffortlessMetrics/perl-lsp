#!/usr/bin/env python3
"""Validate that every gate in .ci/gate-policy.yaml maps to a lane in
policy/ci-lanes.toml (or is explicitly declared as not-mapped).

This is the cross-reference validator: it does not enforce, it reports.
PR 09 of the CI economics rollout.

Usage:
  scripts/ci/validate_gate_lane_mapping.py [--strict]

Exit codes:
  0 - all gates map to a lane (or are explicitly not-mapped)
  1 - mapping mismatch (missing entries) and --strict was passed
"""
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Any

# Many-to-one and one-to-many mappings between gate-policy.yaml gate names and
# policy/ci-lanes.toml lane keys. Right-hand side is a list of lane keys (a
# single gate may map to multiple lanes, e.g. an aggregate gate that runs in
# both pr_smoke and merge_gate_shards).
#
# Gates explicitly not yet mapped to a lane carry an empty list and a reason.
GATE_TO_LANE_MAP: dict[str, dict[str, Any]] = {
    # pr_fast gates roll up under pr_smoke
    "fmt": {"lanes": ["pr_smoke"]},
    "check_conflict_markers": {"lanes": ["conflict_markers"]},
    "release_history": {"lanes": ["pr_smoke"]},
    "readme_heading_check": {"lanes": ["pr_smoke"]},
    "publish_closure": {"lanes": ["pr_smoke"]},
    "publish_manifest_check": {"lanes": ["pr_smoke"]},
    "layer_check": {"lanes": ["pr_smoke"]},
    "published_crate_count_pr_fast": {"lanes": ["pr_smoke"]},
    "release_history_check": {"lanes": ["pr_smoke"]},
    "clippy_scoped": {"lanes": ["pr_smoke"]},
    "unit_scoped": {"lanes": ["pr_smoke"]},
    "check_tests_scoped": {"lanes": ["pr_smoke"]},
    "inline_completion_contract": {"lanes": ["pr_smoke"]},
    "inline_completion_quality_receipt": {"lanes": ["pr_smoke"]},

    # core / foundation gates roll up under merge_gate_shards
    "clippy_core": {"lanes": ["merge_gate_shards"]},
    "unit_core": {"lanes": ["merge_gate_shards"]},
    "perl_token_leaf_contract": {"lanes": ["merge_gate_shards"]},
    "clippy_full": {"lanes": ["merge_gate_shards"]},
    "unit_foundation_full": {"lanes": ["merge_gate_shards"]},
    "unit_parser_stack_full": {"lanes": ["merge_gate_shards"]},
    "unit_analysis_full": {"lanes": ["merge_gate_shards"]},
    "unit_lsp_full": {"lanes": ["merge_gate_shards"]},
    "unit_dap_support_full": {"lanes": ["merge_gate_shards"]},
    "compile_all_targets": {"lanes": ["check_all_targets"]},
    "lsp_smoke": {"lanes": ["ux_tests"]},

    # corpus / parser maintenance gates
    "common_corpus_clean": {"lanes": ["merge_gate_shards"]},
    "parser_corpus_ratchet": {"lanes": ["merge_gate_shards"]},
    "cpan_corpus_ratchet": {"lanes": ["merge_gate_shards"]},
    "parser_audit_closeout": {"lanes": ["merge_gate_shards"]},
    "v2_parity": {"lanes": ["merge_gate_shards"]},
    "v2_bundle_sync": {"lanes": ["merge_gate_shards"]},

    # security / policy gates
    "security_audit": {"lanes": ["security_audit"]},
    "policy_checks": {"lanes": ["pr_smoke"]},
    "workflow_audit": {"lanes": ["pr_smoke"]},
    "nested_lock_check": {"lanes": ["pr_smoke"]},

    # release-adjacent gates
    "docs_build": {"lanes": ["docs_gate"]},
    "published_crate_count": {"lanes": ["release_check"]},
    "release_build": {"lanes": ["release_check"]},
    "inline_completion_binary_smoke": {"lanes": ["release_check"]},
    "version_sync": {"lanes": ["release_check"]},
    "sbom_verify": {"lanes": ["release_check"]},
    "determinism_check": {"lanes": ["release_check"]},

    # nightly deep gates
    "mutation": {"lanes": ["mutation"]},
    "fuzz": {"lanes": ["fuzz"]},
    "benchmarks": {"lanes": ["real_repo_latency"]},
    "full_matrix": {"lanes": ["perl_version_matrix"]},
    "coverage": {"lanes": ["coverage"]},
    "corpus_validation": {"lanes": ["mutation"]},
    "corpus_sweep": {"lanes": ["mutation"]},

    # LSP tier gates
    "lsp_tier_a": {"lanes": ["ux_tests", "real_repo_latency"]},
    "lsp_tier_b": {"lanes": ["real_repo_latency"]},
}


def read_yaml_gate_names(gate_policy_path: Path) -> list[str]:
    """Lightweight gate-name extraction without a full YAML dependency.

    Looks for `^  - name: <ident>$` patterns under the `gates:` block. Avoids
    importing pyyaml, which keeps this validator runnable on a clean Python.
    """
    text = gate_policy_path.read_text(encoding="utf-8")
    in_gates = False
    names: list[str] = []
    for line in text.splitlines():
        stripped = line.rstrip()
        if stripped == "gates:":
            in_gates = True
            continue
        if in_gates and stripped and not line.startswith(" "):
            # Hit the next top-level key — end of gates block.
            break
        if in_gates and line.startswith("  - name:"):
            name = line.split("name:", 1)[1].strip()
            if name:
                names.append(name)
    return names


def read_lane_ids(lanes_path: Path) -> set[str]:
    with lanes_path.open("rb") as f:
        doc = tomllib.load(f)
    return set(doc.get("lane", {}).keys())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--gate-policy", type=Path, default=Path(".ci/gate-policy.yaml")
    )
    parser.add_argument(
        "--lanes", type=Path, default=Path("policy/ci-lanes.toml")
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit 1 on any unmapped gate or missing lane reference.",
    )
    parser.add_argument(
        "--json-out",
        type=Path,
        default=None,
        help="Write the mapping report as JSON to this path.",
    )
    args = parser.parse_args()

    gate_names = read_yaml_gate_names(args.gate_policy)
    lane_ids = read_lane_ids(args.lanes)

    unmapped_gates: list[str] = []
    missing_lanes: list[tuple[str, str]] = []
    mapped: list[tuple[str, list[str]]] = []

    for gate in gate_names:
        entry = GATE_TO_LANE_MAP.get(gate)
        if entry is None:
            unmapped_gates.append(gate)
            continue
        lanes_for_gate = entry.get("lanes") or []
        for lane in lanes_for_gate:
            if lane not in lane_ids:
                missing_lanes.append((gate, lane))
        mapped.append((gate, lanes_for_gate))

    print(f"Gates in {args.gate_policy}: {len(gate_names)}")
    print(f"Lanes in {args.lanes}: {len(lane_ids)}")
    print(f"Mapped: {len(mapped)}")
    print(f"Unmapped gates: {len(unmapped_gates)}")
    if unmapped_gates:
        for g in unmapped_gates[:30]:
            print(f"  - {g}")
        if len(unmapped_gates) > 30:
            print(f"  ... and {len(unmapped_gates) - 30} more")
    print(f"Mapped to non-existent lanes: {len(missing_lanes)}")
    for g, l in missing_lanes:
        print(f"  - {g} -> {l}  (lane not in ci-lanes.toml)")

    if args.json_out:
        report = {
            "gate_policy": str(args.gate_policy),
            "lanes": str(args.lanes),
            "gate_count": len(gate_names),
            "lane_count": len(lane_ids),
            "mapped": [{"gate": g, "lanes": l} for g, l in mapped],
            "unmapped_gates": unmapped_gates,
            "missing_lanes": [{"gate": g, "lane": l} for g, l in missing_lanes],
        }
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if args.strict and (unmapped_gates or missing_lanes):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
