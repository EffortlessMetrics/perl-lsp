# PLSP-SPEC-0018: Edit authorization contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0008](PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
- [PLSP-ADR-0003](../adr/PLSP-ADR-0003-preview-before-edit.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: rename, safe delete, provider promotion ledger, provider
decision receipts, support tiers

## Current Implementation Status

Rename and safe delete already have preview, blocker, fallback, freshness, and
rollback receipts. They are not yet required to route through one shared Rust
authorization type. This spec defines that target contract so future refactor
work cannot return edit-producing behavior without proving the same states.

This spec does not broaden rename or safe-delete behavior.

## Contract

Any provider that can return source edits must first classify the request as
one of:

```text
Allowed
PreviewOnly
Blocked
Fallback
```

The authorization state is canonical. Raw `WorkspaceEdit` values are not a
proof boundary by themselves.

## Target Code Shape

Future code may introduce a shared type with this shape:

```rust
pub enum EditAuthorization {
    Allowed {
        edits: WorkspaceEdit,
        proof: EditProof,
        rollback: RollbackProof,
    },
    PreviewOnly {
        planned_edits: Vec<TextEdit>,
        explanation: ProviderDecisionV1,
    },
    Blocked {
        reason: BlockerReason,
        explanation: ProviderDecisionV1,
    },
    Fallback {
        provider: FallbackProvider,
        explanation: ProviderDecisionV1,
    },
}
```

Until that type exists, rename and safe-delete implementations, tests, and
receipts must preserve the same four-state semantics.

## Authorization States

### Allowed

`Allowed` means the provider may return non-empty edits to the client. The
server still must not apply edits itself.

Required evidence:

- scoped provider class is promoted in the provider promotion ledger
- fact provenance supports edit behavior
- confidence is high
- facts are fresh for the request
- source identity is accepted
- dynamic, generated/no-source, stale, low-confidence, ambiguous, imported, and
  exported blockers are absent
- rollback proof exists and restores original text
- provider decision receipt explains the allowed state

### PreviewOnly

`PreviewOnly` means the provider can explain planned edits but must return no
live edit application for the request.

Required evidence:

- planned edits are source-backed or explicitly labeled as preview-only
- blocker, fallback, or missing-proof reason is visible
- no live edit is returned
- rollback or no-edit boundary is preserved in the receipt

### Blocked

`Blocked` means the provider refuses the edit-producing action.

Required evidence:

- known blocker reason is present
- provider decision receipt is copyable and user-facing
- no live edit is returned
- fallback behavior, if any, is explicit and safe

### Fallback

`Fallback` means the provider delegates to an already-safe legacy or
workspace-index path instead of using the compiler-backed edit plan.

Required evidence:

- fallback provider is named
- fallback does not claim compiler-backed edit authorization
- stale or partial compiler proof cannot strengthen the fallback claim
- provider decision receipt records the fallback state

## Rename Guards

Rename may authorize edits only when all relevant guards pass:

- source-backed target
- fresh semantic facts
- accepted ambiguity guard
- current-source and workspace-index identity agreement for the scoped live
  class
- no generated/no-source target
- no imported/exported ambiguity
- no dynamic, typeglob, `AUTOLOAD`, or symbolic boundary
- no stale or low-confidence fact
- rollback proof for multi-file edits
- provider decision explanation for Allowed, PreviewOnly, Blocked, or Fallback

Same-file lexical rename and package-local rename remain separate live classes.
Proof for one does not authorize the other.

## Safe-Delete Guards

Safe delete may authorize edits only when all relevant guards pass:

- target is an exact source-backed subroutine
- compiler references are zero
- current-source references are zero
- workspace-index references are zero
- workspace identity guard accepts the request
- no generated/no-source target
- no imported/exported target
- no dynamic boundary
- no stale or low-confidence fact
- rollback proof restores original text
- provider decision explanation for Allowed, PreviewOnly, Blocked, or Fallback

Non-subroutine, package-wide, generated, dynamic, no-source, referenced,
ambiguous, imported/exported, stale, low-confidence, and rollback-failure cases
must return no live edits.

## Valid PR Shapes

Valid PRs under this spec include:

- introducing the shared `EditAuthorization` type
- routing rename or safe-delete preview paths through the shared states
- routing live edit paths through `Allowed` proof
- adding tests that assert Allowed, PreviewOnly, Blocked, and Fallback states
- adding validators that reject raw edit returns without authorization proof
- docs PRs that clarify the authorization boundary without changing behavior

## Invalid PR Shapes

Invalid PRs include:

- broad rename or safe-delete cutover from this spec alone
- returning raw `WorkspaceEdit` without an authorization state
- treating preview as live authorization
- authorizing edits from generated/no-source, dynamic, ambient, unknown, stale,
  low-confidence, ambiguous, imported, or exported evidence
- omitting rollback proof for allowed edits
- applying edits on the server
- support-tier promotion without receipts and support review

## Acceptance

A PR satisfies this spec when:

- every edit-producing provider path touched by the PR names an authorization
  state
- `Allowed` paths have edit proof and rollback proof
- `PreviewOnly`, `Blocked`, and `Fallback` paths return no compiler-backed live
  edits
- blocker and fallback reasons are visible through provider decision receipts
- rename and safe-delete tests assert the authorization state for changed paths
- provider promotion ledger and support-tier claims remain conservative

## Proof Commands

Docs-only PRs for this spec may use:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask check-provider-promotion-ledger
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Code PRs must add focused rename or safe-delete tests for the touched state and
run the affected provider checks.

## Non-goals

- No broad package rename.
- No broad safe-delete.
- No generated-member edit authorization.
- No dynamic/typeglob/`AUTOLOAD` edit authorization.
- No server-side edit application.
- No support-tier promotion from this spec alone.

## Claim Boundaries

This spec may be cited to block or defer edit-producing behavior when proof is
missing. It may not be cited as proof that a provider is allowed to edit; only
an `Allowed` receipt with rollback proof can make that claim.
