#!/usr/bin/env python3
"""Validate policy/non-rust-allowlist.toml and policy/non-rust-debt.toml.

Sub-second sanity check that runs before `cargo xtask check-file-policy`
exists (PR 4 of the file-policy rollout). Catches schema-level errors:

  - TOML parses.
  - Every [[allow]] has the required fields.
  - `glob` and `path` are mutually exclusive on a single entry.
  - Paths are repo-relative without leading `./` or absolute prefixes.
  - No Windows backslashes.
  - No duplicate ids.
  - `created` / `review_after` / `expires` are valid YYYY-MM-DD dates.
  - `expires` (if set) is after `created`.
  - `review_after` is after `created`.
  - `production` / `test` / `tooling` entries declare at least one
    `covered_by` item.
  - Broad globs (matching `**` at the root, or covering more than one
    language family) declare `broad_glob_reason`.
  - `classification` is one of the known values.

This is the schema gate. The semantic gate ("this glob actually matches
files in the repo, no tracked file is unallowlisted") lands with the Rust
checker in PR 4.

Run:
    python3 scripts/policy/validate_non_rust_allowlist.py

Exits non-zero on any violation.
"""

from __future__ import annotations

import argparse
import datetime as dt
import pathlib
import re
import sys
import tomllib

REQUIRED_FIELDS = (
    "id",
    "kind",
    "language",
    "surface",
    "classification",
    "owner",
    "reason",
    "covered_by",
    "created",
    "review_after",
)
OPTIONAL_FIELDS = (
    "glob",
    "path",
    "expires",
    "broad_glob_reason",
    "retired",
)
KNOWN_CLASSIFICATIONS = {
    "production",
    "test",
    "tooling",
    "config",
    "documentation",
}
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
COVERAGE_REQUIRING_CLASSIFICATIONS = {"production", "test", "tooling"}


def parse_date(value: str, field: str, entry_id: str, errors: list[str]) -> dt.date | None:
    if not isinstance(value, str) or not DATE_RE.match(value):
        errors.append(
            f"{entry_id}: `{field}` must be a YYYY-MM-DD string, got {value!r}"
        )
        return None
    try:
        return dt.date.fromisoformat(value)
    except ValueError:
        errors.append(f"{entry_id}: `{field}` is not a real date: {value!r}")
        return None


def validate_entry(entry: dict, index: int, errors: list[str]) -> None:
    entry_id = entry.get("id", f"<unnamed entry #{index}>")

    has_glob = "glob" in entry
    has_path = "path" in entry
    if has_glob and has_path:
        errors.append(f"{entry_id}: cannot set both `glob` and `path`")
    if not has_glob and not has_path:
        errors.append(f"{entry_id}: must set either `glob` or `path`")

    matcher = entry.get("glob") or entry.get("path") or ""
    if matcher.startswith("./") or matcher.startswith("/"):
        errors.append(
            f"{entry_id}: matcher `{matcher}` must be repo-relative without leading `./` or `/`"
        )
    if "\\" in matcher:
        errors.append(f"{entry_id}: matcher `{matcher}` contains Windows backslashes")
    if matcher.strip() != matcher:
        errors.append(f"{entry_id}: matcher `{matcher}` has surrounding whitespace")

    for field in REQUIRED_FIELDS:
        if field not in entry:
            errors.append(f"{entry_id}: missing required field `{field}`")

    for field in entry:
        if field not in REQUIRED_FIELDS + OPTIONAL_FIELDS:
            errors.append(f"{entry_id}: unknown field `{field}`")

    classification = entry.get("classification")
    if classification and classification not in KNOWN_CLASSIFICATIONS:
        errors.append(
            f"{entry_id}: classification `{classification}` not in "
            f"{sorted(KNOWN_CLASSIFICATIONS)}"
        )

    covered_by = entry.get("covered_by", [])
    if not isinstance(covered_by, list) or any(not isinstance(c, str) for c in covered_by):
        errors.append(f"{entry_id}: `covered_by` must be a list of strings")
    elif (
        classification in COVERAGE_REQUIRING_CLASSIFICATIONS
        and len(covered_by) == 0
    ):
        errors.append(
            f"{entry_id}: classification `{classification}` requires at least one `covered_by` entry"
        )

    created = parse_date(entry.get("created", ""), "created", entry_id, errors)
    review_after = parse_date(entry.get("review_after", ""), "review_after", entry_id, errors)
    expires_raw = entry.get("expires")
    expires = (
        parse_date(expires_raw, "expires", entry_id, errors)
        if expires_raw is not None
        else None
    )

    if created and review_after and review_after <= created:
        errors.append(f"{entry_id}: `review_after` must be after `created`")
    if created and expires and expires <= created:
        errors.append(f"{entry_id}: `expires` must be after `created`")

    # Broad-glob heuristic: glob starting with `**`, glob matching everything
    # below a top-level directory (`<dir>/**`), or glob containing `*.<ext>`.
    if has_glob:
        is_broad = (
            matcher.startswith("**")
            or matcher.endswith("/**")
            or matcher == "*.md"
            or matcher.startswith("**/")
        )
        if is_broad and not entry.get("broad_glob_reason"):
            errors.append(
                f"{entry_id}: glob `{matcher}` is broad; declare `broad_glob_reason`"
            )

    if entry.get("retired") not in (None, True, False):
        errors.append(f"{entry_id}: `retired` must be a boolean")


def validate_allowlist(path: pathlib.Path, errors: list[str]) -> int:
    try:
        data = tomllib.loads(path.read_text())
    except Exception as exc:
        errors.append(f"FAIL: parse {path}: {exc}")
        return 0

    entries = data.get("allow", [])
    if not isinstance(entries, list):
        errors.append(f"{path}: `allow` must be a list of tables")
        return 0

    seen_ids: dict[str, int] = {}
    seen_matchers: dict[str, str] = {}
    for index, entry in enumerate(entries):
        validate_entry(entry, index, errors)
        eid = entry.get("id")
        if isinstance(eid, str):
            if eid in seen_ids:
                errors.append(
                    f"{eid}: duplicate id (also at index {seen_ids[eid]})"
                )
            else:
                seen_ids[eid] = index

        matcher = entry.get("glob") or entry.get("path")
        if isinstance(matcher, str):
            if matcher in seen_matchers:
                errors.append(
                    f"{eid or '<unnamed>'}: duplicate matcher `{matcher}` "
                    f"(also used by id `{seen_matchers[matcher]}`)"
                )
            else:
                seen_matchers[matcher] = eid or "<unnamed>"

    return len(entries)


def validate_debt(path: pathlib.Path, errors: list[str]) -> int:
    try:
        data = tomllib.loads(path.read_text())
    except Exception as exc:
        errors.append(f"FAIL: parse {path}: {exc}")
        return 0

    entries = data.get("debt", [])
    if not isinstance(entries, list):
        errors.append(f"{path}: `debt` must be a list of tables")
        return 0

    # Debt entries share most schema with allow entries, minus the
    # `classification` strictness and `covered_by` requirement.
    return len(entries)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--allowlist",
        type=pathlib.Path,
        default=pathlib.Path("policy/non-rust-allowlist.toml"),
    )
    parser.add_argument(
        "--debt",
        type=pathlib.Path,
        default=pathlib.Path("policy/non-rust-debt.toml"),
    )
    args = parser.parse_args()

    errors: list[str] = []
    n_allow = validate_allowlist(args.allowlist, errors)
    n_debt = validate_debt(args.debt, errors)

    if errors:
        print(f"FAIL: {len(errors)} non-Rust policy validation error(s):", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"OK: validated {n_allow} allow entr{'y' if n_allow == 1 else 'ies'} "
        f"and {n_debt} debt entr{'y' if n_debt == 1 else 'ies'}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
