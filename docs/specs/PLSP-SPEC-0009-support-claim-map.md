# PLSP-SPEC-0009: Support claim map

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Implemented by:
- [support tiers](../project/status/SUPPORT_TIERS.md)
- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Real Perl Editor Trust routing dashboard](../project/status/real_perl_editor_trust_v1.md)
Status impact: support tiers, provider confidence matrix, provider cutover,
semantic scorecard, semantic shadow compare, Real Perl Editor Trust dashboard

## Current implementation status

This spec is implemented as a support-claim control-plane rule. Current claim
rows, proof commands, known limitations, and next promotion proof live in
[support tiers](../project/status/SUPPORT_TIERS.md). Provider-specific evidence
lives in the provider matrix, cutover, scorecard, and shadow-compare status
surfaces.

Current next work is not stored here; see the routing dashboard and
implementation plan. Generated parser status and receipt counts must stay in
their status surfaces rather than being copied into this spec.

## Contract

Every user-facing support claim must map to:

- a valid support tier
- the user-visible surface or provider it describes
- proof commands or status checks
- status documents that own the current evidence
- known limitations
- next promotion proof
- fallback, blocker, or no-edit behavior where the claim requires it

The support claim map is the public claim boundary for `perl-lsp`. It defines
what the project may say about a surface today. It does not create facts,
promote providers, or replace provider-specific receipts.

## Tier Vocabulary

Support rows must use the tier vocabulary defined by
[support tiers](../project/status/SUPPORT_TIERS.md):

| Tier | Required claim shape |
|---|---|
| `measured-bounded` | The surface is measured by receipts or baselines, and the claim names its measurement boundary. |
| `partial-live-with-fallback` | A scoped live path exists, fallback remains available, and limitations are explicit. |
| `shadowed` | Evidence is collected or compared, but live user-facing behavior has not been promoted. |
| `deferred` | The surface is intentionally not claimed live until named proof exists. |

New tier values require a spec or support-policy update before they appear in
the support map.

## Required Claim Fields

Each support row should preserve enough information for a user, reviewer, or
agent to decide what is claimed without reading chat context:

- surface or command name
- tier
- claim summary
- proof commands or status checks
- status docs that own current evidence
- known limitations
- next promotion proof or next blocker proof
- fallback, blocker, rollback, or no-edit state when relevant

Proof commands must be written as commands, not prose-only claims. Links to
status docs must be relative repository links.

## Claim Language Rules

Support rows and public docs must not claim:

- full CPAN support
- complete static analysis
- safe refactor everywhere
- generated symbols supported without scoped label, virtual-entry, or
  source-anchor wording
- compiler-backed semantic tokens broadly live when only scoped traces are
  proven
- destructive edit support without blocker, rollback, and no-edit behavior

Partial-live rows must name the fallback path. Edit-producing rows must name
blocker, rollback, and no-edit behavior. Generated or dynamic rows must say
whether the result is labeled, virtual, fallback, blocked, or deferred.

Support rows cannot cite stale receipts as promotion proof. Stale receipts may
explain historical context, but promotion requires current proof from the
owning status surface or PR validation.

## Valid PR Shapes

Valid support-claim PRs include:

- adding a support row for a newly documented surface with proof commands and
  known limitations
- updating a row after a provider cutover PR lands with current receipt proof
- narrowing a claim after a blocker, stale-fact issue, or false-allow receipt
- replacing vague public wording with tiered support language
- adding a validator check that enforces required fields, tier values, links,
  backticked commands, or forbidden claim phrases
- documenting that a surface remains shadowed or deferred after new proof

Support-claim PRs must state whether they are docs-only, validator-only, or tied
to a separately proven provider behavior change.

## Invalid PR Shapes

Invalid support-claim PRs include:

- promoting a support tier without current proof commands
- broadening public docs from receipt-only proof
- saying a generated/no-source surface is supported without label or source
  anchor boundaries
- saying rename or safe-delete is safe broadly when only narrow pilots are
  proven
- removing fallback, blocker, rollback, no-edit, or known-limitation language
  from partial-live claims
- citing generated parser bucket movement without a fresh corpus receipt
- copying dashboard rows, generated status counts, or one-off PR queue state
  into this spec
- changing provider behavior and support tiers in the same PR without a named
  live-cutover proof plan

## Acceptance

A support-claim change satisfies this spec when:

- every changed user-facing claim has a valid tier
- partial-live claims name fallback behavior
- edit-producing claims name blocker, rollback, or no-edit behavior
- generated and dynamic claims are labeled, blocked, fallback, shadowed, or
  deferred rather than presented as exact static facts
- proof commands are present and current
- status document links resolve
- known limitations are preserved or made more precise
- next promotion proof is explicit when a claim remains bounded
- no forbidden broad-support language is introduced

## Proof Commands

Support-claim PRs must run the support validator:

```bash
cargo xtask check-support-claims
git diff --check
```

Provider-adjacent claim changes must also run:

```bash
cargo xtask check-provider-confidence-matrix
```

Semantic or parser claim changes may require the owning status checks:

```bash
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
cargo xtask update-status --only parser --check
```

The PR body must list the focused proof actually run. Broad release or CI gates
may be required by the orchestrator, but this spec defines the minimum claim-map
proof.

## Non-goals

- no provider behavior changes
- no support-tier promotion from this spec alone
- no replacement for provider confidence receipts in `PLSP-SPEC-0002`
- no replacement for real-workspace baselines in `PLSP-SPEC-0003`
- no replacement for corpus freshness rules in `PLSP-SPEC-0004`
- no generated parser status counts in durable specs
- no current open PR queue, branch names, or temporary CI failure details

## Claim Boundaries

The support map may claim only the behavior supported by current proof. If a
surface is measured but not live, the claim remains `measured-bounded` or
`shadowed`. If a surface is scoped live with fallback, the claim remains
`partial-live-with-fallback` and must name what still falls back or blocks.

Docs and README changes must use the same support vocabulary as
[support tiers](../project/status/SUPPORT_TIERS.md). They may summarize user
value, but they must not convert scoped proof into broad static-analysis,
generated-symbol, semantic-token, rename, or safe-delete claims.

Status docs own current evidence. Specs own invariant behavior. Implementation
plans and routing dashboards own PR order and next proof.
