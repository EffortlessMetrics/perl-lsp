# PLSP-SPEC-0019: Semantic token class promotion contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Policy: [semantic-token-classes.toml](../../policy/semantic-token-classes.toml)
Status impact: semantic tokens, provider promotion ledger, provider
confidence matrix, support tiers, semantic shadow compare

## Current Implementation Status

Semantic tokens are `partial-live-with-fallback`. The parser/HIR provider
remains the live token source. Compiler-backed token facts may participate only
as reviewed, source-backed, output-neutral trace slices for specific classes.

The current reviewed trace classes are:

```text
subroutine_declaration
method_declaration
package_declaration
phase_block_declaration
field_declaration
method_call
self_method_call
lexical_variable_declaration
lexical_variable_use
```

Those classes prove identity alignment with existing live tokens. They do not
emit new semantic tokens, broaden token classifications, or authorize generated,
dynamic, stale, low-confidence, fallback, or unmatched compiler-token facts.

## Contract

Compiler-backed semantic-token classes are deny-by-default.

A compiler-token class can be promoted only when all of these are true:

1. the compiler span is source-backed
2. the span matches exactly one existing live parser/HIR token
3. live output does not gain unscoped tokens
4. `didChange` freshness is proven for the request shape
5. generated/no-source candidates are blocked
6. stale, dynamic, low-confidence, fallback, and unmatched candidates are
   blocked or shadowed
7. support review says the proof does not imply broad compiler-token promotion

The policy registry in
[semantic-token-classes.toml](../../policy/semantic-token-classes.toml)
records the reviewed classes. Unlisted classes are not promoted.

## Class States

### `partial_live_trace`

The class may appear in provider decision traces when its source-backed compiler
span exactly matches one existing live parser/HIR token. It must not add token
output.

### `shadow_proof`

The class may appear in receipts or shadow comparison output. It must not affect
live provider output.

### `blocked`

The class is known to be unsafe or unsupported for live output. It may appear
only as a blocker, fallback, or explanation.

### `deferred`

The class has no reviewed promotion rule yet. It remains fallback-only until a
new policy row and receipt promote one class.

## Required Class Fields

Each promoted or traced class must define:

- class name
- current state
- live token kind it must match
- whether it emits new output
- exact live-token match requirement
- edit-freshness requirement
- blocker list
- receipt sources
- claim boundary

The registry is checked by:

```bash
cargo xtask check-semantic-token-classes
```

PRs that touch semantic-token class behavior must keep the registry and status
claims in agreement.

## Provider Rules

Semantic-token providers must preserve these rules:

- existing parser/HIR token output remains the fallback and live baseline
- source-backed compiler-token trace slices may explain why a class is trusted
- trace slices must be output-neutral unless a later spec and support review
  explicitly promote emitted output
- one class receipt promotes only that class
- broader compiler-token categories remain blocked, fallback-only, or shadowed
- generated/no-source, stale, dynamic, low-confidence, fallback, and unmatched
  candidates cannot become token identities
- provider decision receipts must expose the fallback or blocker state when a
  compiler-token fact is not used

## Valid PR Shapes

Valid PRs under this spec include:

- adding a new class row to `semantic-token-classes.toml`
- adding source-backed span-match proof for exactly one token class
- adding `didChange` freshness proof for exactly one token class
- adding blockers for generated/no-source, stale, dynamic, low-confidence,
  fallback, or unmatched compiler-token candidates
- adding a validator for semantic-token class policy
- support-review PRs that keep broad compiler-token promotion deferred

Every class-promotion PR must name the class, live token kind, promotion rule,
fallback rule, blocker rule, and receipt.

## Invalid PR Shapes

Invalid PRs include:

- broad compiler-backed semantic-token cutover from a scoped trace receipt
- adding emitted token output without a class-specific support review
- promoting a class whose compiler span does not exactly match one live token
- treating generated/no-source, dynamic, stale, low-confidence, fallback, or
  unmatched compiler facts as token identities
- using token proof to authorize rename, safe delete, diagnostics, or broader
  provider behavior
- support-tier promotion without policy, receipts, and status review

## Acceptance

A semantic-token PR satisfies this spec when:

- every touched compiler-token class has a policy row or remains explicitly
  unlisted and blocked
- promoted/traced classes are source-backed and fresh
- the compiler span matches exactly one existing live token
- live output remains unchanged unless a later policy row explicitly permits
  output expansion
- generated/no-source, stale, dynamic, low-confidence, fallback, and unmatched
  candidates remain blocked or shadowed
- provider decision receipts and support-tier wording stay bounded by the
  reviewed class

## Proof Commands

Docs-only changes to this spec or policy may use:

```bash
cargo xtask check-semantic-token-classes
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask check-provider-promotion-ledger
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Class-behavior PRs must add or update focused semantic-token runtime and shadow
receipts for the touched class.

## Validator Contract

The validator checks:

- every live compiler-token class has a policy row
- every policy row has receipt sources
- every `partial_live_trace` class requires exact live-token match
- every current row is output-neutral unless explicitly reviewed otherwise
- blocker names are drawn from the provider promotion ledger blocker registry,
  plus the semantic-token-local `unmatched_span` blocker

## Non-goals

- No provider behavior change from this spec alone.
- No broad compiler-backed semantic-token promotion.
- No generated/no-source token promotion.
- No dynamic token promotion.
- No emitted output expansion from trace-only class proof.
- No rename, safe-delete, diagnostic, workspace-symbol, or support-tier
  promotion from token proof alone.

## Claim Boundaries

This spec may claim that semantic-token class promotion is controlled by a
class registry and output-neutral proof boundary. It may not claim that broad
compiler-token output is live, or that a scoped class receipt proves any other
class.
