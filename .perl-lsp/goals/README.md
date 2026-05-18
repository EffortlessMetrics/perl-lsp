# Active Goals

Active goals are machine-readable current-state manifests for `perl-lsp` lanes.
They let agents identify the current objective, active work item, proof commands,
and status pointers without scraping chat or hand-maintained narrative.

See [SPEC_SYSTEM.md](../../docs/reference/SPEC_SYSTEM.md) for the full
source-of-truth stack and agent workflow.

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
- current work items
- links to the relevant proposal, spec, ADR, plan, and status docs
- proof commands that define the current checkable boundary

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
id = "plsp-real-perl-editor-trust"
title = "Real Perl editor trust"
status = "active"
owner = "codex-swarm"
created = "YYYY-MM-DD"

objective = """
State the active lane objective.
"""

end_state = [
  "State a checkable lane outcome.",
]

[[work_item]]
id = "work-item-id"
status = "active"
spec = "docs/specs/PLSP-SPEC-####-short-name.md"
plan = "plans/real-perl-editor-trust/implementation-plan.md"
current_pointer = "docs/project/status/parser_accuracy_next.md"
current_status = "docs/project/status/parser.md#raw-failure-buckets"
commands = [
  "cargo xtask update-status --only parser --check",
]
```
