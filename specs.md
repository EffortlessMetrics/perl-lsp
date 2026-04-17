# Release History Drift Check — Specification

## Feature Description

A CI script that detects drift between git tags, release note files, and the release ledger. It runs as part of the `policy_checks` gate and fails if:

1. A `v*` tag exists without a matching `docs/releases/v*.md` file (excluding `v*-rc*` prerelease tags)
2. A `v*` tag exists without a matching row in `RELEASE_HISTORY.md`
3. The newest release tag isn't in `CHANGELOG.md` as `## [X.Y.Z]`
4. `RELEASE_HISTORY.md` contains a notes file link for a version that has no corresponding tag (unless marked `(CL)`)

## Script Behavior

### Input
- Git tags matching `v*` (fetched via `git tag --list`)
- `docs/releases/` directory (note files)
- `RELEASE_HISTORY.md` (ledger)
- `CHANGELOG.md` (version headers)

### Output
- Exit 0 on success (no drift detected)
- Exit 1 on drift (with descriptive error messages)
- All output goes to stdout/stderr (no separate evidence file needed)

### Exemptions
- **Prerelease tags** (`v*-rc*`): Ignored entirely
- **CHANGELOG-only entries** (`(CL)` in Released column): No tag exists; these are scope markers, not releases. Examples: `v0.9.0`, `v0.10.0`, `v0.8.8`
- **Pre-existing gaps**: Tags with `—` in the Notes file column of `RELEASE_HISTORY.md` are grandfathered (e.g., `v0.7.2`, `v0.7.3`, `v0.8.0`, `v0.8.2`, `v0.5.0`, `v0.1.0-pest`)

### Algorithm

```
1. Collect all v* tags, strip leading 'v', exclude *-rc* patterns
2. For each tag:
   a. Check docs/releases/v<version>.md exists → FAIL if missing
   b. Check RELEASE_HISTORY.md contains version string → FAIL if missing
3. Find newest (highest) tag by version sort
   a. Check CHANGELOG.md contains "## [<newest_version>]" → FAIL if missing
4. For each version in RELEASE_HISTORY.md with a notes file link:
   a. If the version is NOT marked (CL) and has no tag → WARN (legacy gap, don't fail)
```

## Integration

### Gate Configuration
Add `bash scripts/check_release_history.sh` to the `policy_checks` gate chain in `.ci/gate-policy.yaml`:

```yaml
- name: policy_checks
  tier: merge_gate
  command: >-
    cargo xtask check-from-raw &&
    bash scripts/check-version-sync.sh &&
    bash scripts/check_release_history.sh &&   # <-- NEW
    bash ci/check_missing_docs.sh &&
    bash ci/check_parse_errors.sh &&
    python3 scripts/check_features_invariants.py
```

### Script Location
`scripts/check_release_history.sh` — follows the `scripts/check-version-sync.sh` naming pattern (though unlike that script, this one is self-contained, not an xtask wrapper).

## Acceptance Criteria

1. **Tag → notes file**: When a new `v*` tag (non-rc) is created, if `docs/releases/v<X.Y.Z>.md` does not exist, the script exits 1 with message "Missing release notes: docs/releases/v<X.Y.Z.md>"

2. **Tag → ledger row**: When a new `v*` tag is created, if `RELEASE_HISTORY.md` does not contain a row for that version, the script exits 1 with message "Missing RELEASE_HISTORY.md entry for <version>"

3. **Newest tag in CHANGELOG**: If the highest-version tag is not present in CHANGELOG.md as `## [X.Y.Z]`, the script exits 1 with message "Newest tag <tag> not found in CHANGELOG.md"

4. **Passes on current master**: Given the existing `(CL)` exemptions and pre-existing gap grandfathering, the script exits 0 on current master

5. **Output is actionable**: Error messages include the missing file/entry name so a developer knows exactly what to create

## Non-Goals

- **YAML front-matter validation**: The script only checks file existence, not schema correctness. A follow-up issue will add YAML validation via an xtask phase.
- **Auto-generation**: The script only detects drift; it does not create missing files or entries.
- **CHANGELOG content validation**: Only the `## [X.Y.Z]` header existence is checked; content quality is out of scope.
- **Tag creation validation**: The script does not prevent creation of tags; it only runs as a post-check in CI.

## Dependencies

- `git` (standard POSIX utilities: `git tag --list`, `grep`, `sed`, `sort -V`)
- No new external dependencies — pure shell script using utilities already available in CI