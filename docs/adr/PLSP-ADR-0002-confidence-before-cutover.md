# PLSP-ADR-0002: Confidence before provider cutover

Status: accepted
Date: 2026-05-13
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](../specs/PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0003](../specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`

## Context

`perl-lsp` can increasingly derive editor answers from parser, semantic,
workspace, and compiler-backed facts. Those facts can improve completion,
hover, goto definition, references, diagnostics, rename, safe delete, document
symbols, workspace symbols, semantic tokens, and DAP module-path behavior.

The same facts can also be unsafe when their confidence or freshness boundary is
unclear. A stale compiler fact can authorize an edit to the wrong code. A
low-confidence inferred fact can make a navigation answer look exact. A dynamic
Perl boundary or generated member can look like a static source location when
it is only a labeled possibility.

The repo already tracks provider cutover as a staged process in
[provider cutover](../project/status/provider_cutover.md) and related semantic
status surfaces. This ADR makes that operating rule durable: facts do not
become live authority merely because they exist.

## Decision

Provider cutover requires confidence and freshness receipts before
compiler-backed or semantic facts can authorize user-visible edits, deletion,
or definitive navigation claims.

Before a provider broadens live compiler-backed behavior, committed receipts
must show:

- fact source
- provenance
- confidence
- freshness
- fallback state
- blocker reason when blocked
- user-facing blocker UX for edit-producing actions
- live behavior comparison before cutover
- rollback or fallback behavior for the cutover slice

Rename and safe delete must block on stale, low-confidence, generated, or
dynamic-boundary facts unless a narrower spec explicitly proves the behavior is
safe. Goto, references, hover, completion, diagnostics, symbols, semantic
tokens, and DAP module paths must label, rank, fall back, or remain shadowed
when proof is incomplete.

## Applicability

This ADR applies to provider behavior that consumes compiler-backed, semantic,
workspace, parser-derived, or Perl-oracle facts for:

- completion
- goto definition
- hover
- references
- workspace symbols
- document symbols
- semantic tokens
- rename
- safe delete
- diagnostics
- DAP module paths and Perl subprocess seams

It does not require every receipt PR to cut behavior over. Receipt PRs may stay
shadow-only or fallback-only while they build proof.

## Cutover Rules

1. Fact availability is not provider cutover.
2. Shadow comparison comes before broad live behavior.
3. Fresh high-confidence facts may drive a scoped live slice only when fallback
   and rollback are proven.
4. Medium-confidence facts may be labeled, ranked, or shadowed; they must not
   authorize unsafe edits by themselves.
5. Low-confidence facts may inform fallback ranking; they must not authorize
   unsafe edits.
6. Stale facts must block edit/destructive actions or fall back to legacy
   behavior.
7. Dynamic-boundary facts must block exact/static claims or be labeled
   explicitly.
8. Generated members must be labeled and must not imply exact source-location
   promises unless separately proven.
9. Real-workspace quality receipts are required before broadening providers
   that are noisy, destructive, or project-scale user-facing.
10. PR bodies must state the live behavior state, fallback behavior, claim
    boundary, and rollback path.

## Consequences

Positive consequences:

- Refactor operations fail closed instead of authorizing edits from stale or
  ambiguous facts.
- Navigation and hover can explain when an answer is exact, ranked, fallback,
  generated, dynamic, or unavailable.
- Completion and diagnostics can improve incrementally without silently
  promoting unsupported dynamic inference.
- Real-workspace proof becomes a cutover gate for project-scale provider
  quality.
- Users see fewer mysterious wrong answers because provider receipts explain
  why the LSP acted, fell back, blocked, or refused.

Tradeoffs:

- Live provider behavior can trail available fact-layer capability.
- Some features will remain shadowed or fallback-only until receipt coverage is
  strong enough.
- Edit-producing providers need extra blocker UX proof before cutover.
- Status and PR bodies must be precise about what is live, shadowed, blocked,
  or fallback-only.

## Alternatives Considered

### Cut over whenever compiler facts exist

Rejected. Fact existence does not prove freshness, precision, or user-safe
behavior. This would make the editor more powerful before it is explainable.

### Use confidence labels only in diagnostics

Rejected. Confidence and freshness affect all editor-provider surfaces, not
only diagnostics. Rename, safe delete, navigation, completion, hover, symbols,
semantic tokens, and DAP module behavior all need boundaries.

### Keep confidence rules in PR bodies only

Rejected. PR bodies are necessary evidence, but they are not durable enough for
future provider work. The repo needs a stable decision that future specs,
plans, and status surfaces can reference.

## Follow-up Obligations

- Keep `PLSP-SPEC-0002` linked from provider confidence and cutover PRs.
- Link real-workspace provider quality proof to `PLSP-SPEC-0003` before broad
  live cutover of noisy or project-scale providers.
- Update provider status surfaces when a provider moves from shadowed to
  partial live or live.
- Keep runtime receipt tests explicit that a receipt PR did or did not broaden
  live behavior.
- Record blocker UX for edit-producing providers before user-visible refactor
  cutover.

## Status Links

- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Compiler facts](../project/status/compiler_facts.md)

## Why ADR-worthy

This is a durable safety decision for editor trust. It defines when facts may
authorize live provider behavior and when they must remain shadowed, labeled,
fallback-only, or blocked.
