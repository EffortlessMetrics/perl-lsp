# Development Moved to perl-lsp-swarm

Active development has moved to
[`EffortlessMetrics/perl-lsp-swarm`](https://github.com/EffortlessMetrics/perl-lsp-swarm).

`perl-lsp` remains the release, history, and canonical package-lineage repo until
curated sync or release PRs promote swarm work back there.

New feature work should target `perl-lsp-swarm`.

## Repository Roles

| Repo | Role |
|---|---|
| `perl-lsp-swarm` | Active development execution repo for agent lanes, proof receipts, spec hardening, promotion-ledger work, cleanup trains, and compiler substrate work |
| `perl-lsp` | Release lineage, historical upstream, emergency release fixes, and curated sync target |

## Sync Invariant

`perl-lsp` must not advance ahead of `perl-lsp-swarm`.

Routine work starts in `perl-lsp-swarm`. If an emergency release fix must land in
`perl-lsp`, mirror or sync that change through `perl-lsp-swarm` before treating
the old repo as current.

## Current Boundary

This marker is documentation only. It does not change provider behavior, support
tiers, CI workflows, branch protection, package publication, or release
automation.
