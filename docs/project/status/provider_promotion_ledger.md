# Provider Promotion Ledger

> Human-owned. This ledger converts provider proof into explicit promotion,
> fallback, and blocker decisions. It does not generate metrics, broaden live
> behavior, or replace provider-specific receipts.

This page answers:

> Which fact class can move from receipt to scoped live behavior, and what
> still forces fallback or blocks promotion?

Use this ledger after adding provider proof and before changing user-visible
behavior. The source of current evidence remains the provider-specific receipt,
[provider confidence matrix](provider_confidence_matrix.md), and
[provider cutover](provider_cutover.md).

## Operating Rule

Every row must support one of four decisions:

```text
promote
fallback
block
defer
```

A proof row that does not enable one of those decisions should stay out of the
main trust lane until its promotion boundary is clear.

## Ledger

| Surface | Fact class | Current state | Next proof | Promotion condition | Fallback condition | Blocker condition |
| --- | --- | --- | --- | --- | --- | --- |
| Semantic tokens | Subroutine declaration token | `partial live trace slice` | Scoped field/method-call live-trace expansion or another scoped class proof before broader compiler-token promotion | Fresh source-backed compiler span matches exactly one existing live parser/HIR `function` token and emits no new token output | Parser/HIR token output remains live when compiler identity is absent or unmatched | Generated/no-source, dynamic, stale, low-confidence, fallback, or broader compiler-token class |
| Semantic tokens | Method declaration token | `partial live trace slice` | Another scoped field/method-call live-trace expansion or scoped class proof before broader compiler-token promotion | Fresh source-backed `token:method_declaration:` span matches exactly one existing live `method` token, proves `didChange` freshness, and emits no new token output | Parser/HIR token output remains live for unproven method-token shapes | Broader `token:method:` identity, generated/no-source, dynamic, stale, low-confidence, or unmatched span |
| Semantic tokens | Package declaration token | `partial live trace slice` | Another scoped field/method-call live-trace expansion or scoped class proof before broader compiler-token promotion | Fresh source-backed `token:package_declaration:` span matches exactly one existing live `namespace` token, proves `didChange` freshness, and emits no new token output | Parser/HIR token output remains live for unproven namespace-token shapes | Generated/no-source, dynamic, stale, low-confidence, fallback, or unmatched compiler span |
| Semantic tokens | Field declaration token | `support-reviewed scoped proof` | Scoped field/method-call live-trace expansion or another scoped class proof before broader compiler-token promotion | Fresh source-backed `token:field_declaration:` span matches exactly one existing live `variable` token, proves `didChange` freshness, and emits no new token output | Parser/HIR token output remains live for unproven variable-token shapes | Broader variable identity, generated/no-source, dynamic, stale, low-confidence, or unmatched span |
| Semantic tokens | Method call token | `support-reviewed scoped proof` | Scoped field/method-call live-trace expansion or another scoped class proof before broader compiler-token promotion | Fresh source-backed `token:method_call:` span matches exactly one existing live `method` token, proves `didChange` freshness, and emits no new token output | Parser/HIR token output remains live for unproven method-call shapes | Broader `token:method:` identity, generated/no-source, dynamic, stale, low-confidence, or unmatched span |
| Workspace symbols | Source-backed exact symbol | `partial live` | Project-shape quality receipts before broader expansion | Non-empty query against a fresh ready workspace index returns high-confidence source-backed symbols | Empty query, partial index, open-document fallback, or absent ready-index proof | Stale, dynamic, low-confidence, ambiguous, or generated/no-source candidate |
| Workspace symbols | Source-backed generated/framework symbol | `generated-label pilot` | Project-shaped generated/no-source proof before broader generated-symbol expansion | Fresh source-backed generated/framework member has an explicit generated label and a framework declaration anchor with bounded confidence | Keep generated candidate receipt-only when source anchor or label is missing | Virtual/no-source generated member, exact generated method-body claim, stale fact, dynamic boundary, low confidence, ambiguity, partial index, or fallback/noise candidate |
| Rename | Same-file lexical symbol | `narrow live` | Keep unsafe-edit receipts fresh when rename facts change | Current document proves exactly one source-backed `my` or `state` declaration edit and references remain in the same file | Legacy rename or no edit when the scoped lexical proof is absent | Generated member, package-wide target, import/export ambiguity, dynamic boundary, stale fact, low confidence, or ambiguous candidate |
| Rename | Package-local source-backed plan | `package-local pilot` | Keep project-shaped unsafe-edit and edit-freshness receipts fresh; broader promotion deferred | Materialized source-backed semantic edit set exactly matches the workspace source/ambiguity guard and rollback/no-edit proof is preserved | Existing safe workspace-index path may answer when the compiler plan is partial or current-source coverage requires a broader fresh edit set | Imported/exported fact, generated member, package-wide edit, dynamic/typeglob/AUTOLOAD boundary, stale fact, low confidence, missing compiler proof, ambiguous identity, or source/ambiguity guard mismatch |
| Safe delete | Exact static source-backed subroutine | `source-backed pilot` | Additional project-shaped false-allow and blocker receipts | Compiler plan is allowed, target is fresh high-confidence source-backed subroutine, exact references are zero, and rollback proof restores original text | `perl.previewSafeDelete` returns no-edit explanation when live delete proof is incomplete | Imported/exported symbol, generated member, non-subroutine target, no-source target, typeglob alias, AUTOLOAD, symbolic ref, dynamic require, stale fact, low confidence, fallback fact, or rollback failure |
| Provider explanation | Request-local provider decision | `partial live schema` | Keep schema stable as providers add live slices | Provider request emits or persists fact source, confidence, freshness, source-backed state, fallback state, blocker, claim boundary, and user message | `perl.explainProviderDecision` can replay the latest trace or accept a request-local receipt payload | Missing receipt, unredacted workspace data, unsupported schema version, or provider-specific fields that contradict normalized fallback/blocker state |

## Promotion Discipline

- Promote only one fact class at a time.
- Keep live behavior bounded to the exact promotion condition in this ledger.
- Use fallback when the proof is incomplete but the existing provider can answer
  safely.
- Block edit-producing behavior when freshness, source backing, ambiguity, or
  rollback proof is missing.
- Record the next compiler substrate gap after a class is promoted or blocked.
