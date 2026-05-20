# PLSP-SPEC-0015: Real Perl Editor Trust v1 boundary

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0008](PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0010](PLSP-SPEC-0010-support-claim-map.md)
- [PLSP-SPEC-0012](PLSP-SPEC-0012-user-facing-trust-surfaces.md)
- [PLSP-SPEC-0014](PLSP-SPEC-0014-refactor-acceptance.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0018](PLSP-SPEC-0018-edit-authorization-contract.md)
- [PLSP-SPEC-0019](PLSP-SPEC-0019-semantic-token-class-promotion-contract.md)
- [PLSP-SPEC-0020](PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
- [PLSP-ADR-0003](../adr/PLSP-ADR-0003-preview-before-edit.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: support tiers, provider confidence matrix, provider promotion
ledger, provider cutover, real-workspace receipts, workspace trust report

## Current Implementation Status

This spec is accepted as the Real Perl Editor Trust v1 boundary contract. It
turns the current routing dashboard into a normative claim boundary: compiler
facts may help users only inside their proof boundary, and every provider
promotion must remain traceable to support tiers, provider confidence,
promotion-ledger rows, and receipts.

Current evidence and the active per-surface state live in:

- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider promotion ledger](../project/status/provider_promotion_ledger.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)

Do not copy current dashboard tables, parser counts, open PR state, or one-off
receipt values into this spec. Status docs own current evidence. This spec owns
the invariant that future work must preserve.

## Contract

Real Perl Editor Trust v1 is the editor trust boundary for `perl-lsp`. It is not
a broad compiler cutover, full static-analysis claim, release approval, or
Rust-native Perl runtime milestone.

The trusted editor loop is:

```text
completion suggests it
hover explains it
definition jumps to it
references finds uses
diagnostics explain why they fire or stay conservative
rename and safe-delete know whether edits are safe
symbols and tokens expose project shape without false exactness
explain-provider-decision exposes the receipt boundary
workspace trust report explains setup state without probing
```

Every trusted answer must expose or be backed by:

- fact source
- confidence
- freshness
- source-backed state or source range when available
- fallback state
- blocker reason when blocked
- dynamic-boundary state when relevant
- claim boundary

The project-wide invariant is:

```text
A fact can help the user only inside its proof boundary.
```

## Source, Generated, Dynamic, And Ambient Rules

Provider and compiler facts must preserve these boundaries:

| Fact state | Allowed v1 use | Forbidden v1 use |
| --- | --- | --- |
| Source-backed, fresh, high confidence | Scoped exact behavior when the provider-specific promotion row and receipts allow it | Broad support claims beyond the promoted fact class |
| Source-backed generated | Labeled virtual symbols, hover/completion explanation, blocker evidence, or scoped pilot behavior when proven | Exact generated method-body locations or rename/delete authorization without class-specific proof |
| Generated/no-source | Receipt-only, blocked, or explanation-only | Exact source-backed behavior, rename, safe-delete, or unlabeled workspace-symbol promotion |
| Dynamic boundary | Explanation, fallback, or edit blocker | Exact static claims or edit authorization |
| Ambient input | Report in setup/trust/determinism surfaces | Silent treatment as workspace source |
| Stale, low-confidence, or ambiguous | Fallback, preview, explanation, or blocker | Edit authorization or exact provider behavior |
| Unknown | Fallback-only or blocked | Promotion to exact behavior |

## Surface Boundary

Real Perl Editor Trust v1 may claim only these bounded surface classes:

| Surface | v1 boundary |
| --- | --- |
| Completion | Partial live with fallback for proven visible-symbol and narrow source-backed receiver slices; generated, dynamic, stale, unknown, and medium/low-confidence receiver classes remain fallback, shadowed, or blocked until their own receipts promote one class. |
| Hover | Partial live provenance-backed explanations for exact, imported, generated/framework, dynamic-boundary, module-resolution, and fallback paths where receipts exist. |
| Definition | Partial live exact/imported source-backed navigation; generated/no-source, stale, low-confidence, ambiguous, and dynamic cases keep fallback or blockers. |
| References | Partial live exact/imported/literal-require source-backed references where scoped receipts prove freshness and declaration behavior; generated, coderef, typeglob, stale, ambiguous, and dynamic cases remain bounded. |
| Diagnostics | Partial live diagnostics plus explanation payloads; explanation-only work must not change severity, suppression, resolver behavior, workspace scanning, or support tier. |
| Document symbols | Partial live source-backed parser-syntax symbols; generated/no-source and dynamic expansion remains gated. |
| Workspace symbols | Partial live source-backed ready-index symbols plus the generated-label pilot; generated symbols must be labeled and source-anchored, never fake generated method bodies. |
| Semantic tokens | Existing parser/HIR token output plus scoped source-backed compiler-token trace slices that emit no unscoped new output. |
| Rename | Same-file lexical rename and narrow package-local pilot only; broad package/compiler-backed rename remains gated by edit authorization proof. |
| Safe delete | Exact unreferenced source-backed subroutine pilot only, guarded by current-source references, workspace-index references, workspace identity, and rollback proof. |
| Provider explanations | Copyable provider-decision receipts and user messages explain act/fallback/block/defer decisions without creating stronger facts. |
| Workspace trust report | Advisory setup state from already-known configuration/client state only; no Perl probe, `perldoc` execution, DAP launch, debug-session inspection, or workspace scan. |

## Promotion Engine Requirements

Every main-lane provider PR must name:

```text
one fact class
one provider surface
one promotion rule
one fallback rule
one blocker rule
one receipt
```

Every promotion-ledger row must support one of:

```text
promote
fallback
block
defer
```

Receipts that do not lead to one of those decisions are side-lane evidence until
their promotion boundary is clear.

## Valid PR Shapes

Valid PRs under this boundary include:

- docs or policy PRs that make the v1 boundary more explicit
- receipt PRs that prove one fact class for one surface without broadening live
  behavior
- scoped cutover PRs that satisfy the promotion-ledger row for one fact class
- support-review PRs that preserve, narrow, or explicitly defer a support claim
- validator PRs that enforce schema, ledger, blocker, or claim-boundary rules
- explanation-surface PRs that render existing provider or diagnostic receipts
  without changing provider behavior

Every valid PR must state whether it changes documentation, policy, receipts,
runtime behavior, provider output, edit authorization, schemas, or validators.

## Invalid PR Shapes

Invalid PRs include:

- broad provider cutover without a promotion-ledger row and receipts
- broad rename or safe-delete promotion without edit authorization proof
- treating generated/no-source, dynamic, stale, low-confidence, ambiguous, or
  ambient facts as exact source-backed facts
- changing diagnostic suppression, severity, resolver behavior, workspace
  scanning, or support tiers from an explanation-only PR
- turning workspace trust report into probing, Perl execution, `perldoc`
  execution, DAP launch, debug-session inspection, or raw-path reporting
- claiming generated workspace symbols have exact generated method-body
  locations
- claiming compiler-backed semantic tokens broadly live from scoped trace proof
- mixing parser bucket movement, provider cutover, release approval, security
  reporting, and docs/policy contracts in one PR

## Acceptance

A Real Perl Editor Trust v1 PR satisfies this spec when:

- the changed surface remains inside the support tier and promotion-ledger
  boundary
- source-backed, generated, generated/no-source, dynamic, ambient, stale,
  low-confidence, ambiguous, and unknown facts keep their required behavior
- partial-live surfaces preserve fallback
- edit-producing providers preserve blocker/no-edit/rollback proof
- explanation surfaces do not create facts or broaden behavior
- status docs link to current evidence instead of copying generated counts into
  durable specs
- proof commands match the touched surface
- the PR body states the claim boundary and validation gaps

## Proof Commands

Docs-only changes to this spec must run:

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Provider-adjacent PRs must also run the focused provider, semantic, refactor,
token, symbol, diagnostic, trust-report, or workspace receipt tests for the
touched surface.

Parser-status or parser-accuracy claim changes must also run:

```bash
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
```

## Non-goals

- No provider behavior change from this spec alone.
- No broad completion, hover, definition, references, diagnostics, semantic
  tokens, workspace symbols, rename, safe-delete, DAP, or module-resolution
  cutover.
- No release, publish, or Rust-native Perl runtime approval.
- No replacement for provider confidence receipts, support tiers, edit safety,
  workspace trust report, or refactor acceptance specs.
- No generated parser status counts, active PR queue state, or one-off CI
  failures in durable spec text.

## Claim Boundaries

This spec may claim that the project has a v1 trust boundary and promotion
discipline. It may not claim that all provider surfaces are complete, broad
compiler-backed behavior is live, edits are safe broadly, or generated/dynamic
facts are exact.

Real Perl Editor Trust v1 is useful because it tells users what `perl-lsp`
knows, why it knows it, what it cannot prove, and what it refuses to change.
The same substrate may later support PIR, determinism, differential real-Perl
conformance, and bounded compiler/runtime paths, but those later platform
claims require their own specs and receipts.
