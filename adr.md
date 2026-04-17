# ADR-042: Release History Drift Check

**Status:** Proposed

## Context

The release history surface (`RELEASE_HISTORY.md`, `docs/releases/vX.Y.Z.md`, CHANGELOG sections) is manually maintained. There is no CI check to catch drift — a release can ship without updating these files.

The existing `policy_checks` gate (line 272 of `.ci/gate-policy.yaml`) already chains together policy/compliance checks: `check-version-sync.sh`, `check_missing_docs.sh`, `check_parse_errors.sh`, and `check_features_invariants.py`. Adding a release-history drift check to this chain follows the established pattern.

## Decision

We implement a standalone shell script `scripts/check_release_history.sh` that detects drift between:
1. Git tags and their `docs/releases/v*.md` files
2. Git tags and their `RELEASE_HISTORY.md` rows
3. The newest tag and its `CHANGELOG.md` entry
4. `RELEASE_HISTORY.md` notes file links and actual tag existence

The script is added to the `policy_checks` gate command chain (not a new standalone gate) because release-history drift is a compliance/policy issue, not a code quality issue.

### Exemption Mechanism

Pre-existing gaps are grandfathered via the existing `(CL)` convention in `RELEASE_HISTORY.md`. Entries marked `(CL)` (CHANGELOG-only) in the Released column have no tag and are explicitly exempt from the drift checks.

Tags with `—` in the Notes file column (e.g., `v0.7.2`, `v0.7.3`, `v0.8.0`, `v0.8.2`, `v0.5.0`, `v0.1.0-pest`) are also exempt — they have ledger rows but never had notes files. This is the established pre-existing state and the script uses this as its baseline.

### Tag Filtering

Only `v*` tags are checked. Tags matching `v*-rc*` (e.g., `v0.8.3-rc1`) are excluded because release candidates do not require release notes.

## Consequences

### Benefits
- Prevents drift from accumulating — new tags without release surface entries will fail the gate
- Uses existing `(CL)` exemption convention — no new mechanism needed
- Shell script is fast, portable, and easy to debug in CI

### Tradeoffs / Known Limitations
- Shell-only script cannot validate YAML front-matter schema in `docs/releases/v*.md` files — deferred to a follow-up xtask phase
- Pre-existing gaps are grandfathered — this is intentional (acceptance criteria: "passes cleanly on current master")

## Alternatives Considered

### 1. New standalone gate
Rejected: Adding a new gate entry increases gate count and management overhead. Release-history drift is a policy/compliance issue like the other `policy_checks`, so grouping it there is cleaner.

### 2. xtask subcommand
Rejected: The drift check is purely file-existence based. A Rust xtask adds compilation overhead and complexity without benefit for v1. YAML front-matter validation can be added later as a separate phase.

### 3. Hardcoded exempt tag list
Rejected: Using the `(CL)` convention in `RELEASE_HISTORY.md` is self-documenting and already formalized. A hardcoded list would be easier to forget to update for future CHANGELOG-only entries.