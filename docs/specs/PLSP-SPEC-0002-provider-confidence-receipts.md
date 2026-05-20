# PLSP-SPEC-0002: Provider confidence receipts

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Implemented by:
- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [support tiers](../project/status/SUPPORT_TIERS.md)
- [completed active goal manifest](../../.perl-lsp/goals/active.toml)
Status impact: provider cutover, semantic scorecard, semantic shadow compare,
UX capability dashboard

## Current implementation status

This spec is implemented as a control-plane rule. Current evidence lives in:

- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [support tiers](../project/status/SUPPORT_TIERS.md)
- [Real Perl Editor Trust routing dashboard](../project/status/real_perl_editor_trust_v1.md)
- [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)

Current next work is not stored here; see the routing dashboard and provider
status surfaces.

## Contract

Compiler-backed provider behavior must be confidence-aware before live cutover.

Before a provider uses compiler-backed or semantic facts as live user-visible
behavior, it needs receipts that explain why the provider acted, fell back,
blocked, or refused. The receipt must record:

- fact source
- provenance
- confidence
- freshness
- fallback state
- blocker reason when blocked
- user-facing blocker UX for edits or destructive actions
- live behavior comparison before cutover

Provider receipt PRs may add proof, traces, labels, blockers, or shadow
comparisons. They must not broaden live provider or refactor behavior unless a
separate cutover PR satisfies this spec and states rollback behavior.

## Provider Surfaces

This spec applies to:

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

Provider-specific status lives in [provider cutover](../project/status/provider_cutover.md),
[UX capability dashboard](../project/status/ux_capability_dashboard.md),
[semantic scorecard](../project/status/semantic_scorecard.md), and
[semantic shadow compare](../project/status/semantic_shadow_compare.md).
Specs link to those sources instead of copying their generated or
human-maintained tables.

## Confidence States

Provider receipts should make these states explicit:

| State | Provider behavior before broad live cutover |
|---|---|
| High confidence, fresh | May drive a scoped live slice only when fallback and rollback are proven |
| Medium confidence, fresh | May label, rank, or shadow; must not authorize unsafe edits by itself |
| Low confidence, fresh | May inform fallback ranking; must not authorize unsafe edits |
| Stale fact | Must block edits/destructive actions or fall back to legacy behavior |
| Dynamic boundary | Must block exact/static claims or label the boundary explicitly |
| Generated member | Must be labeled and must not imply an exact source-location promise unless separately proven |
| Missing proof | Must remain shadowed, fallback-only, or unavailable |

## Acceptance Examples

Valid receipt PRs:

- stale rename facts block edits and record a user-facing blocker reason
- stale safe-delete facts block deletion and compare against the live no-edit
  baseline
- low-confidence ambiguity records fallback or blocker state instead of
  authorizing an unsafe edit
- dynamic boundaries are labeled or blocked rather than promoted as exact
  static facts
- generated members are labeled and stay out of exact source-location claims
  until proven
- runtime receipts compare the live provider result with the compiler-backed
  plan without changing live behavior

Invalid PR shapes:

- live rename cutover without stale/low-confidence/dynamic blocker receipts
- safe-delete behavior that deletes from generated or stale facts
- goto or references claiming exact source locations for dynamic boundaries
- completion fallback that broadens to all workspace symbols without noise or
  ranking proof
- diagnostics that suppress true unknowns using low-confidence or stale facts
- provider receipt PRs that also change unrelated parser runtime behavior

## Cutover Requirements

A live provider cutover must satisfy all of these before broadening behavior:

1. Provider-specific shadow receipts exist.
2. Receipts include source, provenance, confidence, freshness, and fallback
   state.
3. Stale, low-confidence, generated, and dynamic-boundary cases are tested.
4. Edit-producing providers include blocker reasons and user-facing blocker UX.
5. Runtime receipts compare live behavior with compiler-backed candidate
   behavior.
6. Real-workspace quality is proven when the provider can be noisy, destructive,
   or user-visible at project scale.
7. Rollback or fallback behavior is documented in the PR body.
8. Status docs are updated by the appropriate generator or human-owned status
   surface, not by copying generated data into specs.

## Acceptance

A provider confidence PR satisfies this spec when:

- the PR names the provider surface and current status source
- the PR states whether behavior is shadow, partial live, fallback, blocked, or
  cutover
- receipts expose source, provenance, confidence, freshness, fallback, and
  blocker state where relevant
- stale facts do not authorize edits or destructive actions
- low-confidence facts do not authorize unsafe edits
- generated and dynamic-boundary facts are labeled or blocked
- live behavior comparison exists before cutover
- the PR body states claim boundaries and rollback/fallback behavior

## Proof Commands

Focused refactor blocker proof:

```bash
cargo test -p perl-lsp-rs-core --lib rename_shadow safe_delete_shadow -- --nocapture
cargo test -p perl-lsp-rs --lib refactor_runtime_blocker -- --nocapture
```

Semantic status proof:

```bash
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Provider-specific PRs may add narrower test commands when the provider surface
has a more precise receipt test.

## Non-goals

- no broad live provider cutover from this spec alone
- no parser bucket or corpus freshness rules
- no real-workspace baseline contract; that belongs in `PLSP-SPEC-0003`
- no support-tier claim map; that belongs in the status/support lane
- no dynamic Perl inference beyond documented boundaries
- no unsafe rename or safe-delete edits from stale, low-confidence, generated,
  or dynamic facts

## Claim Boundaries

Receipt PRs may claim that a provider records confidence/freshness/fallback or
blocker evidence for a scoped surface. They may not claim broad live cutover
unless the cutover requirements are met.

Shadow PRs may claim comparison coverage. They may not claim the live provider
uses compiler-backed facts.

Partial-live PRs may claim the specific high-confidence family promoted by the
PR. They must keep fallback and blocker behavior available, and they must state
which generated, stale, low-confidence, and dynamic cases remain shadowed or
blocked.
