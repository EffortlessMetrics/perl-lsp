# ADR/Spec Findings — work-e5cf7a7c

## What This ADR Decides

The ADR decides to split the diagnostic stability fix into two independent PRs:
1. **PR 1**: Generation-aware guard on `publish_parse_errors_fast` — ships immediately
2. **PR 2**: Incremental AST reuse — deferred pending API design work in `perl-incremental-parsing`

## Key Decision

All three prior agents (verification_agent, plan_review_agent, maintainer_vision_agent) converged on the same conclusion: Phase 2 (generation guard) is ready to implement now with low risk; Phase 1 (incremental AST) is blocked on an API gap in `IncrementalDocument::apply_edits()` which silently discards parse errors on success.

The ADR formalizes this split, allowing PR 1 to ship without waiting for the more complex incremental AST design work.

## Alternatives Considered

- **Single combined PR**: Rejected — Phase 1 blocked on separate API design
- **Only generation guard, drop incremental AST**: Rejected — performance problem is real and should be fixed
- **Increase debounce**: Rejected — hides symptoms, doesn't fix root cause

## Consequences

- Diagnostic flicker from generation races eliminated immediately (PR 1)
- Incremental AST path deferred but not abandoned — tracked as separate design issue
- Two independent PRs = two review cycles, but each is smaller and safer

## Acceptance Criteria

Per `specs.md`:
1. Stale parse errors are not published when generation advances during computation
2. Guard mirrors existing `publish_diagnostics` pattern (diagnostics.rs:495–506)
3. No regression in non-stale case
4. Test covers stale-generation skip path
