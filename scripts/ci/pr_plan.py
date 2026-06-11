#!/usr/bin/env python3
"""Advisory PR Plan: forecast LEM cost and selected lanes from policy TOML.

Reads:
  policy/ci-budget.toml
  policy/ci-lanes.toml
  policy/ci-risk-packs.toml
  policy/trust-lanes.toml

Inputs:
  --base, --head     git refs (defaults: origin/main, HEAD)
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


def lane_paths_match(lane: dict[str, Any], files: list[str]) -> bool:
    """True if a lane's path filter matches at least one changed file.

    A lane without a `paths` field is treated as path-agnostic (matches).
    A lane with a `paths` field is selected only when at least one changed
    file matches, mirroring the GitHub workflow `paths:` filter behavior.
    """
    patterns = lane.get("paths")
    if not patterns:
        return True
    return any(path_matches_glob(f, p) for f in files for p in patterns)


TRUST_LANE_RULES: dict[str, list[tuple[str, list[str]]]] = {
    "parser_fixture_only": [
        (
            "parser fixture or generated parser status",
            [
                "crates/perl-parser*/tests/**",
                "crates/perl-parser*/fixtures/**",
                "tests/fixtures/parser/**",
                "docs/project/status/parser.md",
                "docs/project/status/parser_accuracy_next.md",
            ],
        )
    ],
    "parser_runtime_fix": [
        (
            "parser, lexer, or parser-core runtime source",
            [
                "crates/perl-parser*/src/**",
                "crates/perl-lexer/src/**",
                "crates/perl-parser-core/src/**",
            ],
        )
    ],
    "provider_receipt": [
        (
            "provider receipt, shadow, scorecard, or runtime proof",
            [
                "docs/project/status/provider_confidence_matrix.md",
                "docs/project/status/provider_cutover.md",
                "docs/project/status/provider_promotion_ledger.md",
                "docs/project/status/semantic_scorecard.md",
                "docs/project/status/semantic_shadow_compare.md",
                "crates/perl-lsp-rs/tests/**",
                "crates/perl-lsp-rs-core/tests/**",
                "crates/perl-lsp-ux-tests/**",
            ],
        )
    ],
    "provider_live_cutover": [
        (
            "live provider runtime source",
            [
                "crates/perl-lsp-rs/src/runtime/**",
                "crates/perl-lsp-rs-core/src/providers/**",
                "crates/perl-lsp-core/src/**",
                "crates/perl-lsp-feature-*/src/**",
            ],
        )
    ],
    "support_claim_change": [
        (
            "public support claim surface",
            [
                "README.md",
                "vscode-extension/README.md",
                "docs/project/status/SUPPORT_TIERS.md",
            ],
        )
    ],
    "subprocess_seam": [
        (
            "Perl, module path, perldoc, DAP, or launch seam",
            [
                "crates/perl-dap*/**",
                "crates/perl-module*/**",
                "crates/*perldoc*/**",
                "crates/*oracle*/**",
                "vscode-extension/src/**launch**",
                "vscode-extension/src/**dap**",
            ],
        )
    ],
    "real_workspace_receipt": [
        (
            "real-workspace baseline or livability receipt",
            [
                "docs/forensics/**",
                "crates/perl-lsp-ux-tests/**",
                "crates/perl-lsp-rs/tests/**real_workspace**",
                "crates/perl-lsp-rs/tests/**baseline**",
            ],
        )
    ],
    "release_proof": [
        (
            "release, packaging, managed-binary, or distribution surface",
            [
                "RELEASE_HISTORY.md",
                "CHANGELOG.md",
                ".github/workflows/publish*.yml",
                ".github/workflows/release*.yml",
                "docs/release/**",
                "vscode-extension/package.json",
                "vscode-extension/package-lock.json",
            ],
        )
    ],
    "dependency_update": [
        (
            "dependency graph, lockfile, or toolchain surface",
            [
                "Cargo.toml",
                "Cargo.lock",
                "**/Cargo.toml",
                "rust-toolchain*",
                "package.json",
                "package-lock.json",
                "pnpm-lock.yaml",
                "npm-shrinkwrap.json",
            ],
        )
    ],
    "docs_status_only": [
        (
            "docs, status, spec, ADR, policy, or CI-planning control surface",
            [
                "docs/**",
                "**/*.md",
                "policy/**",
                "scripts/ci/pr_plan.py",
                "scripts/ci/validate_*",
                ".github/workflows/pr-plan.yml",
            ],
        )
    ],
}


def match_trust_lane_files(
    files: list[str],
    *,
    class_id: str,
) -> list[dict[str, Any]]:
    matches: list[dict[str, Any]] = []
    for reason, patterns in TRUST_LANE_RULES.get(class_id, []):
        matched = sorted(
            {path for path in files if any(path_matches_glob(path, p) for p in patterns)}
        )
        if matched:
            matches.append({"reason": reason, "files": matched})
    return matches


def trust_lane_entry(
    class_id: str,
    class_doc: dict[str, Any],
    matches: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "id": class_id,
        "risk_rank": class_doc.get("risk_rank"),
        "claim_boundary": class_doc.get("claim_boundary", ""),
        "required_checks": class_doc.get("required_checks", []),
        "optional_checks": class_doc.get("optional_checks", []),
        "skipped_by_policy_checks": class_doc.get("skipped_by_policy_checks", []),
        "widening_triggers": class_doc.get("widening_triggers", []),
        "receipt_paths": class_doc.get("receipt_paths", []),
        "support_claim_impact": class_doc.get("support_claim_impact", ""),
        "matches": matches,
    }


def classify_trust_lanes(
    files: list[str],
    trust_lanes_doc: dict[str, Any],
    *,
    estimated_lem: float,
    band: str,
    selected_lanes: list[dict[str, Any]],
) -> dict[str, Any]:
    classes = trust_lanes_doc.get("class", {})
    matched: list[dict[str, Any]] = []

    if isinstance(classes, dict):
        for class_id, class_doc in classes.items():
            if not isinstance(class_doc, dict):
                continue
            matches = match_trust_lane_files(files, class_id=class_id)
            if matches:
                matched.append(trust_lane_entry(class_id, class_doc, matches))

    matched.sort(
        key=lambda entry: (
            int(entry["risk_rank"]) if isinstance(entry.get("risk_rank"), int) else 0,
            str(entry["id"]),
        ),
        reverse=True,
    )
    strongest = matched[0] if matched else None
    changed_surface = sorted(
        {
            match["reason"]
            for entry in matched
            for match in entry.get("matches", [])
            if isinstance(match.get("reason"), str)
        }
    )

    result: dict[str, Any] = {
        "schema_version": trust_lanes_doc.get("schema_version", 1),
        "policy": trust_lanes_doc.get("policy", "trust-lanes"),
        "status": trust_lanes_doc.get("status", "advisory"),
        "spec": trust_lanes_doc.get("spec"),
        "classes": matched,
        "strongest_class": strongest,
        "changed_surface": changed_surface,
        "hosted_ci_estimate": {
            "estimated_lem": estimated_lem,
            "band": band,
            "selected_lanes": len(selected_lanes),
        },
    }
    if strongest:
        result["required_proof"] = strongest.get("required_checks", [])
        result["skipped_by_policy_checks"] = strongest.get(
            "skipped_by_policy_checks", []
        )
        result["widening_triggers"] = strongest.get("widening_triggers", [])
        result["support_claim_impact"] = strongest.get("support_claim_impact", "")
    else:
        result["required_proof"] = []
        result["skipped_by_policy_checks"] = []
        result["widening_triggers"] = []
        result["support_claim_impact"] = ""
    return result


def select_lanes(
    *,
    files: list[str],
    labels: list[str],
    risk_pack_ids: list[str],
    risk_packs: dict[str, Any],
    lanes: dict[str, Any],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Pick lanes that should run for this PR.

    Returns (selected, skipped) where:
      selected: lanes that will run, with selection origin attached.
      skipped:  lanes considered (default_pr or label-triggered) but
                excluded because their path filter did not match. Useful
                for transparency in the PR Plan summary.
    """
    # Track origin: where the selection came from.
    origins: dict[str, list[str]] = {}

    def mark(lane_id: str, origin: str) -> None:
        origins.setdefault(lane_id, []).append(origin)

    if docs_only(files):
        if "docs_gate" in lanes:
            mark("docs_gate", "docs-only")
    else:
        # Drive default-PR lanes from policy/ci-lanes.toml's `default_pr = true`
        # flag rather than hardcoding a list. This keeps the policy file as the
        # single source of truth: any lane added to ci-lanes.toml with
        # default_pr = true is automatically picked up.
        # `docs_gate` is excluded because it is handled by the docs-only branch.
        for lane_id, lane in lanes.items():
            if lane.get("default_pr") and lane_id != "docs_gate":
                mark(lane_id, "default-pr")

    # Add lanes from selected risk packs.
    for pack_id in risk_pack_ids:
        pack = risk_packs.get(pack_id, {})
        for lane_id in pack.get("lanes", []):
            if lane_id in lanes:
                mark(lane_id, f"risk-pack:{pack_id}")

    # Label-triggered lanes.
    label_set = {l.lower() for l in labels}
    for lane_id, lane in lanes.items():
        lane_labels = [l.lower() for l in lane.get("labels", [])]
        for lbl in lane_labels:
            if lbl in label_set:
                mark(lane_id, f"label:{lbl}")

    # full-ci pulls in deep_lanes for matched risk packs.
    if "full-ci" in label_set:
        for pack_id in risk_pack_ids:
            pack = risk_packs.get(pack_id, {})
            for lane_id in pack.get("deep_lanes", []):
                if lane_id in lanes:
                    mark(lane_id, "deep-lane:full-ci")

    selected: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    for lane_id in sorted(origins.keys()):
        lane = lanes[lane_id]
        entry = {
            "id": lane_id,
            "intent": lane.get("intent", ""),
            "runner": lane.get("runner", "ubuntu_24_04"),
            "base_lem": lane.get("base_lem"),
            "default_pr": bool(lane.get("default_pr", False)),
            "blocking": bool(lane.get("blocking", False)),
            "origin": origins[lane_id],
        }
        # Honor lane-level `paths:` filters. A path-filtered lane that doesn't
        # match the diff is reported as skipped (not silently dropped).
        if lane_paths_match(lane, files):
            selected.append(entry)
        else:
            entry["skipped_reason"] = "paths-filter-no-match"
            skipped.append(entry)
    return selected, skipped


def lane_lem(lane: dict[str, Any], multipliers: dict[str, float]) -> float:
    """Resolve a lane's LEM, using base_lem if present else base_minutes × multiplier."""
    if lane.get("base_lem") is not None:
        return float(lane["base_lem"])
    if lane.get("base_minutes") is not None:
        runner = lane.get("runner", "ubuntu_24_04")
        mult = multipliers.get(runner, 1.0)
        return float(lane["base_minutes"]) * float(mult)
    return 0.0


def load_learned_history(path: Path) -> dict[str, Any]:
    """Read .ci/metrics/ci-lane-history.json if present; tolerant on errors."""
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {}


def apply_learned_estimates(
    lanes: list[dict[str, Any]], history: dict[str, Any]
) -> tuple[float, int]:
    """Mutate `lanes` in place: replace `base_lem` with learned estimate where
    available. Returns (delta_lem, learned_count) for the summary.

    Estimate model (matches scripts/ci/learned_estimate.py):
      estimate = max(static_floor, p50_recent_actual * 1.15)
    """
    if not history:
        return 0.0, 0
    by_lane: dict[str, Any] = history.get("lanes") or {}
    delta = 0.0
    learned_count = 0
    for lane in lanes:
        entry = by_lane.get(lane["id"])
        if not isinstance(entry, dict) or not entry.get("learned"):
            continue
        p50 = entry.get("p50")
        floor = entry.get("static_floor")
        if not isinstance(p50, (int, float)):
            continue
        learned_estimate = float(p50) * 1.15
        if isinstance(floor, (int, float)) and float(floor) > learned_estimate:
            new_lem = float(floor)
            source = "static_floor"
        else:
            new_lem = learned_estimate
            source = "learned (p50 * 1.15)"
        old_lem = lane.get("base_lem")
        if isinstance(old_lem, (int, float)):
            delta += new_lem - float(old_lem)
        lane["base_lem"] = new_lem
        lane["learned"] = True
        lane["learned_source"] = source
        lane["p90_warning"] = entry.get("p90")
        lane["p95_hard_planning"] = entry.get("p95")
        learned_count += 1
    return delta, learned_count


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
        "| Lane | Runner | Base LEM | Blocking | Origin |",
        "|---|---|---:|:---:|---|",
    ]
    for lane in plan["selection"]["lanes"]:
        base = lane.get("base_lem")
        if base is None:
            base = "—"
        origin = ", ".join(f"`{o}`" for o in lane.get("origin", []))
        lines.append(
            f"| `{lane['id']}` | {lane['runner']} | {base} | "
            f"{'✓' if lane['blocking'] else ''} | {origin} |"
        )

    trust_lanes = plan.get("trust_lanes") or {}
    strongest = trust_lanes.get("strongest_class")
    if strongest:
        lines.append("")
        lines.append("## Trust lane (advisory)")
        lines.append("")
        lines.append(
            f"**Strongest class:** `{strongest['id']}` "
            f"(risk rank `{strongest.get('risk_rank', '?')}`)"
        )
        classes = trust_lanes.get("classes") or []
        class_ids = [entry.get("id") for entry in classes if entry.get("id")]
        if class_ids:
            lines.append(f"**Matched classes:** {', '.join(f'`{c}`' for c in class_ids)}")
        lines.append("")
        lines.append(str(strongest.get("claim_boundary", "")))
        changed_surface = trust_lanes.get("changed_surface") or []
        if changed_surface:
            lines.append("")
            lines.append("Changed surface:")
            for surface in changed_surface:
                lines.append(f"- {surface}")
        required = strongest.get("required_checks") or []
        if required:
            lines.append("")
            lines.append("Required proof:")
            for item in required:
                lines.append(f"- {item}")
        skipped_checks = strongest.get("skipped_by_policy_checks") or []
        if skipped_checks:
            lines.append("")
            lines.append("Skipped by policy:")
            for item in skipped_checks:
                lines.append(f"- {item}")
        widening = strongest.get("widening_triggers") or []
        if widening:
            lines.append("")
            lines.append("Widen if:")
            for item in widening:
                lines.append(f"- {item}")
        support_impact = strongest.get("support_claim_impact")
        if support_impact:
            lines.append("")
            lines.append(f"Support claim impact: {support_impact}")

    skipped = plan["selection"].get("skipped_lanes") or []
    if skipped:
        lines.append("")
        lines.append("## Considered but skipped")
        lines.append("")
        lines.append("Lanes that would have been selected but are skipped because their")
        lines.append("`paths:` filter didn't match this diff.")
        lines.append("")
        lines.append("| Lane | Reason | Origin |")
        lines.append("|---|---|---|")
        for lane in skipped:
            origin = ", ".join(f"`{o}`" for o in lane.get("origin", []))
            reason = lane.get("skipped_reason", "?")
            lines.append(f"| `{lane['id']}` | {reason} | {origin} |")

    if plan["selection"]["risk_packs"]:
        lines.append("")
        lines.append("## Risk packs")
        lines.append("")
        for p in plan["selection"]["risk_packs"]:
            lines.append(f"- `{p}`")

    # Highlight ripr's role explicitly: contributors should be able to see
    # whether ripr ran on this PR and why.
    ripr_lane = next(
        (l for l in plan["selection"]["lanes"] if l["id"] == "ripr_advisory"), None
    )
    ripr_skipped = next(
        (l for l in skipped if l["id"] == "ripr_advisory"), None
    )
    if ripr_lane or ripr_skipped:
        lines.append("")
        lines.append("## ripr (advisory)")
        lines.append("")
        if ripr_lane:
            origin = ", ".join(f"`{o}`" for o in ripr_lane.get("origin", []))
            lines.append(f"Selected — origin: {origin}")
        else:
            lines.append("Skipped — no production Rust paths changed.")
        lines.append("")
        lines.append(
            "ripr is mutation-testing-lite at static-analysis prices. It is "
            "**advisory**; it does not block merges. See "
            "[`docs/ci/ripr.md`](../blob/master/docs/ci/ripr.md)."
        )

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
    parser.add_argument("--base", default=os.environ.get("BASE_SHA", "origin/main"))
    parser.add_argument("--head", default=os.environ.get("HEAD_SHA", "HEAD"))
    parser.add_argument("--labels-json", default="[]")
    parser.add_argument("--budget", type=Path, default=Path("policy/ci-budget.toml"))
    parser.add_argument("--lanes", type=Path, default=Path("policy/ci-lanes.toml"))
    parser.add_argument(
        "--risk-packs", type=Path, default=Path("policy/ci-risk-packs.toml")
    )
    parser.add_argument(
        "--trust-lanes", type=Path, default=Path("policy/trust-lanes.toml")
    )
    parser.add_argument(
        "--json-out", type=Path, default=Path("target/ci/ci-plan.json")
    )
    parser.add_argument(
        "--history",
        type=Path,
        default=Path(".ci/metrics/ci-lane-history.json"),
        help="Optional learned-LEM history (PR 16). Falls back to static "
        "base_lem when missing or sparse.",
    )
    parser.add_argument("--summary", type=str, default=os.environ.get("GITHUB_STEP_SUMMARY"))
    args = parser.parse_args()

    budget_doc = read_toml(args.budget)
    lanes_doc = read_toml(args.lanes)
    risk_packs_doc = read_toml(args.risk_packs)
    trust_lanes_doc = read_toml(args.trust_lanes)

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
    selected_lanes, skipped_lanes = select_lanes(
        files=files,
        labels=labels,
        risk_pack_ids=selected_packs,
        risk_packs=risk_packs,
        lanes=lanes,
    )

    # Apply learned LEM estimates from .ci/metrics/ci-lane-history.json when
    # the file is present and the lane has enough samples. Falls back to the
    # static base_lem when history is absent or sparse.
    history = load_learned_history(args.history)
    learned_delta, learned_count = apply_learned_estimates(selected_lanes, history)

    estimated_lem = sum(lane_lem(lane, multipliers) for lane in selected_lanes)
    rate = float(budget.get("linux_minute_rate_usd", 0.008))

    warnings: list[str] = []
    band = band_for(estimated_lem, budget)
    label_set = {l.lower() for l in labels}
    has_ack = "ci-budget-ack" in label_set or "full-ci" in label_set
    has_override = "ci-budget-override" in label_set or "full-ci" in label_set

    over_ceiling_failure = False
    if band == "elevated":
        warnings.append(
            f"Estimated LEM {estimated_lem:.1f} is in the *elevated* band "
            f"(>{budget.get('default_limit_lem', 35)}). Consider whether all "
            "selected lanes are needed."
        )
    elif band == "high":
        if has_ack:
            warnings.append(
                f"Estimated LEM {estimated_lem:.1f} is in the *high* band; "
                "acknowledged via `ci-budget-ack` (or `full-ci`)."
            )
        else:
            warnings.append(
                f"Estimated LEM {estimated_lem:.1f} is in the *high* band. "
                "Apply `ci-budget-ack` if this spend is intentional."
            )
    elif band == "over_ceiling":
        if has_override:
            warnings.append(
                f"Estimated LEM {estimated_lem:.1f} exceeds hard ceiling "
                f"({budget.get('hard_limit_lem', 125)}); explicitly overridden "
                "via `ci-budget-override` (or `full-ci`)."
            )
        else:
            over_ceiling_failure = True
            warnings.append(
                f"::error::Estimated LEM {estimated_lem:.1f} exceeds hard "
                f"ceiling ({budget.get('hard_limit_lem', 125)}). Apply "
                "`ci-budget-override` or `full-ci` to acknowledge, or trim "
                "selected lanes."
            )

    trust_lanes = classify_trust_lanes(
        files,
        trust_lanes_doc,
        estimated_lem=estimated_lem,
        band=band,
        selected_lanes=selected_lanes,
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
            "skipped_lanes": skipped_lanes,
        },
        "trust_lanes": trust_lanes,
        "warnings": warnings,
        "guard": {
            "hard_ceiling_exceeded": band == "over_ceiling",
            "override_present": has_override,
            "ack_present": has_ack,
            "failed": over_ceiling_failure,
        },
        "learned": {
            "history_present": bool(history),
            "lanes_using_learned": learned_count,
            "delta_lem_vs_static": learned_delta,
        },
    }

    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(json.dumps(plan, indent=2) + "\n")

    if args.summary:
        summary_path = Path(args.summary)
        summary_path.parent.mkdir(parents=True, exist_ok=True)
        with summary_path.open("a", encoding="utf-8") as f:
            f.write(render_summary(plan))

    print(json.dumps({"estimated_lem": estimated_lem, "band": band, "lanes": len(selected_lanes)}))
    if over_ceiling_failure:
        return 2  # distinct from generic error so workflow can branch on it
    return 0


if __name__ == "__main__":
    sys.exit(main())
