# Active Goals

Active goals are machine-readable current-state manifests for `perl-lsp-swarm`
lanes. They let agents identify the current objective, repo roles, WIP caps,
active work item, proof commands, and status pointers without scraping chat or
hand-maintained narrative.

| Layer | Owns | Must not do |
|---|---|---|
| Active goal | Machine-readable current work, status pointers, active work-item IDs, proof command list | Prose-only strategy, generated status content, durable design rationale |

## Manifest Contract

The active manifest should live at `.perl-lsp/goals/active.toml`. Future archived
manifests should move under `.perl-lsp/goals/archive/`.

An active manifest should include:

- stable lane ID and title
- active/inactive status
- objective and end state
- current repo and release-lineage repo
- trust/substrate/reliability lane caps and ownership
- current work items
- links to the relevant proposal, specs, plan, status docs, and operating model
- RTK-prefixed proof commands that define the current checkable boundary

Run the manifest validator after changing the active goal:

```bash
cargo xtask check-active-goal-manifest
```

## Status Pointers

Goal manifests point at status docs; they do not copy generated state. Preferred
current-state pointers for Real Perl Editor Trust are:

- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
- [parser status](../../docs/project/status/parser.md)
- [provider cutover](../../docs/project/status/provider_cutover.md)
- [semantic scorecard](../../docs/project/status/semantic_scorecard.md)
- [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md)
- [UX capability dashboard](../../docs/project/status/ux_capability_dashboard.md)

## Minimal Shape

```toml
id = "plsp-swarm-real-perl-editor-trust"
title = "perl-lsp-swarm execution lane"
status = "active"
owner = "codex-swarm"
created = "YYYY-MM-DD"

proposal = "docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md"
plan = "plans/real-perl-editor-trust/implementation-plan.md"
status_pointer = "docs/project/status/real_perl_editor_trust_v1.md"
operating_model = "docs/swarm/operating-model.md"

objective = """
State the active lane objective.
"""

end_state = [
  "State a checkable lane outcome.",
]

[current]
lane = "real_perl_editor_trust_v1"
repo = "perl-lsp-swarm"
release_lineage_repo = "perl-lsp"
status = "swarm_execution_cutover"

[limits]
trust_prs = 2
substrate_prs = 2
reliability_prs = 4

[[lanes]]
id = "trust"
pr_cap = 2
owns = ["provider_promotion_ledger"]
rule = "No broadening; name promotion, fallback, blocker, and receipt boundaries."

[trust.next]
items = ["real_perl_editor_trust_smoke_receipt"]

[[work_item]]
id = "work-item-id"
status = "active"
lane = "trust"
claim_boundary = "State the no-broadening boundary."
files = ["docs/project/status/parser_accuracy_next.md"]
commands = [
  "rtk cargo xtask update-status --only parser --check",
]
```
