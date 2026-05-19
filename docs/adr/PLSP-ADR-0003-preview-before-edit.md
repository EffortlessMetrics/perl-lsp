# PLSP-ADR-0003: Preview before edit

Status: accepted
Date: 2026-05-19
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](../specs/PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0008](../specs/PLSP-SPEC-0008-edit-producing-provider-safety.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)

## Context

Edit-producing providers are not ordinary query providers. A wrong completion,
hover, symbol, or navigation result is visible noise. A wrong rename or safe
delete can damage source.

`perl-lsp` supports increasingly precise compiler-backed and semantic facts,
but Perl remains dynamic. Generated members, runtime imports, typeglobs,
symbolic references, stale compiler facts, and ambiguous workspace identity can
all make an edit look safe when it is not.

The repo already uses previews, provider receipts, blocker reasons, rollback
proof, and narrow live pilots for rename and safe delete. This ADR makes that
architecture decision durable.

## Decision

Edit-producing providers must pass through preview/no-edit, receipt, rollback,
and blocker proof before narrow live cutover.

Rename and safe delete may be conservative by design. They must refuse,
preview, or fall back when proof is missing instead of returning speculative
edits.

Preview commands are product surfaces. They allow users and support workflows to
inspect planned edits, blockers, fallback reasons, and copyable receipts before
any live edit class is broadened.

## Rules

1. Preview comes before live edit expansion.
2. Receipt-only PRs cannot broaden live edit behavior.
3. Rollback proof is required before a new live edit class is promoted.
4. Fresh source-backed identity is required before edits are returned.
5. Generated, dynamic, stale, low-confidence, ambiguous, fallback, and no-source
   facts cannot authorize edits.
6. The server may return `WorkspaceEdit` values to the client, but it must not
   apply those edits itself.
7. Support-tier promotion requires proof commands and explicit status review.
8. PR bodies must state whether the change is preview-only, receipt-only,
   rollback proof, or live cutover.

## Consequences

Positive consequences:

- Rename and safe delete can ship useful narrow slices without pretending broad
  static certainty exists.
- Users get a safer preview path for destructive actions.
- Bug reports can include structured blocker and rollback receipts.
- Future agents have a durable rule that prevents treating edit-producing
  providers like completion or hover.

Tradeoffs:

- Some edits remain unavailable even when facts look promising.
- Live refactor behavior grows more slowly than read-only provider behavior.
- More tests and status review are required before edit cutover.
- Preview commands must be maintained as stable UX, not removed after pilots.

## Alternatives Considered

### Treat rename and safe delete like ordinary providers

Rejected. Query providers can fall back visibly when confidence is low. Edit
providers need a fail-closed path because mistakes change source.

### Allow compiler-backed edits whenever facts are high confidence

Rejected. High confidence alone is not enough for edits. Freshness, source
backing, identity guards, current-source reference checks, workspace-reference
checks, and rollback proof are also required for the relevant edit class.

### Keep previews as temporary scaffolding

Rejected. Preview commands are the user-facing way to understand planned edits
and blockers. They remain useful even after a narrow live pilot exists because
broader edit classes must still be explainable without returning edits.

## Follow-up Obligations

- Keep [PLSP-SPEC-0008](../specs/PLSP-SPEC-0008-edit-producing-provider-safety.md)
  linked from rename and safe-delete cutover work.
- Keep preview commands documented as user-facing product surfaces.
- Keep support tiers explicit that rename and safe delete are
  `partial-live-with-fallback` unless proof promotes a scoped claim.
- Keep generated, dynamic, stale, low-confidence, ambiguous, imported/exported,
  referenced, no-source, non-subroutine, and package-wide boundaries visible in
  provider receipts.

## Status Links

- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [Provider promotion ledger](../project/status/provider_promotion_ledger.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Editor trust user guide](../how-to/EDITOR_TRUST.md)

## Why ADR-worthy

Preview-before-edit is an architecture decision, not only a test policy. It
defines how destructive provider behavior becomes live and why conservative
no-edit behavior is correct when proof is incomplete.
