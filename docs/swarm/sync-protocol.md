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

### perl-lsp to Swarm

Use only for emergency release fixes or old-repo-only markers.

Required before merging old `perl-lsp` work:

- verify the same content exists in swarm, or merge a swarm mirror PR first
- keep the old-repo PR scoped to release-lineage work
- run the narrowest old-repo validation for the changed files
- run the corresponding swarm validation after the mirror

Source-to-swarm sync PRs should use merge commits when commit ancestry matters.
Do not squash source-sync PRs that are meant to prove `source/master` ancestry.

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
