# Goal Portfolio

The portfolio is machine-readable repository state for `perl-lsp-swarm`. It
records enabled programs, lane ownership, WIP caps, dependencies, and proof
surfaces without appointing one repository-global objective for every worker.
An orchestrator claims an issue for one session and worktree; that claim is not
stored as a mutable default in this file.

| Layer | Owns | Must not do |
|---|---|---|
| Portfolio | Enabled programs, lane caps, selection inputs, and authority boundaries | One session's current issue, branch lease, or worker state |
| Program manifest | Program objective, lanes, work items, proof commands, and status pointers | Repository-global priority across sibling programs |

## Manifest Contract

The portfolio lives at `.perl-lsp/goals/active.toml` as the in-place schema 3
compatibility migration. There is no second portfolio file; future archived
manifests should move under `.perl-lsp/goals/archive/`.

An active manifest should include:

- schema and `mode = "portfolio"`
- authority and selection policy
- one or more enabled/disabled program manifest entries
- an explicit `kind` for every program: `lane_routing` or `milestone_ledger`

All entries must have valid identity, path, kind, and parseable manifest shape.
Only enabled entries contribute active lanes, work items, and milestone
validation totals.

Legacy `active_program`, `active_lane`, and `default_program` fields are accepted
only as temporary compatibility warnings. They are never selection authority.

Run the manifest validator after changing the portfolio:

```bash
cargo xtask check-active-goal-manifest
```

## Program and claim layers

Program manifests retain the durable objective, lane caps, work items, status
documents, and proof commands. A future work-order/claim command will compile a
single GitHub issue into a session-local lease. Until then, `goals next` remains
read-only and an explicit `--program` is required to inspect one program.

## Status Pointers

Goal manifests point at status docs; they do not copy generated state. Preferred
current-state pointers for Real Perl Editor Trust are:

- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
- [parser status](../../docs/project/status/parser.md)
- [provider cutover](../../docs/project/status/provider_cutover.md)
- [semantic scorecard](../../docs/project/status/semantic_scorecard.md)
- [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md)
- [UX capability dashboard](../../docs/project/status/ux_capability_dashboard.md)

## Minimal Portfolio Shape

```toml
schema = 3
mode = "portfolio"

[selection]
strategy = "eligible_portfolio"
require_explicit_claim = true
respect_lane_caps = true
respect_dependencies = true
respect_conflict_surfaces = true

[[program]]
id = "real_perl_editor_trust"
manifest = ".perl-lsp/goals/programs/real_perl_editor_trust.toml"
kind = "lane_routing"
enabled = true
```
