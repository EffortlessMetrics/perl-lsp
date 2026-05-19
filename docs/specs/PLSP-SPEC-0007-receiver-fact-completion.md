# PLSP-SPEC-0007: Receiver-fact completion

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Receiver facts implementation plan](../project/RECEIVER_FACTS_IMPLEMENTATION_PLAN.md)
Status impact: provider confidence matrix, provider cutover, semantic scorecard,
semantic shadow compare, UX capability dashboard, support tiers

## Current implementation status

This spec is accepted as the receiver-aware completion cutover contract. The
current implementation has landed a narrow source-backed receiver completion
pilot for the proved hash-slot class, plus fallback preservation for dynamic
hash keys. Current evidence lives in [receiver_facts.md](../project/status/receiver_facts.md),
[provider_confidence_matrix.md](../project/status/provider_confidence_matrix.md),
[provider_cutover.md](../project/status/provider_cutover.md), and
[SUPPORT_TIERS.md](../project/status/SUPPORT_TIERS.md).

Completion remains `partial-live-with-fallback`. Broader receiver forms,
generated/no-source members, dynamic keys, stale facts, low-confidence facts,
and unknown receivers remain fallback, blocked, or future receipt work until a
separate PR satisfies this contract.

## Contract

Receiver-aware completion may rank or add method candidates only when receiver
facts are fresh, source-backed, high-confidence, and fallback-preserving.

This spec defines when completion may consume receiver facts. It depends on
[PLSP-SPEC-0005](PLSP-SPEC-0005-receiver-expression-facts.md) for semantic
receiver extraction and [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
for provider confidence, fallback, and cutover requirements. It must not
duplicate those lower-level fact and receipt contracts.

## Receiver Forms

Completion may request receiver facts for these receiver shapes:

| Receiver form | Completion contract |
| --- | --- |
| `$self->method` | May rank package methods only when `$self` has a fresh source-backed object/package fact. |
| `$object->method` | May rank package methods only when the object variable resolves to a high-confidence package fact. |
| `Class->new` | May use static package receiver evidence for constructor and class-method completion. |
| `$hash{key}->method` | May use a static hash-slot receiver fact only when the key is literal/static and the slot fact is fresh. |
| `$hashref->{key}->method` | May use a static hashref-slot receiver fact only when the key is literal/static and the slot fact is fresh. |
| `$array[$i]` | Must not claim an exact package unless the indexed or element fact is source-backed and high-confidence. |
| unknown receiver | Must preserve legacy fallback and label low-confidence or unknown receiver evidence when surfaced. |
| dynamic key | Must not claim an exact package or suppress legacy fallback. |

## Required Fact Shape

Provider-visible receiver facts must expose, directly or through a structured
receipt, these fields before they influence completion ranking or candidate
inclusion:

```text
receiver_kind
inferred_package
shape_fact
confidence
evidence
freshness
dynamic_boundary
source_range
fallback_state
```

`inferred_package` may be absent. Absence means completion may use legacy
fallback according to the provider confidence rules; it must not invent an exact
receiver package.

`source_range` must identify the source-backed receiver expression when exact
receiver facts drive ranking or candidate inclusion. Generated, virtual, or
framework-derived evidence may be labeled, but it is not source-backed exact
receiver evidence unless a separate proof explicitly promotes that class.

## Cutover Ladder

Receiver-aware completion must move through this sequence:

1. Extract receiver facts in the semantic analyzer.
2. Add completion ranking receipts that show how receiver facts would affect
   candidates.
3. Preserve legacy fallback for unknown, dynamic, stale, and unsupported
   receivers.
4. Enable a narrow source-backed pilot only for fresh high-confidence receiver
   facts with proven fallback behavior.
5. Update support tiers only after proof commands and status docs justify the
   scoped claim.

Skipping a step is invalid. Receipt-only PRs must not broaden live completion
behavior.

## Valid PR Shapes

Valid PRs under this spec include:

- semantic analyzer PRs that add or refine receiver facts without changing
  completion behavior
- receipt PRs that compare receiver-fact ranking against existing completion
  output without changing live behavior
- fallback PRs that prove unknown or dynamic receivers keep legacy candidates
  and labels
- narrow cutover PRs that enable source-backed receiver completion for one
  proven receiver family
- status PRs that update support tiers only after proof already landed

Each PR must state which receiver form it affects and which generated, dynamic,
stale, low-confidence, and unknown cases remain fallback or blocked.

## Invalid PR Shapes

Invalid PRs include:

- method completion from stale facts
- dynamic hash keys treated as exact receiver evidence
- generated or no-source members treated as source-backed receivers
- unknown receivers suppressing legacy completion candidates
- all-workspace method fallback used as a substitute for receiver proof
- completion ranking changes without receipt coverage
- support-tier promotion from facts-only or docs-only work

## Acceptance

A receiver-aware completion PR satisfies this spec when:

- exact receiver completion is limited to fresh, source-backed,
  high-confidence receiver facts
- unknown and dynamic receivers preserve legacy fallback behavior
- dynamic boundaries are labeled or blocked instead of promoted to exact facts
- generated or no-source members are labeled and do not imply exact source
  backing
- receiver details or receipts expose receiver kind, confidence, evidence,
  freshness, and fallback state
- exact receiver candidates rank above fallback candidates
- fallback candidates, when present, remain bounded and labeled
- status docs name the claim boundary and next proof before any support-tier
  promotion

## Proof Commands

Facts-only receiver PRs should use:

```bash
./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked receiver_fact
./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked
git diff --check
```

Completion receipt or cutover PRs should add provider proof:

```bash
./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked completion
./scripts/cargo-safe check --all-targets -p perl-lsp-rs-core --profile agent --locked
cargo xtask semantic-shadow-compare --check
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
git diff --check
```

Docs-only PRs for this spec may use:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
git diff --check
```

## Non-goals

- No broad method-completion cutover.
- No all-workspace fallback expansion.
- No generated/no-source method-body location claim.
- No dynamic hash-key or dynamic method-name exactness.
- No parser rewrite or new AST shape requirement; that belongs outside this
  completion contract.
- No replacement of [PLSP-SPEC-0005](PLSP-SPEC-0005-receiver-expression-facts.md)
  fact extraction requirements.
- No support-tier promotion from this spec alone.

## Claim Boundaries

Facts-only PRs may claim receiver facts are available for a scoped expression
family. They may not claim user-visible completion behavior.

Receipt PRs may claim receiver-fact completion ranking is measured. They may not
claim live cutover.

Narrow cutover PRs may claim live receiver-aware completion only for the exact
receiver form, confidence level, source-backing class, and fallback behavior
proved by tests and status docs.

Support-tier updates must keep completion as `partial-live-with-fallback` unless
the support map, provider confidence matrix, and semantic shadow compare prove a
different public claim. Any generated, dynamic, stale, low-confidence, or
unknown receiver class not covered by proof must remain fallback, labeled, or
blocked.

## Current Evidence Owners

Current state and evidence live outside this spec:

- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Semantic capability dashboard](../project/status/semantic_capability_dashboard.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Receiver facts implementation plan](../project/RECEIVER_FACTS_IMPLEMENTATION_PLAN.md)

Do not copy dashboard rows, generated parser bucket counts, current PR order, or
one-off receipt filenames into this spec.
