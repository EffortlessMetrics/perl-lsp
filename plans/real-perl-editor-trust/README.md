# Real Perl Editor Trust Plans

This directory holds implementation plans for the Real Perl Editor Trust lane.
Plans translate proposals, specs, ADRs, and current status receipts into a
reviewable PR sequence.

| Layer | Owns | Must not do |
|---|---|---|
| Plan | PR order, work-item decomposition, proof commands, rollback notes, issue/PR handoff state | Product claims, durable architecture decisions, generated metric content |

## Lane Scope

Real Perl Editor Trust covers three rails:

- parser compatibility from generated parser status and raw bucket receipts
- provider trust through confidence, freshness, fallback, and blocker receipts
- control-plane execution through plans and active goals that future agents can
  follow without chat history

Plans should link to generated status and receipt files, then state the next
reviewable unit of work. They should not replace generated status docs or
rewrite subsystem dashboards.

## Work Item Shape

Each work item should include:

```md
## Work item: id

Status:
Linked proposal:
Linked spec:
Linked ADR:
Blocks:
Blocked by:

Goal
Production delta
Non-goals
Acceptance
Proof commands
Rollback
```

## Current Status Sources

- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
- [parser status](../../docs/project/status/parser.md)
- [parser failure worklist](../../docs/project/status/parser_accuracy_failure_worklist.md)
- [provider cutover](../../docs/project/status/provider_cutover.md)
- [semantic scorecard](../../docs/project/status/semantic_scorecard.md)
- [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md)
- [semantic capability dashboard](../../docs/project/status/semantic_capability_dashboard.md)
- [UX capability dashboard](../../docs/project/status/ux_capability_dashboard.md)
