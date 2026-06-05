#!/usr/bin/env python3
"""CI check-run classifier for perl-lsp-swarm — REPORT ONLY, exits 0.

Classifies each FAILING CI check-run into exactly one of 7 classes,
with a one-line rationale and routing recommendation.

HONEST CAVEAT: This is a fresh mirror of perl-lsp-swarm; the lane-history
corpus has 0 recorded samples at time of writing.  The taxonomy is structural
(derived from gate-policy.yaml, trust-lanes.toml, and CI workflow structure),
not learned from historical failure data.  Classification confidence improves
as lane-history is populated by aggregate_lane_history.py.

Classes
-------
product_defect
    Gate fails, quarantine=false, retries exhausted, in a unit/LSP/corpus
    gate.  Routing: needs-builder-fix → builder.

coverage_artifact
    quarantine=true OR required=false; or a baseline-drift failure.
    Routing: log & ignore; do not block merge.

infra_issue
    conclusion=cancelled (concurrency kill) or conclusion=timed_out near the
    gate's timeout boundary.
    Routing: retry once; escalate if stable across retries.

policy_mismatch
    Gate in the mechanical-correctness set {fmt, check_conflict_markers,
    layer_check, publish_manifest_check, v2_bundle_sync, nested_lock_check};
    quarantine=false.
    Routing: needs-builder-fix → pr-responder (mechanical).

review_gate
    draft-pr-check run_ci=false; preflight-latest-check is_latest=false.
    Routing: ignore — expected path.

expected_path_skip
    quarantine=true in gate-policy; ux-flakes.json active entry for this
    check name; Windows-scope required=false.
    Routing: ignore — policy-sanctioned.

unknown
    No pattern matches.
    Routing: human triage; do not merge.

Input
-----
JSON file path (positional) or stdin.  A JSON array of check-run objects,
each with at least {"name": str, "conclusion": str}.  Optional fields are
handled with safe defaults:
  required      bool  default True
  quarantine    bool  default False
  run_ci        bool  default True
  is_latest     bool  default True

Output
------
Human-readable table to stdout, one row per failing check.  Exits 0 always
(report-only; never blocks the caller).

Optional thin GitHub wrapper (--pr N) fetches live check-runs via the gh CLI
but is not required — offline fixture testing is the primary path.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Class taxonomy
# ---------------------------------------------------------------------------

# The 7 classification labels — order determines nothing; priority is encoded
# in classify_one() via explicit elif chains.
CLASS_PRODUCT_DEFECT = "product_defect"
CLASS_COVERAGE_ARTIFACT = "coverage_artifact"
CLASS_INFRA_ISSUE = "infra_issue"
CLASS_POLICY_MISMATCH = "policy_mismatch"
CLASS_REVIEW_GATE = "review_gate"
CLASS_EXPECTED_PATH_SKIP = "expected_path_skip"
CLASS_UNKNOWN = "unknown"

# Gate names that signal a mechanical policy/style failure (policy_mismatch).
POLICY_GATE_NAMES: frozenset[str] = frozenset(
    {
        "fmt",
        "check_conflict_markers",
        "conflict-markers",
        "layer_check",
        "publish_manifest_check",
        "v2_bundle_sync",
        "nested_lock_check",
    }
)

# Substrings of check names whose failure implies a unit/LSP/corpus product
# defect (product_defect) when quarantine=false and required=true.
# NOTE: security_audit, parser_corpus_ratchet, cpan_corpus_ratchet are
# intentionally EXCLUDED here — they are measurement/quality gates (coverage_artifact
# per spec acceptance tests 3 and 8), not product defect indicators.
PRODUCT_GATE_SUBSTRINGS: tuple[str, ...] = (
    "CI Gate shard",
    "CI Gate (Merge-Blocking)",
    "UX Regression Tests",
    "Compile All Targets",
    "LSP Memory Smoke",
    "pr-smoke",
    "pr_smoke",
    "Detect Flaky Tests",
    "lsp_smoke",
    "common_corpus_clean",
)

# Gate name substrings that identify measurement/quality gates.  Failures of
# these gates that carry quarantine=true or required=false are coverage_artifact
# (environmental drift, broken tooling) rather than expected_path_skip (policy-
# sanctioned skip) or product_defect.
# Source: issue #907 taxonomy table + acceptance tests 3 and 8.
COVERAGE_GATE_SUBSTRINGS: tuple[str, ...] = (
    "security_audit",
    "parser_corpus_ratchet",
    "cpan_corpus_ratchet",
    "mutation",
    "fuzz",
    "benchmarks",
    "published_crate_count",
    "coverage",
    "baseline",
)

# Names that are review-gate signals (draft / superseded-SHA skip).
REVIEW_GATE_NAMES: frozenset[str] = frozenset(
    {
        "draft-pr-check",
        "preflight-latest-check",
    }
)

# Conclusions that suggest an infrastructure failure.
INFRA_CONCLUSIONS: frozenset[str] = frozenset({"cancelled", "timed_out"})

ROUTING: dict[str, str] = {
    CLASS_PRODUCT_DEFECT: "needs-builder-fix → builder",
    CLASS_COVERAGE_ARTIFACT: "log & ignore; do not block merge",
    CLASS_INFRA_ISSUE: "retry once; escalate if stable across retries",
    CLASS_POLICY_MISMATCH: "needs-builder-fix → pr-responder (mechanical)",
    CLASS_REVIEW_GATE: "ignore — expected path",
    CLASS_EXPECTED_PATH_SKIP: "ignore — policy-sanctioned",
    CLASS_UNKNOWN: "human triage; do not merge",
}


# ---------------------------------------------------------------------------
# Pure classifier (testable offline)
# ---------------------------------------------------------------------------


def classify_one(check: dict[str, Any]) -> tuple[str, str]:
    """Classify a single check-run dict.

    Returns (class_label, rationale).  Always returns a valid class label,
    defaulting to ``unknown`` when no pattern matches.

    Missing optional fields are handled via .get() with safe defaults so the
    function never raises on partial input.
    """
    name: str = str(check.get("name", ""))
    conclusion: str = str(check.get("conclusion", ""))
    quarantine: bool = bool(check.get("quarantine", False))
    required: bool = bool(check.get("required", True))
    run_ci: bool = bool(check.get("run_ci", True))
    is_latest: bool = bool(check.get("is_latest", True))

    # 1. review_gate — draft or superseded-SHA: check even before looking at
    #    conclusion because these checks may have conclusion=skipped or neutral.
    if name in REVIEW_GATE_NAMES:
        if name == "draft-pr-check" and not run_ci:
            return (
                CLASS_REVIEW_GATE,
                "draft-pr-check: run_ci=false — PR is a draft; expected skip",
            )
        if name == "preflight-latest-check" and not is_latest:
            return (
                CLASS_REVIEW_GATE,
                "preflight-latest-check: is_latest=false — superseded SHA; expected skip",
            )
        # If neither sub-condition triggered, fall through to normal checks.

    # 2. coverage_artifact / expected_path_skip — quarantine=true or required=false
    #    are policy signals that SUPERSEDE the physical failure mode (infra_issue).
    #    A quarantined gate that also times out is still policy-sanctioned; routing
    #    it as infra_issue would cause incorrect retry of a deliberately-suppressed gate.
    #
    #    Discriminate between the two skip flavours:
    #    - coverage_artifact: the gate is a measurement/quality tool whose threshold
    #      drifted or tooling broke (e.g. security_audit broken by CVSS 4.0).
    #    - expected_path_skip: the gate is intentionally not exercised (windows path
    #      filter, quarantined test with #[ignored], etc.).
    if quarantine or not required:
        if _name_matches_coverage_gate(name):
            return (
                CLASS_COVERAGE_ARTIFACT,
                f"{name!r} is a measurement/quality gate; "
                f"{'quarantine=true' if quarantine else 'required=false'} — "
                "environmental drift or broken tooling, not a product regression",
            )
        return (
            CLASS_EXPECTED_PATH_SKIP,
            f"{'quarantine=true' if quarantine else 'required=false'} in gate-policy "
            "— policy-sanctioned non-blocking skip",
        )

    # 3. infra_issue — conclusion=cancelled or timed_out (only for non-quarantined gates)
    if conclusion in INFRA_CONCLUSIONS:
        return (
            CLASS_INFRA_ISSUE,
            f"conclusion={conclusion!r} — concurrency kill or timeout boundary hit",
        )

    # 4. coverage_artifact — conclusion=skipped or neutral when NOT quarantined:
    #    baseline-drift / coverage check with required=false already caught
    #    above; catch remaining coverage-flavoured skips here.
    if conclusion in ("skipped", "neutral"):
        lower = name.lower()
        if any(kw in lower for kw in ("coverage", "baseline", "drift")):
            return (
                CLASS_COVERAGE_ARTIFACT,
                f"conclusion={conclusion!r} on coverage/baseline check — artifact, not defect",
            )

    # 5. policy_mismatch — mechanical-correctness gate
    if _name_matches_policy_gate(name):
        return (
            CLASS_POLICY_MISMATCH,
            f"{name!r} is a mechanical-correctness gate — formatting or policy violation",
        )

    # 6. product_defect — core unit/LSP/corpus gate failing
    if _name_matches_product_gate(name):
        return (
            CLASS_PRODUCT_DEFECT,
            f"{name!r} is a core product gate — test/LSP/corpus failure, quarantine=false",
        )

    # 7. coverage_artifact — coverage gates that aren't quarantined but are
    #    non-required (catch-all for remaining coverage-flavoured failures)
    lower = name.lower()
    if any(kw in lower for kw in ("coverage", "baseline", "drift", "mutation", "fuzz")):
        return (
            CLASS_COVERAGE_ARTIFACT,
            f"{name!r} matches coverage/quality keyword — likely baseline-drift artifact",
        )

    # 8. unknown — no pattern matched
    return (
        CLASS_UNKNOWN,
        f"no classification pattern matched for {name!r} (conclusion={conclusion!r})",
    )


def _name_matches_policy_gate(name: str) -> bool:
    """Return True if the check name maps to a mechanical policy gate."""
    # Exact match against the policy set.
    if name in POLICY_GATE_NAMES:
        return True
    # Substring check for GitHub Actions job names that embed the gate name.
    lower = name.lower()
    return any(gate in lower for gate in POLICY_GATE_NAMES)


def _name_matches_product_gate(name: str) -> bool:
    """Return True if the check name maps to a core product gate."""
    lower = name.lower()
    return any(sub.lower() in lower for sub in PRODUCT_GATE_SUBSTRINGS)


def _name_matches_coverage_gate(name: str) -> bool:
    """Return True if the check name maps to a measurement/quality gate.

    Coverage gates are distinct from product gates: their failures indicate
    environmental drift (threshold changes, broken tooling) rather than
    product regressions.  Spec source: issue #907 taxonomy + acceptance tests 3 and 8.
    """
    lower = name.lower()
    return any(sub.lower() in lower for sub in COVERAGE_GATE_SUBSTRINGS)


# ---------------------------------------------------------------------------
# Input / output
# ---------------------------------------------------------------------------


def load_check_runs(source: str | None) -> list[dict[str, Any]]:
    """Load check-run JSON from a file path or stdin.

    Returns a list of dicts.  Tolerant of both a bare list and a GitHub API
    envelope with a ``check_runs`` key.
    """
    if source is None:
        raw = sys.stdin.read()
    else:
        raw = Path(source).read_text(encoding="utf-8")

    doc = json.loads(raw)

    # Unwrap GitHub API envelope if present.
    if isinstance(doc, dict):
        doc = doc.get("check_runs", doc.get("check_suite", doc))
        if isinstance(doc, dict):
            # Still a dict — caller may have passed a single check-run.
            doc = [doc]

    if not isinstance(doc, list):
        print(
            f"ERROR: expected a JSON array of check-runs, got {type(doc).__name__}",
            file=sys.stderr,
        )
        return []

    return doc


def filter_failing(check_runs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Return only check-runs with a failing conclusion.

    Failing conclusions: failure, timed_out, cancelled, action_required.
    Skipped/neutral/success are excluded unless explicitly flagged.
    """
    failing_conclusions = {"failure", "timed_out", "cancelled", "action_required"}
    return [c for c in check_runs if c.get("conclusion") in failing_conclusions]


def fetch_check_runs_via_gh(pr_number: int) -> list[dict[str, Any]]:
    """Fetch check-runs for a PR via the gh CLI.

    This is a nice-to-have thin wrapper.  Returns an empty list on any error
    so the caller can degrade gracefully.
    """
    try:
        result = subprocess.run(
            [
                "gh",
                "pr",
                "checks",
                str(pr_number),
                "--json",
                "name,conclusion,required",
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(
                f"WARNING: gh pr checks returned non-zero: {result.stderr.strip()}",
                file=sys.stderr,
            )
            return []
        return json.loads(result.stdout)
    except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
        print(f"WARNING: could not fetch check-runs via gh: {exc}", file=sys.stderr)
        return []


def format_results(results: list[tuple[dict[str, Any], str, str]]) -> str:
    """Render classification results as a human-readable report."""
    if not results:
        return "No failing check-runs to classify.\n"

    lines: list[str] = []
    lines.append(
        f"{'CHECK NAME':<50} {'CLASS':<22} {'ROUTING':<42} RATIONALE"
    )
    lines.append("-" * 160)

    for check, cls, rationale in results:
        name = str(check.get("name", ""))
        routing = ROUTING.get(cls, "")
        lines.append(f"{name:<50} {cls:<22} {routing:<42} {rationale}")

    lines.append("")
    # Summary counts.
    class_counts: dict[str, int] = {}
    for _, cls, _ in results:
        class_counts[cls] = class_counts.get(cls, 0) + 1
    lines.append("Summary:")
    for cls, count in sorted(class_counts.items()):
        lines.append(f"  {cls}: {count}")

    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def run(args: argparse.Namespace) -> int:
    """Main logic; returns exit code (always 0 — report only)."""
    # Determine check-run source.
    if args.pr is not None:
        check_runs = fetch_check_runs_via_gh(args.pr)
        if not check_runs:
            print("No check-runs retrieved for PR; nothing to classify.")
            return 0
    elif args.input:
        check_runs = load_check_runs(args.input)
    else:
        check_runs = load_check_runs(None)  # stdin

    failing = filter_failing(check_runs)

    results: list[tuple[dict[str, Any], str, str]] = []
    for check in failing:
        cls, rationale = classify_one(check)
        results.append((check, cls, rationale))

    if args.json:
        output = json.dumps(
            [
                {
                    "name": c.get("name", ""),
                    "conclusion": c.get("conclusion", ""),
                    "class": cls,
                    "rationale": rationale,
                    "routing": ROUTING.get(cls, ""),
                }
                for c, cls, rationale in results
            ],
            indent=2,
        )
        print(output)
    else:
        print(format_results(results), end="")

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "input",
        nargs="?",
        default=None,
        metavar="FILE",
        help="Path to JSON file of check-runs (default: stdin)",
    )
    parser.add_argument(
        "--pr",
        type=int,
        default=None,
        metavar="N",
        help="Fetch check-runs live from PR N via the gh CLI (optional)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON output instead of human-readable table",
    )
    args = parser.parse_args()
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
