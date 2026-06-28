# perl-lsp Sync Protocol

`perl-lsp-swarm` is the active development source of truth. `perl-lsp` is the
release, history, and canonical package-lineage repo.

This protocol is control-plane documentation only. It does not change branch
protection, CI workflows, release automation, package publication, or provider
behavior.

## Source of Truth

| Repo | Authority |
|---|---|
| `perl-lsp-swarm/main` | Active development, agent lanes, proof receipts, spec hardening, promotion-ledger work, cleanup trains, compiler substrate work |
| `perl-lsp/master` | Release lineage, historical upstream, user-facing package lineage, emergency release fixes, curated sync target |

Default routing:

```text
new development -> perl-lsp-swarm
curated release sync -> perl-lsp
emergency release fix -> perl-lsp, then immediate swarm mirror/sync
```

## Swarm-First Promotion Policy

Do not mirror every `perl-lsp-swarm` PR into `perl-lsp`.

`perl-lsp-swarm` is the development and proof queue. `perl-lsp` is the release
surface. Old-repo PRs should be opened only at deliberate promotion points:

- release prep
- urgent install, setup, docs, or user-facing correction
- dependency or release blocker
- security fix
- settled batch of swarm changes
- post-release backport or channel repair

Do not open speculative implementation PRs in `perl-lsp`. Do not duplicate
active swarm work into the old repo while the swarm PR is still the canonical
review surface.

When the same work appears in both repositories:

1. Pick the canonical swarm PR or issue.
2. Preserve any useful commits, tests, review notes, or reproduction details by
   porting them back to the swarm surface when they are still relevant.
3. Close old-repo duplicates with a pointer to the canonical swarm PR or issue.
4. Do not treat the old-repo duplicate as a release sync unless it meets one of
   the deliberate promotion points above.

The goal is a smaller source-of-truth queue, not two queues carrying the same
development work.

## Hard Invariant

`perl-lsp/master` must not be ahead of `perl-lsp-swarm/main`.

Before merging any `perl-lsp` PR, one of these must be true:

1. The same change is already present in `perl-lsp-swarm/main`.
2. A swarm sync PR that includes the change is ready to merge first.
3. The change is an emergency release fix and a swarm mirror PR is opened before
   the old-repo PR is treated as complete.

If this invariant is uncertain, stop and sync swarm first.

## Sync Directions

### Swarm to perl-lsp

Use for curated release syncs.

Required before opening the old-repo sync PR:

- list included swarm PRs
- list excluded swarm PRs and why
- run the relevant support/provider/status checks in swarm
- preserve release notes and package-lineage docs
- state whether the sync changes user-facing package behavior

The old-repo PR body must link to the swarm source PRs and list verification.

#### Mechanics: history-preserving complete-tree merge

The two repos are **content-synced-divergent** — they share an old merge-base but
have since diverged with real work on both sides, so there is no clean
fast-forward. Use a **complete-tree merge**: take swarm's whole tree, but record
both histories via a 2-parent merge commit.

```bash
# in the perl-lsp checkout, on a fresh sync branch off origin/master:
git fetch swarm main                              # refresh the swarm remote-tracking ref
git checkout -B release/sync-vX.Y.Z origin/master
git merge -s ours --no-commit swarm/main          # record BOTH parents, keep our tree for now
git read-tree -u --reset swarm/main               # set the tree to swarm's COMPLETE content
# EXCLUDE swarm-internal harness (restore perl-lsp's own, drop swarm-only scripts):
git rm -rq .claude && git checkout origin/master -- .claude
git rm -q scripts/agent-cleanup.ps1 scripts/agent-preflight.ps1 scripts/swarm-clean
git add -A && git commit -m "release: history-preserving merge sync swarm/main (X.Y.Z) -> master"
```

**Why complete-tree, not per-file.** `git merge -X theirs` resolves *per file*,
mixing swarm's version of some files in a diverged crate with perl-lsp's version
of others — leaving the crate referencing methods that no longer exist (observed
on a 0.17.0 sync: `perl-dap` failed with E0599 ×6). `read-tree --reset
swarm/main` takes swarm's tree **whole**, so every crate is internally
consistent (swarm's tree already passed swarm CI). The `-s ours --no-commit`
step records both parents without letting the "ours" strategy keep perl-lsp's
tree.

**Verify before pushing to the canonical repo:**

- `git log -1 --format='%p' | wc -w` → **2** (both parents recorded).
- `git diff --name-only HEAD swarm/main` → only the EXCLUDE paths differ.
- `cargo check --workspace; echo "EXIT=$?"` → **EXIT=0**. Capture cargo's *own*
  exit — do **not** pipe cargo through `tail`/`head`, which replaces cargo's exit
  code with the pager's `0` and hides a real build failure.
- Confirm swarm/main is the *current* GitHub HEAD, not a stale local ref:
  `gh api repos/EffortlessMetrics/perl-lsp-swarm/commits/main --jq .sha`.

**EXCLUDE** (swarm-internal, not for the release repo): `.claude/` agent harness
(restore perl-lsp's own), swarm-only scripts (`agent-cleanup.ps1`,
`agent-preflight.ps1`, `swarm-clean`). **PRESERVE** perl-lsp release-lineage:
`docs/releases/vX.Y.*.md`, `RELEASE_HISTORY.md` (swarm carries these too once
synced, so the complete-tree merge keeps them).

### perl-lsp to Swarm

Use only for emergency release fixes or old-repo-only markers.

Required before merging old `perl-lsp` work:

- verify the same content exists in swarm, or merge a swarm mirror PR first
- keep the old-repo PR scoped to release-lineage work
- run the narrowest old-repo validation for the changed files
- run the corresponding swarm validation after the mirror

Source-to-swarm sync PRs should use merge commits when commit ancestry matters.
Do not squash source-sync PRs that are meant to prove `source/master` ancestry.

**Reconciling accumulated perl-lsp-unique work.** When `perl-lsp/master` has
drifted ahead with parallel work (release-lineage aside), identify the genuine
unique set with:

```bash
git cherry -v swarm/main origin/master | grep '^+' \
  | grep -ivE 'mirror|^queue:|Merge pull request|^release:'
```

In practice this set is dominated by **new files** (e.g. `test(ux): lock …`
behavior receipts), which port to swarm as additions rather than a
conflict-merge. Bring them back so swarm becomes the superset and the
[Hard Invariant](#hard-invariant) is restored. Note that a complete-tree
swarm→perl-lsp release sync intentionally takes swarm's whole tree, so any
perl-lsp-unique work not yet in swarm is dropped from the *release tree* (it
survives in history via the merge's perl-lsp parent) until this reverse step
lands it in swarm — do the reverse step, or accept that work ships a later
release.

## Cadence

Routine cadence:

```text
merge in swarm
batch into curated release sync
open old-repo sync PR
verify release-lineage gates
merge old-repo sync
tag or publish only with explicit release approval
```

Emergency cadence:

```text
open old-repo emergency PR
open or merge swarm mirror first
merge old-repo emergency PR only after the invariant is preserved
run post-merge support/provider/status checks
```

## Pre-Cut Verification (release repo)

The cut (tag + publish to crates.io / marketplace / Open VSX / Docker) is
**irreversible**. Before dispatching `release-orchestration.yml`, confirm on the
release repo (`perl-lsp`):

- The sync PR is merged; `perl-lsp/master` HEAD is the 2-parent sync commit at
  the target version (`[workspace.package].version` and `CHANGELOG.md` show a
  dated `## [X.Y.Z]`).
- Publish secrets are reachable where the cut runs. Repo-level `gh secret list`
  may show only a subset, and org/environment secrets are not listable without
  `admin:org`. The strongest evidence is a **prior successful release**:
  `gh run list --workflow=release-orchestration.yml` and
  `gh run list --workflow=publish-crates.yml` showing past `success` from
  `master` means the full chain (CARGO_REGISTRY_TOKEN, VSCE_PAT, OVSX_PAT,
  DOCKER_USERNAME/PASSWORD) is wired.
- `release-orchestration.yml` is present with inputs `version`, `prerelease`,
  `skip_crates`, `skip_extension`, `skip_docker`. If a single channel fails
  mid-cut, re-dispatch with that channel's `skip_*` set rather than re-tagging.

Tag, publish, and GitHub release creation still require explicit approval per
[Branch Protection Expectations](#branch-protection-expectations).

## Docs That Must Stay Aligned

- `docs/swarm/operating-model.md`
- `docs/swarm/review-rules.md`
- `docs/swarm/sync-protocol.md`
- `docs/project/status/development-moved-to-perl-lsp-swarm.md`
- `docs/project/status/real_perl_editor_trust_v1.md`
- `docs/project/status/provider_promotion_ledger.md`
- `.perl-lsp/goals/active.toml`

Do not duplicate generated status tables. Link to the status source instead.

## Branch Protection Expectations

This document does not configure branch protection.

Expected policy:

- `perl-lsp-swarm/main` remains the development integration branch.
- `perl-lsp/master` accepts curated syncs and release-lineage work only.
- Required checks must be green before merge unless a documented emergency
  exception exists.
- Branch deletion, force-push, history rewrite, tagging, package publish, and
  GitHub release creation still require explicit approval.

## Completion Criteria

A sync is complete only when:

- the target PR is merged
- the source/target invariant is checked
- required support/provider/status receipts are still valid
- any excluded work is listed
- release or publish claims are not made without explicit approval
