# PLSP-ADR-0001: Generated status is the control plane

Status: accepted
Date: 2026-05-13
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0001](../specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
- [PLSP-SPEC-0004](../specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`

## Context

`perl-lsp` now has generated parser status, parser accuracy status, semantic
scorecards, provider cutover status, UX dashboards, and receipt-backed proof
commands. Those artifacts are not only reports for humans after work is done.
They also decide what work is valid next.

The parser lane is the clearest example:

- [parser accuracy next](../project/status/parser_accuracy_next.md) records
  whether measurement failures or gaps exist.
- When measurement is clear, it points agents to
  [parser raw failure buckets](../project/status/parser.md#raw-failure-buckets)
  only when current generated status lists nonzero bucket evidence.
- [parser status](../project/status/parser.md) records raw bucket rows,
  corpus receipt provenance, and freshness boundaries.

Without an explicit decision, agents can treat generated status as passive
background documentation and rely on chat history, stale plans, or remembered
bucket names. That makes the lane fragile and increases overclaim risk.

## Decision

Generated status files are control-plane artifacts for parser and editor-trust
work. They route valid next work, define current receipt boundaries, and anchor
claims that appear in plans, PR bodies, and support/status docs.

For the Real Perl Editor Trust lane:

- `parser_accuracy_next.md` decides whether parser work is measurement work or
  capability work.
- `parser.md` owns parser compatibility baselines, raw failure buckets, and
  corpus receipt freshness.
- Provider and semantic status files own current provider proof state,
  confidence/freshness boundaries, and cutover limits.
- Specs describe how to interpret status outputs.
- Xtask/status generators own generated content.
- Plans sequence PRs from the generated status pointers.

Agents must read generated status before choosing parser/provider work and must
not hand-edit generated sections.

## Consequences

Positive consequences:

- Agents can continue the lane from repository artifacts instead of chat
  transcripts.
- Empty measurement queues can route directly to capability buckets when
  generated status lists nonzero bucket evidence, without a human remembering
  the next bucket.
- Stale corpus data can still drive fixture discovery without becoming a
  current bucket-count claim.
- Provider cutover work can stay gated on status-backed confidence and
  freshness receipts.
- Support claims can link to proof status instead of duplicating tables.

Tradeoffs:

- Generated status must remain readable enough for agents, not just for release
  reviewers.
- Status generator changes become control-plane changes and need focused
  review.
- Plans and PR bodies must state which generated status pointer they followed.
- If generated status is stale or unavailable, work must explicitly defer the
  affected claim instead of filling the gap with prose.

## Operating Rules

Agents and maintainers must follow these rules:

1. Read the relevant generated status file before choosing parser or provider
   work.
2. Treat generated status pointers as the current route unless the
   implementation plan explicitly parks them with a reason. Do not start
   raw-bucket work when generated parser status lists `none`.
3. Do not hand-edit generated sections.
4. Do not copy generated metric tables into specs, ADRs, or plans.
5. Link to generated status from specs and PR bodies.
6. State receipt freshness and claim boundaries when a PR follows a generated
   parser bucket or provider proof gap.
7. Regenerate status only through the owning xtask/status commands.

## Alternatives Considered

### Keep generated status as passive reporting

Rejected. Passive reporting leaves next-work selection in operator memory and
chat context. That is exactly the failure mode this lane is removing.

### Move current state into hand-maintained plans

Rejected. Plans are useful for PR sequencing, rollback, and proof commands, but
they drift when they duplicate generated status. Plans should link to status
and record why a pointer is parked, not become another metric source.

### Put all routing logic in issues

Rejected. Issues are useful review and assignment surfaces, but the repo must
remain executable by agents without scraping issue comments. Issues should
reference generated status and specs, not replace them.

## Follow-up Obligations

- Keep `PLSP-SPEC-0001` and `PLSP-SPEC-0004` linked from parser bucket and
  corpus freshness work.
- Add the Real Perl Editor Trust implementation plan so PR sequencing follows
  this ADR without duplicating generated status.
- Add the active goal manifest so agents have a machine-readable entry point.
- When a status generator changes a work-routing rule, review it as a
  control-plane change.

## Status Links

- [Parser accuracy next](../project/status/parser_accuracy_next.md)
- [Parser status](../project/status/parser.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)

## Why ADR-worthy

This is a durable operating-model decision. It changes how maintainers and
agents choose valid work, state proof boundaries, and avoid stale claims across
parser compatibility and editor-provider trust lanes.
