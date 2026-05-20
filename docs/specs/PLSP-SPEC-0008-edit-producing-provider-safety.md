# PLSP-SPEC-0008: Edit-producing provider safety

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
- [PLSP-ADR-0003](../adr/PLSP-ADR-0003-preview-before-edit.md)
Linked specs:
- [PLSP-SPEC-0018](PLSP-SPEC-0018-edit-authorization-contract.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: provider confidence matrix, provider cutover, support tiers,
semantic shadow compare, UX capability dashboard

## Contract

Any provider that returns a `WorkspaceEdit` must prove safety before producing
edits.

Edit-producing providers are stricter than read-only providers. Completion,
hover, symbols, diagnostics, and navigation can be wrong in visible ways;
rename and safe delete can damage source. They must fail closed when proof is
missing.

This spec sharpens [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
for providers that can return edits. It does not replace the provider receipt
contract; it defines the additional safety bar for edit output.
The shared Allowed/PreviewOnly/Blocked/Fallback state model is defined by
[PLSP-SPEC-0018](PLSP-SPEC-0018-edit-authorization-contract.md).

## Shared Safety Rules

Edit-producing providers must satisfy these rules before returning non-empty
edits:

- preview before edit
- rollback before promotion
- source-backed proof before live edits
- freshness proof before edit
- identity guard before edit
- fallback or no-edit behavior when proof is missing
- generated, dynamic, stale, low-confidence, ambiguous, fallback, and no-source
  facts cannot authorize edits
- the server returns `WorkspaceEdit` values to the client but does not apply
  them itself

Preview commands are product surfaces. They are not temporary scaffolding and
must remain available for broader request shapes that are not live edit classes.

## Rename Requirements

Rename may return edits only for scoped classes that have explicit proof.

Same-file lexical rename is scoped to current-document source proof. It may
return edits only when the current source proves exactly one `my` or `state`
declaration identity and all returned edits remain inside the proven scope.

The package-local pilot may return edits only when all of these hold:

- the request targets a fresh source-backed semantic edit set
- the materialized semantic edit set exactly matches the workspace
  source/ambiguity guard
- the current-source and workspace-index evidence agree on identity
- rollback or no-edit behavior is proven
- imported, exported, ambiguous, generated, dynamic, stale, low-confidence,
  package-wide, missing-proof, and partial unsafe plans return no edits or
  fall back to the existing safe path
- `didChange` freshness has been proven for the request shape before edits are
  returned

Partial semantic plans may fall back only to an already-safe legacy or
workspace-index path. They must not become broader compiler-backed rename.

## Safe-Delete Requirements

Safe delete may return edits only for exact unreferenced source-backed
subroutine deletion.

The live pilot may return a delete `WorkspaceEdit` only when all of these hold:

- compiler allow proof is fresh and high-confidence
- the source guard resolves an exact source-backed subroutine definition
- current-source references are zero
- workspace-index references are zero
- workspace identity guard accepts the target
- rollback proof restores the original text

These request classes must return no edit:

- non-subroutine target
- package-wide target
- generated or no-source symbol
- dynamic-boundary symbol
- stale fact
- low-confidence fact
- imported or exported symbol
- referenced symbol
- current-source referenced target
- workspace-index referenced target
- ambiguous identity
- fallback/no-source candidate
- rollback failure

`workspace/willDeleteFiles` warning behavior is separate from symbol safe delete
and does not authorize symbol deletion.

## Valid PR Shapes

Valid PRs under this spec include:

- preview-only PRs that expose planned edits, blockers, and no-edit decisions
- receipt PRs that prove stale, generated, dynamic, low-confidence, referenced,
  imported/exported, or ambiguous cases return no edit
- rollback PRs that prove a returned `WorkspaceEdit` can be inverted
- narrow live pilot PRs that return edits only for one source-backed class with
  fallback and blocker proof
- status PRs that review support claims after proof already landed

Every PR must state whether it is preview-only, receipt-only, rollback proof,
or live cutover.

## Invalid PR Shapes

Invalid PRs include:

- broad live rename or safe-delete cutover from receipt-only proof
- edits authorized by stale, generated, dynamic, low-confidence, fallback, or
  no-source facts
- package-wide rename or deletion without an explicit safety spec and proof
- imported/exported or ambiguous package-local rename returning edits
- referenced, generated/no-source, non-subroutine, or package-wide safe delete
  returning edits
- server-side edit application
- support-tier promotion without proof commands and status review
- docs/status changes that imply broader edit behavior without provider proof

## Acceptance

An edit-producing provider PR satisfies this spec when:

- returned edits are limited to the scoped live class named by the PR
- no-edit and fallback paths are tested for near-miss cases
- blocker reasons are user-visible through provider receipts or preview output
- rollback proof exists before a live edit class is promoted
- current-source freshness is proven for request shapes affected by edits
- dynamic, generated, stale, low-confidence, ambiguous, imported/exported, and
  no-source cases cannot authorize edits
- the PR body states claim boundary, fallback/no-edit behavior, and validation
- support-tier rows are updated only when the proof covers the claim

## Proof Commands

Focused edit-provider proof uses provider-specific tests plus the support gates:

```bash
cargo test -p perl-lsp-rs-core --lib rename_shadow --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs-core --lib safe_delete_shadow --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs-core --lib rename_package_pilot --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs --lib refactor_runtime_blocker --profile agent --locked -- --nocapture
cargo xtask semantic-shadow-compare --check
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
git diff --check
```

Live cutover PRs must add narrower runtime receipt commands for the specific
rename or safe-delete class being promoted.

Docs-only PRs for this spec may use:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
git diff --check
```

## Non-goals

- No broad compiler-backed rename cutover.
- No broad symbol deletion.
- No package-wide rename or package-wide deletion.
- No generated/no-source edit authorization.
- No dynamic Perl edit authorization.
- No server-side application of workspace edits.
- No support-tier promotion from this spec alone.

## Claim Boundaries

Rename remains `partial-live-with-fallback` unless support tiers, provider
confidence, and runtime receipts prove a narrower or broader claim. Same-file
lexical rename and the package-local pilot are distinct live classes; proof for
one does not promote the other.

Safe delete remains `partial-live-with-fallback` for exact unreferenced
source-backed subroutine deletion. Proof for a source-backed subroutine does
not promote generated, dynamic, non-subroutine, package-wide, imported/exported,
referenced, fallback, or no-source deletion.

Preview and receipt PRs may claim explainable no-edit behavior. They may not
claim live edit behavior unless they also prove rollback, freshness, identity,
and fallback boundaries.

## Current Evidence Owners

Current state and evidence live outside this spec:

- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [Provider promotion ledger](../project/status/provider_promotion_ledger.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Editor trust user guide](../how-to/EDITOR_TRUST.md)

Do not copy current receipt rows, generated parser state, active PR order, or
one-off CI failures into this spec.
