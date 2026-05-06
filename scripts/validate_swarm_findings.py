#!/usr/bin/env python3
"""Validate the tracked swarm findings ledger."""

from __future__ import annotations

import json
import re
import sys
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FINDINGS_PATH = ROOT / ".claude" / "swarm-state" / "findings.json"

ALLOWED_ROOT_KEYS = {"_comment", "schema_version", "last_updated", "findings"}
ALLOWED_FINDING_KEYS = {
    "id",
    "kind",
    "status",
    "recorded_on",
    "summary",
    "decision",
    "surfaces",
    "evidence",
    "follow_up",
    "notes",
}
ALLOWED_EVIDENCE_KEYS = {"type", "ref"}
ALLOWED_KINDS = {
    "control_plane",
    "runtime_invariant",
    "docs_drift",
    "workflow_gap",
    "tracking_gap",
}
ALLOWED_STATUSES = {"active", "landed", "watching", "superseded"}
ALLOWED_EVIDENCE_TYPES = {"file", "pr", "issue", "doc", "hook", "setting"}
FINDING_ID_RE = re.compile(r"^SWARM-FINDING-[0-9]{4}$")


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_date(raw: str, field: str) -> date:
    try:
        return date.fromisoformat(raw)
    except ValueError as exc:
        fail(f"{field} must be ISO date YYYY-MM-DD: {exc}")
    raise AssertionError("unreachable")


def ensure_nonempty_string(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        fail(f"{field} must be a non-empty string")
    return value


def resolve_findings_path() -> Path:
    if len(sys.argv) > 2:
        fail("usage: validate_swarm_findings.py [path/to/findings.json]")
    if len(sys.argv) == 2:
        return Path(sys.argv[1]).resolve()
    return DEFAULT_FINDINGS_PATH


def main() -> None:
    findings_path = resolve_findings_path()
    try:
        raw = findings_path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        fail(f"findings ledger not found: {exc.filename}")
    data = json.loads(raw)

    extra_root = set(data) - ALLOWED_ROOT_KEYS
    if extra_root:
        fail(f"unexpected root keys: {sorted(extra_root)}")

    if data.get("schema_version") != 1:
        fail("schema_version must be 1")

    last_updated = parse_date(ensure_nonempty_string(data.get("last_updated"), "last_updated"), "last_updated")

    findings = data.get("findings")
    if not isinstance(findings, list):
        fail("findings must be an array")

    seen_ids: set[str] = set()
    seen_surfaces: set[str] = set()
    latest_recorded = date.min

    for index, finding in enumerate(findings):
        if not isinstance(finding, dict):
            fail(f"finding #{index + 1} must be an object")

        extra_finding = set(finding) - ALLOWED_FINDING_KEYS
        if extra_finding:
            fail(f"finding #{index + 1} has unexpected keys: {sorted(extra_finding)}")

        finding_id = ensure_nonempty_string(finding.get("id"), f"finding #{index + 1}.id")
        if not FINDING_ID_RE.fullmatch(finding_id):
            fail(f"{finding_id} must match SWARM-FINDING-####")
        if finding_id in seen_ids:
            fail(f"duplicate finding id: {finding_id}")
        seen_ids.add(finding_id)

        kind = ensure_nonempty_string(finding.get("kind"), f"{finding_id}.kind")
        if kind not in ALLOWED_KINDS:
            fail(f"{finding_id}.kind must be one of {sorted(ALLOWED_KINDS)}")

        status = ensure_nonempty_string(finding.get("status"), f"{finding_id}.status")
        if status not in ALLOWED_STATUSES:
            fail(f"{finding_id}.status must be one of {sorted(ALLOWED_STATUSES)}")

        recorded_on = parse_date(
            ensure_nonempty_string(finding.get("recorded_on"), f"{finding_id}.recorded_on"),
            f"{finding_id}.recorded_on",
        )
        latest_recorded = max(latest_recorded, recorded_on)

        ensure_nonempty_string(finding.get("summary"), f"{finding_id}.summary")
        ensure_nonempty_string(finding.get("decision"), f"{finding_id}.decision")

        surfaces = finding.get("surfaces")
        if not isinstance(surfaces, list) or not surfaces:
            fail(f"{finding_id}.surfaces must be a non-empty array")
        for surface_index, surface in enumerate(surfaces):
            surface_value = ensure_nonempty_string(surface, f"{finding_id}.surfaces[{surface_index}]")
            seen_surfaces.add(surface_value)

        evidence = finding.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            fail(f"{finding_id}.evidence must be a non-empty array")
        for evidence_index, item in enumerate(evidence):
            if not isinstance(item, dict):
                fail(f"{finding_id}.evidence[{evidence_index}] must be an object")
            extra_evidence = set(item) - ALLOWED_EVIDENCE_KEYS
            if extra_evidence:
                fail(f"{finding_id}.evidence[{evidence_index}] has unexpected keys: {sorted(extra_evidence)}")
            evidence_type = ensure_nonempty_string(item.get("type"), f"{finding_id}.evidence[{evidence_index}].type")
            if evidence_type not in ALLOWED_EVIDENCE_TYPES:
                fail(
                    f"{finding_id}.evidence[{evidence_index}].type must be one of {sorted(ALLOWED_EVIDENCE_TYPES)}"
                )
            ensure_nonempty_string(item.get("ref"), f"{finding_id}.evidence[{evidence_index}].ref")

        follow_up = finding.get("follow_up")
        if not isinstance(follow_up, list):
            fail(f"{finding_id}.follow_up must be an array")
        for follow_up_index, step in enumerate(follow_up):
            ensure_nonempty_string(step, f"{finding_id}.follow_up[{follow_up_index}]")

        notes = finding.get("notes")
        if notes is not None:
            ensure_nonempty_string(notes, f"{finding_id}.notes")

    if latest_recorded > last_updated:
        fail("last_updated must be on or after the newest finding.recorded_on date")

    if findings and not seen_surfaces:
        fail("at least one surface must be referenced across findings")

    print(f"Validated {len(findings)} findings in {findings_path}")


if __name__ == "__main__":
    main()
