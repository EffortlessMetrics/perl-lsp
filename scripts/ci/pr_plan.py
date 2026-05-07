#!/usr/bin/env python3
"""Advisory PR Plan: forecast LEM cost and selected lanes from policy TOML.

Reads:
  policy/ci-budget.toml
  policy/ci-lanes.toml
  policy/ci-risk-packs.toml

Inputs:
  --base, --head     git refs (defaults: origin/master, HEAD)
  --labels-json      JSON array of label strings (e.g. github PR labels)
  --json-out         path to write ci-plan.json
  --summary          path to GITHUB_STEP_SUMMARY (optional; written if set)

Output: target/ci/ci-plan.json (or path passed via --json-out).

This is the Python prototype. PR 12 replaces it with `cargo xtask ci plan`,
which reuses the existing ci-scope changed-file classifier.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


def read_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as f:
        return tomllib.load(f)


def changed_files(base: str, head: str) -> list[str]:
    try:
        out = subprocess.check_output(
            ["git", "diff", "--name-only", f"{base}...{head}"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        return []
    return [line for line in out.splitlines() if line.strip()]


def path_matches_glob(path: str, pattern: str) -> bool:
    """Subset of fnmatch tuned for our policy globs (supports **)."""
    # Escape regex specials except *, ?, /, .
    regex_parts: list[str] = []
    i = 0
    while i < len(pattern):
        c = pattern[i]
        if c == "*" and i + 1 < len(pattern) and pattern[i + 1] == "*":
            regex_parts.append(".*")
            i += 2
            if i < len(pattern) and pattern[i] == "/":
                i += 1
        elif c == "*":
            regex_parts.append("[^/]*")
            i += 1
        elif c == "?":
            regex_parts.append("[^/]")
            i += 1
        elif c == ".":
            regex_parts.append(r"\.")
            i += 1
        elif c in r"+()|[]{}^$\\":
            regex_parts.append(re.escape(c))
            i += 1
        else:
            regex_parts.append(c)
            i += 1
    regex = "^" + "".join(regex_parts) + "$"
    return re.match(regex, path) is not None


def classify_areas(files: list[str], risk_packs: dict[str, Any]) -> tuple[list[str], list[str]]:
    """Return (selected_risk_pack_ids, matched_areas)."""
    selected: list[str] = []
    areas: set[str] = set()
    for pack_id, pack in risk_packs.items():
        paths: list[str] = pack.get("paths", [])
        keywords: list[str] = pack.get("keywords", [])
        matched = False
        for f in files:
            if any(path_matches_glob(f, p) for p in paths):
                matched = True
                break
            if any(k in f.lower() for k in keywords):
                matched = True
                break
        if matched:
            selected.append(pack_id)
            areas.add(pack_id)
    return selected, sorted(areas)


def docs_only(files: list[str]) -> bool:
    if not files:
        return False
    docs_globs = [
        "docs/**",
        "**/*.md",
        "README*",
        "CHANGELOG*",
        "RELEASE_HISTORY*",
    ]
    for f in files:
        if not any(path_matches_glob(f, g) for g in docs_globs):
            return False
    return True


def select_lanes(
    *,
    files: list[str],
    labels: list[str],
    risk_pack_ids: list[str],
    risk_packs: dict[str, Any],
    lanes: dict[str, Any],
) -> list[dict[str, Any]]:
    """Pick lanes that should run for this PR."""
    selected_ids: set[str] = set()

    if docs_only(files):
        if "docs_gate" in lanes:
            selected_ids.add("docs_gate")
    else:
        # Drive default-PR lanes from policy/ci-lanes.toml's `default_pr = true`
        # flag rather than hardcoding a list. This keeps the policy file as the
        # single source of truth: any lane added to ci-lanes.toml with
        # default_pr = true is automatically picked up.
        # `docs_gate` is excluded because it is handled by the docs-only branch.
        for lane_id, lane in lanes.items():
            if lane.get("default_pr") and lane_id != "docs_gate":
                selected_ids.add(lane_id)

    # Add lanes from selected risk packs.
    for pack_id in risk_pack_ids:
        pack = risk_packs.get(pack_id, {})
        for lane_id in pack.get("lanes", []):
            if lane_id in lanes:
                selected_ids.add(lane_id)

    # Label-triggered lanes.
    label_set = {l.lower() for l in labels}
    for lane_id, lane in lanes.items():
        lane_labels = [l.lower() for l in lane.get("labels", [])]
        if any(lbl in label_set for lbl in lane_labels):
            selected_ids.add(lane_id)

    # full-ci pulls in deep_lanes for matched risk packs.
    if "full-ci" in label_set:
        for pack_id in risk_pack_ids:
            pack = risk_packs.get(pack_id, {})
            for lane_id in pack.get("deep_lanes", []):
                if lane_id in lanes:
                    selected_ids.add(lane_id)

    out: list[dict[str, Any]] = []
    for lane_id in sorted(selected_ids):
        lane = lanes[lane_id]
        out.append(
            {
                "id": lane_id,
                "intent": lane.get("intent", ""),
                "runner": lane.get("runner", "ubuntu_24_04"),
                "base_lem": lane.get("base_lem"),
                "base_minutes": lane.get("base_minutes"),
                "default_pr": bool(lane.get("default_pr", False)),
                "blocking": bool(lane.get("blocking", False)),
            }
        )
    return out


def lane_lem(lane: dict[str, Any], multipliers: dict[str, float]) -> float:
    """Resolve a lane's LEM, using base_lem if present else base_minutes × multiplier."""
    if lane.get("base_lem") is not None:
        return float(lane["base_lem"])
    if lane.get("base_minutes") is not None:
        runner = lane.get("runner", "ubuntu_24_04")
        mult = multipliers.get(runner, 1.0)
        return float(lane["base_minutes"]) * float(mult)
    return 0.0


def band_for(lem: float, budget: dict[str, Any]) -> str:
    if lem <= budget.get("default_limit_lem", 35):
        return "default"
    if lem <= budget.get("elevated_limit_lem", 75):
        return "elevated"
    if lem <= budget.get("hard_limit_lem", 125):
        return "high"
    return "over_ceiling"


def render_summary(plan: dict[str, Any]) -> str:
    bud = plan["budget"]
    lines = [
        "# PR Plan",
        "",
        f"**Estimated LEM:** `{bud['estimated_lem']:.1f}` ({bud['band']})",
        f"**Default limit:** `{bud['default_limit_lem']}`  /  "
        f"**Elevated:** `{bud['elevated_limit_lem']}`  /  "
        f"**Hard ceiling:** `{bud['hard_limit_lem']}`",
        f"**Estimated $:** `${bud['estimated_usd']:.2f}` (display only)",
        "",
        "## Selected lanes",
        "",
        "| Lane | Runner | Base LEM | Default-PR | Blocking |",
        "|---|---|---:|:---:|:---:|",
    ]
    for lane in plan["selection"]["lanes"]:
        base = lane.get("base_lem")
        if base is None:
            base = f"{lane.get('base_minutes', '?')}m"
        lines.append(
            f"| `{lane['id']}` | {lane['runner']} | {base} | "
            f"{'✓' if lane['default_pr'] else ''} | {'✓' if lane['blocking'] else ''} |"
        )
    if plan["selection"]["risk_packs"]:
        lines.append("")
        lines.append("## Risk packs")
        lines.append("")
        for p in plan["selection"]["risk_packs"]:
            lines.append(f"- `{p}`")
    if plan["warnings"]:
        lines.append("")
        lines.append("## Warnings")
        lines.append("")
        for w in plan["warnings"]:
            lines.append(f"- {w}")
    lines.append("")
    lines.append(
        "_PR Plan is advisory. See "
        "[`docs/ci/lem-budgeting.md`](../blob/master/docs/ci/lem-budgeting.md) "
        "for the LEM model._"
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=os.environ.get("BASE_SHA", "origin/master"))
    parser.add_argument("--head", default=os.environ.get("HEAD_SHA", "HEAD"))
    parser.add_argument("--labels-json", default="[]")
    parser.add_argument("--budget", type=Path, default=Path("policy/ci-budget.toml"))
    parser.add_argument("--lanes", type=Path, default=Path("policy/ci-lanes.toml"))
    parser.add_argument(
        "--risk-packs", type=Path, default=Path("policy/ci-risk-packs.toml")
    )
    parser.add_argument(
        "--json-out", type=Path, default=Path("target/ci/ci-plan.json")
    )
    parser.add_argument("--summary", type=str, default=os.environ.get("GITHUB_STEP_SUMMARY"))
    args = parser.parse_args()

    budget_doc = read_toml(args.budget)
    lanes_doc = read_toml(args.lanes)
    risk_packs_doc = read_toml(args.risk_packs)

    budget = budget_doc.get("budget", {})
    multipliers = budget_doc.get("runner_multipliers", {})
    lanes = lanes_doc.get("lane", {})
    risk_packs = risk_packs_doc.get("risk_pack", {})

    try:
        labels = json.loads(args.labels_json) if args.labels_json else []
        if not isinstance(labels, list):
            labels = []
    except json.JSONDecodeError:
        labels = []

    files = changed_files(args.base, args.head)
    selected_packs, areas = classify_areas(files, risk_packs)
    selected_lanes = select_lanes(
        files=files,
        labels=labels,
        risk_pack_ids=selected_packs,
        risk_packs=risk_packs,
        lanes=lanes,
    )

    estimated_lem = sum(lane_lem(lane, multipliers) for lane in selected_lanes)
    rate = float(budget.get("linux_minute_rate_usd", 0.008))

    warnings: list[str] = []
    band = band_for(estimated_lem, budget)
    if band == "elevated":
        warnings.append(
            f"Estimated LEM {estimated_lem:.1f} is in the *elevated* band "
            f"(>{budget.get('default_limit_lem', 35)}). Consider whether all selected lanes are needed."
        )
    elif band == "high":
        warnings.append(
            f"Estimated LEM {estimated_lem:.1f} is in the *high* band. "
            "Consider applying `ci-budget-ack`."
        )
    elif band == "over_ceiling":
        if not any(lbl in {"full-ci", "ci-budget-override"} for lbl in labels):
            warnings.append(
                f"Estimated LEM {estimated_lem:.1f} exceeds hard ceiling "
                f"({budget.get('hard_limit_lem', 125)}). PR Plan is advisory in this rollout PR; "
                "PR 13 will fail without `full-ci` or `ci-budget-override`."
            )

    plan: dict[str, Any] = {
        "schema_version": 1,
        "repo": "perl-lsp",
        "base_sha": args.base,
        "head_sha": args.head,
        "labels": labels,
        "posture": "rust",
        "budget": {
            "estimated_lem": estimated_lem,
            "band": band,
            "default_limit_lem": int(budget.get("default_limit_lem", 35)),
            "elevated_limit_lem": int(budget.get("elevated_limit_lem", 75)),
            "hard_limit_lem": int(budget.get("hard_limit_lem", 125)),
            "estimated_usd": estimated_lem * rate,
        },
        "changed": {
            "files": files,
            "areas": areas,
            "docs_only": docs_only(files),
        },
        "selection": {
            "risk_packs": selected_packs,
            "lanes": selected_lanes,
        },
        "warnings": warnings,
    }

    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(json.dumps(plan, indent=2) + "\n")

    if args.summary:
        summary_path = Path(args.summary)
        summary_path.parent.mkdir(parents=True, exist_ok=True)
        with summary_path.open("a") as f:
            f.write(render_summary(plan))

    print(json.dumps({"estimated_lem": estimated_lem, "band": band, "lanes": len(selected_lanes)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
