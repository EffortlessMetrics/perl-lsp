# 2026-05-03 — `RELEASE_HISTORY.md` Ledger Drift Blocks Master CI for All PRs

**Lens**: When a published version is missing from `RELEASE_HISTORY.md`, the drift gate (`just ci-release-history` / `ci-release-history-check`) fails on *every* PR opened against master, not just release PRs. Six PRs would have been blocked simultaneously if more had been opened.

## What blocked

After v0.13.3 published (release-orchestration run `25274134830` SUCCESS), the next PR opened against master was `#7874` (managed-install regression locks). PR Smoke and CI Gate (Merge-Blocking) both failed with:

```
$ bash scripts/check_release_history.sh
ERROR: Missing RELEASE_HISTORY.md entry for 0.13.3
Release history drift detected.
$ just ci-release-history → exit 1
$ just ci-release-history-check → exit 1
```

The PR's diff was test-only (downloader.test.ts). It did not touch `RELEASE_HISTORY.md`. The failure was rooted in master state, not the PR's diff.

`#7874` would have been blocked indefinitely; so would `#7877` (the gitignore PR), and any other PR opened against master while the ledger was stale.

## Why the row was missing

The v0.13.3 release-prep PR (`#7872`) ran `cargo xtask bump-version 0.13.3`, which updates 42 version sites across 7 files (Cargo.toml, Cargo.lock, features.toml, vscode-extension/package.json + lock, crates/perl-module/Cargo.toml, CLAUDE.md, docs/project/ROADMAP.md). It did *not* touch `RELEASE_HISTORY.md`.

Looking at the prior 0.13.2 release-prep PR (`#7851`): it *did* include a `RELEASE_HISTORY.md` row update. But the v0.13.3 release-prep was authored independently from the bump output, and the row was simply forgotten.

The bump tool doesn't know about the ledger. The release-orchestration doesn't update the ledger. The release-prep PR author has to remember.

## The fix (immediate)

PR `#7876` added the missing row + 4 link refs (notes, version, GitHub release, compare range), matching the schema of surrounding entries. After it merged, `bash scripts/check_release_history.sh` returned `Release history drift check passed` and `#7874` + `#7877` could be rebased and merged.

## Why this is master-state-broken, not "release fails"

The gate runs on every PR's CI. A stale ledger doesn't fail the *release* — it fails *all subsequent PRs*. This is a particularly wide blast radius: contributors not involved with the release see their PRs failing on a gate they didn't touch.

Drift gates are great defensive infrastructure (they prevent silent ledger rot). But they make release-prep PRs **unsplittable** — anything that unbundles the version bump from the ledger update creates a master-CI tripwire.

## The fix (structural)

Two options to prevent recurrence:

**Option A: orchestration auto-appends the row after publish.**

`release-orchestration.yml` adds a final step that opens a small PR adding the `RELEASE_HISTORY.md` row. Requires `pull-requests: write` permission on the workflow's GitHub token. Window of master-CI-broken between publish and the auto-PR landing — typically a few minutes.

**Option B: release-prep generator owns the row.**

Extend `cargo xtask bump-version` to also append the `RELEASE_HISTORY.md` row. Then any release-prep PR (which uses `bump-version`) automatically includes the row. No window of broken master.

**Option C: release finalization fails if ledger row missing.**

Add a check to `release-orchestration.yml` that fails the publish if `RELEASE_HISTORY.md` does not contain the version being released. Trip-wire — strong invariant, but trips at the latest possible point.

Recommended priority (different from the initial intuition):

1. **Option B + Option C together**. The generator makes correctness easy; the trip-wire prevents skipping. This is the strongest combination.
2. **Option A alone**. Convenient but reintroduces the window of broken master between publish and auto-PR.

The follow-up issue worth filing:

```
fix(release): bump-version appends RELEASE_HISTORY row + release-orchestration verifies ledger before publish
```

## Detection signal

If multiple unrelated PRs simultaneously fail PR Smoke / CI Gate on `release_history` or `release_history_check` gates after a release lands, the ledger is stale. Check:

```bash
git fetch origin master
grep "v$LATEST_VERSION" RELEASE_HISTORY.md  # should match a row
```

If the version isn't in the ledger, file a small fix PR adding the row before any other work proceeds. The fix PR will itself fail the same gate; that's expected — it's the PR that fixes the gate. Use `--admin` to merge if needed since the PR's own diff is what makes the gate pass.

## Lesson

Drift gates protect strongly *and* spread blast radius. They make any unbundling a master-CI tripwire. Either bundle invariants enforceably (Option B), or instrument the gate to be self-recovering (Option A), or fail fast at the join-point (Option C). Don't rely on humans remembering to update both halves of a pair.

## Related

- Articles: `../articles/RELEASES_FAIL_AT_SEAMS.md` (this is the bundling-drift seam)
- Reference: `../reference/RELEASE_PROOF_PROTOCOL.md` (ledger update is now in the pre-release checks)
- Reference: `../reference/FAILURE_CLASSIFICATION.md` (workflow class — workflow doesn't enforce the bundling invariant)
- The fix that landed: `#7876` (`fix(release): add v0.13.3 to RELEASE_HISTORY ledger`)
