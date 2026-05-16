# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

For a row-per-provider receipt summary with fact source, confidence, freshness,
fallback, runtime comparison, live state, and next proof, see
[provider confidence matrix](provider_confidence_matrix.md).
For the Real Perl Editor Trust v1 routing dashboard that ties provider state to
support claims, real-workspace receipts, and next PRs, see
[real_perl_editor_trust_v1.md](real_perl_editor_trust_v1.md).

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current semantic-shadow compare artifact records forty-eight deterministic
  receipts across definition, references, completion, hover, diagnostics, workspace-symbol,
  document-symbol, semantic-token, rename, and safe-delete surfaces.
- Definition/reference shadow proof now records imported-symbol,
  framework-generated, dynamic-boundary, low-confidence fallback, stale fact, and
  real-workspace quality candidate/occurrence traces.
- Definition now has a narrow live exact/imported runtime slice: a single
  fresh, high-confidence, source-backed `ExactAst`, explicit import, default
  export, or export-tag candidate can drive `textDocument/definition`;
  generated/no-source, dynamic, stale, low-confidence, and ambiguous candidates
  retain legacy fallback.
- References now have a narrow live source-backed runtime slice: fresh,
  high-confidence, source-backed `ExactAst`, `ImportExportInference`, or
  `LiteralRequireImport` occurrence references can drive
  `textDocument/references` when declaration inclusion is off; generated/no-source,
  dynamic, stale, low-confidence, ambiguous, and declaration-including requests
  retain legacy fallback.
- Completion shadow proof now records compiler visible-symbol candidate deltas,
  generated-member labels, and dynamic-boundary blockers. A narrow live
  completion slice promotes only high-confidence imported/exported
  visible-symbol facts; generated and dynamic-boundary candidates remain
  shadowed or blocked.
- The Mojolicious scenario 28 completion ranking receipt now records
  real-workspace visible-symbol candidate counts, top-N churn, useful/noisy
  additions, generated labels, and dynamic/fallback labels without broadening
  live completion behavior.
- The Mojolicious scenario 29 hover provenance receipt now records exact,
  imported, generated/framework, dynamic-shaped, module-resolution, and
  fallback/missing-fact hover surfaces without broadening hover behavior.
- The Mojolicious scenario 30 navigation quality receipt now records
  definition/reference result counts and valid LSP shapes for module-resolution,
  exact-local, imported-symbol, dynamic-boundary-shaped, and
  declaration-including probes without broadening navigation behavior.
- Hover provenance proof now records imported-symbol, framework-generated,
  dynamic-boundary, and fallback paths with typed fact-source traces and source /
  confidence labels.
- Hover now has a narrow live runtime slice for traced compiler fact,
  framework-adapter, and dynamic-boundary hover paths. Legacy hover remains the
  fallback when compiler facts are absent, stale, or only legacy-equivalent.
- Diagnostics now have a narrow live cutover for high-confidence imported and
  generated visible-symbol facts. Ambiguous, low-confidence, and
  dynamic-boundary cases remain fallback or blocked instead of being silently
  suppressed.
- Rename and safe-delete now have boundary-shadowed proof for exact static
  allow decisions plus dynamic-boundary, stale compiler fact, low-confidence,
  and generated-member blockers. Runtime blocker UX receipts compare live
  rename / symbol safe-delete request paths with compiler plans for exact
  static, dynamic-boundary, generated-member, stale-fact, and low-confidence
  cases. Mojolicious scenario 35 adds a real-workspace rename unsafe-edit
  receipt for exact local lexical edits, generated/dynamic no-edit boundaries,
  and open-document freshness without broadening live refactor behavior.
  Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt and
  proves generated `has` accessors return no rename edits. Rename also has a
  narrow live same-file lexical slice for sigiled variables when the current
  document proves exactly one `my` or `state` declaration edit. Broad
  compiler-backed, package-wide, generated, dynamic, stale, and low-confidence
  rename remain blocked or fallback/shadow data. A scoped package/compiler-backed
  pilot proof now classifies source-backed definition/reference plans as
  receipt-only evidence without enabling live package rename, and the runtime
  package-pilot receipt records the real-workspace empty-plan boundary with zero
  live package-rename edits.
- Mojolicious scenario 36 adds a real-workspace safe-delete warning receipt for
  `workspace/willDeleteFiles` when `lib/Mojolicious/Static.pm` has dependent
  workspace files. It proves file-delete warning UX only; symbol-level
  safe-delete remains boundary-shadowed.
- Workspace symbols now have source/freshness and real-workspace quality shadow
  proof for fresh compiler facts, framework-generated candidates,
  dynamic-boundary blockers, stale compiler facts, and candidate/noise deltas.
  Non-empty queries against the ready workspace index now persist and report a
  narrow source-backed/high-confidence live trace. Empty-query, partial-index,
  open-document fallback, generated/no-source, stale, dynamic, and ambiguous
  compiler candidates remain fallback or gated.
- Document symbols now have a narrow live source-backed parser-syntax slice for
  fresh, high-confidence `ExactAst` symbols. Framework-generated/no-source,
  dynamic-boundary, stale, low-confidence, and ambiguous candidates remain
  gated or fallback-only.
- Document symbols and workspace symbols now have runtime quality receipts that
  call the live `textDocument/documentSymbol` and `workspace/symbol` handlers
  and capture live provider counts and results. Document-symbol receipts now
  include source-backed compiler symbol counts and fact-source traces; workspace
  symbols remain no-live-behavior-change receipts.
  Seven BDD receipt tests cover document symbols (provider field,
  source-backed live cutover, count integrity, symbol presence, shadow state,
  notes proof, unknown-URI graceful handling) and eight tests cover workspace
  symbols (provider field, no-live-behavior-change, count integrity, query echo,
  shadow state, notes proof, empty-query, no-match query). These receipts
  complete the runtime integration proof step for both surfaces.
- The Mojolicious scenario 32 document-symbol receipt records live
  source-backed package/sub symbols, generated `has` candidate counts,
  dynamic-boundary-shaped names, LSP shape validity, missing-symbol counts, and
  edit freshness without broadening document-symbol provider behavior.
- The Mojolicious scenario 33 workspace-symbol receipt records live-provider
  query latency, repeated-query count stability, useful/noisy hits, generated
  candidate gating, dynamic-boundary-shaped names, and edit freshness without
  broadening workspace-symbol provider behavior.
- Semantic tokens now have source/freshness shadow proof for explicit
  parser/HIR classifications, compiler-backed classifications,
  dynamic-boundary blockers, and stale compiler facts. These receipts do not
  broaden live semantic-token behavior.
- Semantic tokens now have runtime integration quality receipts
  (`semantic_tokens_runtime_quality_receipt`) that exercise the live
  `textDocument/semanticTokens/full` handler and capture token count, shadow
  state, a narrow compiler-backed subroutine-declaration live-pilot span match,
  and a quality-proof note. Eleven BDD tests confirm receipt correctness,
  no-live-behavior-change invariant, no-token-output-change invariant, and
  token-count parity with the live handler. Broader compiler-fact token
  candidates remain pending staged cutover.
- The Mojolicious scenario 34 semantic-token receipt records live token counts,
  LSP 5-tuple/span validity, expected source-backed token hits,
  dynamic-boundary string non-promotion, and edit freshness without broadening
  semantic-token provider behavior.
- Dancer2 scenario 38 adds second-project semantic-token quality proof for
  package, DSL, app, typeglob-boundary, and edit-freshness token shapes without
  broadening semantic-token provider behavior.
- Other provider surfaces remain trace/proof infrastructure only until their
  own cutover proof lands.

## Navigation Live Quality Dashboard

Definition and references now have a narrow live loop for source-backed,
high-confidence facts. This dashboard is the guardrail before broadening
navigation to generated, dynamic, or lower-confidence candidates.

The source of truth for current receipt counts remains
[semantic_shadow_compare.md](semantic_shadow_compare.md); this table records
which navigation slices are live, fallback-only, or blocked.

| Slice | Live status | Receipt source | Fallback / blocker rule |
| --- | --- | --- | --- |
| `definition_exact_live` | `partial live` | [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindDefinition` release-readiness receipts | Single fresh, high-confidence, source-backed `ExactAst` candidate may answer live; otherwise legacy fallback. |
| `definition_imported_live` | `partial live` | [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindDefinition` import/export receipts | Single fresh, high-confidence explicit import, default export, or export-tag candidate may answer live; ambiguous or stale import facts fall back. |
| `references_exact_live` | `partial live` | [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindReferences` release-readiness receipts | Fresh, high-confidence, source-backed exact occurrences may answer live when declaration inclusion is off. |
| `references_imported_live` | `partial live` | [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindReferences` import/export receipts | Fresh, high-confidence, source-backed `ImportExportInference` or `LiteralRequireImport` occurrences may answer live when declaration inclusion is off. |
| `generated_fallback` | `fallback / shadow` | Framework-generated `FindDefinition` and `FindReferences` traces | Generated or virtual members without exact source ranges stay labeled fallback/shadow data. |
| `dynamic_blocked` | `blocked / fallback` | Dynamic-boundary navigation traces | Dynamic-boundary candidates block compiler-backed navigation and keep legacy fallback. |
| `stale_blocked` | `blocked / fallback` | Stale-fact navigation traces | Stale compiler facts cannot answer as confirmed live navigation results. |
| `ambiguous_or_low_confidence_fallback` | `fallback / shadow` | Low-confidence and ambiguous navigation traces | Low-confidence or ambiguous candidates may inform receipts but cannot drive live navigation. |

## State Definitions

Provider states are acceptance tiers, not release labels. A provider can move to
the next tier only when the evidence for that tier is committed or generated by
the relevant receipt command.

| State | Meaning | Exit gate |
| --- | --- | --- |
| `unavailable` | No provider-specific compiler-fact proof exists yet, or the surface has no owner issue. | Add an owner issue plus fixture-backed fact evidence. |
| `fixture-backed` | The fact layer has deterministic fixtures, but provider behavior is still legacy or unproven. | Add provider-specific shadow receipts with source, provenance, confidence, freshness, fallback, and dynamic-boundary state. |
| `shadowed` | Legacy and compiler-fact outcomes are compared in receipts without changing live runtime behavior. `ranked-shadowed` and `boundary-shadowed` are shadow subtypes for candidate ranking and refactor-blocker proof. | Show zero unacceptable fixture regressions, explicit stale/dynamic handling, and a scoped live-cutover plan. |
| `provenance-backed` | Runtime or receipt output can explain the fact source, provenance, confidence, and freshness for the scoped answer. | Prove fallback behavior and real-workspace quality before broadening beyond the scoped path. |
| `partial live` | One or more high-confidence fact families are live with legacy fallback and fail-closed stale/dynamic handling. | Add real-workspace quality receipts, rollback/fallback proof, and provider-specific noise or precision deltas before expanding. |
| `live` | Compiler facts are the default provider source for the scoped surface while traces, fallback, and dynamic-boundary blockers remain available. | Keep receipts fresh when fact families or provider behavior change. |
| `blocked` | Proof found a safety, freshness, noise, or precision issue that prevents live behavior. | Close the blocker with a targeted fix and rerun the provider proof lane. |

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives, plus high-confidence imported/generated visible-symbol facts; fallback diagnostics remain available | Broader false-positive / false-negative fixture receipts before any additional diagnostic families move live |
| Completion | `partial live / shadowed` | Existing completion paths remain live; high-confidence imported/exported compiler visible-symbol facts can contribute live candidates with legacy fallback; semantic-shadow fixtures and the Mojolicious scenario 28 ranking receipt trace generated labels, rank deltas, useful/noisy additions, and dynamic-boundary blockers without promoting those families | Additional real-workspace candidate quality across more project shapes before any broader generated, dynamic, method, or workspace-wide live cutover |
| Hover | `partial live / provenance-backed` | Runtime hover uses compiler-fact cutover for traced compiler fact, framework-adapter, and dynamic-boundary paths when fresh workspace facts are available; legacy hover remains fallback; hover cutover/shadow code labels imported, generated, dynamic-boundary, and fallback paths with fact-source traces and source/confidence text; Mojolicious scenario 29 records project-shaped hover surfaces without behavior changes | Additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Definition / goto | `partial live / ranked-shadowed` | A single fresh, high-confidence, source-backed `ExactAst`, explicit import, default export, or export-tag candidate can drive live `textDocument/definition` with legacy fallback. Generated/no-source, dynamic-boundary, low-confidence, ambiguous, stale, and broader real-workspace candidates remain traced as fallback/shadow proof. Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes without behavior changes. | Broader generated/dynamic migration requires additional project-shape receipts and no false-exact source-location claims |
| References | `partial live / ranked-shadowed` | Fresh, high-confidence, source-backed `ExactAst`, `ImportExportInference`, or `LiteralRequireImport` occurrence references can drive live `textDocument/references` when `includeDeclaration=false`; generated/no-source, dynamic-boundary, low-confidence, ambiguous, stale, and declaration-including requests remain traced as fallback/shadow proof. Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes without behavior changes. | Broader references migration requires precision/recall receipts for generated, coderef, typeglob, and declaration-including cases |
| Rename | `partial live lexical / boundary-shadowed compiler facts` | Same-file sigiled lexical rename can use current-document scoped AST proof only when exactly one `my` or `state` declaration edit is proven; rename plan receipts still trace exact static edits, dynamic-boundary blockers, stale compiler facts, low-confidence ambiguity, runtime blocker UX notes, live-vs-compiler exact-static receipt data, Mojolicious scenario 35 and Dancer2 scenario 37 real-workspace unsafe-edit proof, the scoped lexical cutover in [#8915](https://github.com/EffortlessMetrics/perl-lsp/pull/8915), `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` rollback proof, the receipt-only package/compiler-backed pilot classifier plus real-workspace empty-plan runtime boundary, and `perl.previewPackageRename` no-edit preview UX with rollback/no-edit receipts for empty-plan and imported-call edit-noise previews before any broad compiler-backed refactor behavior | Dedicated package/compiler-backed live pilot proof with rollback and fallback guardrails before broader refactor migration |
| Safe delete | `boundary-shadowed` | Safe-delete receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, stale compiler facts, runtime blocker UX notes, live-vs-compiler exact-static receipt data, Mojolicious scenario 36 file-delete warning UX, Dancer2 and RealBaseline symbol-level blocker/allowed request shapes, and `perl.previewSafeDelete` scoped no-edit UX before any live compiler-backed symbol delete edits | Actual symbol-delete edit cutover proof with rollback before any broader refactor migration |
| Workspace symbols | `partial live source-backed` | Existing workspace index remains the live provider source; non-empty ready-index results can answer live with source-backed/high-confidence traces; semantic-shadow fixtures still trace fresh compiler, generated, dynamic-boundary, stale fact, and real-workspace quality candidates; runtime quality receipts capture source-backed ready-index counts/results; Mojolicious scenario 33 records live-provider noise, generated candidate gating, dynamic-boundary-shaped names, and edit freshness | Additional generated/dynamic/noise receipts before broader workspace-symbol expansion |
| Document symbols | `partial live source-backed` | Fresh, high-confidence, source-backed parser-syntax `ExactAst` symbols can drive live `textDocument/documentSymbol` results with fallback retained for astless documents and gated generated/no-source, dynamic-boundary, stale, low-confidence, and ambiguous candidates. Semantic-shadow fixtures still trace explicit syntax, generated, dynamic-boundary, and stale fact candidates; runtime quality receipts capture live provider counts/results plus source-backed compiler traces; Mojolicious scenario 32 records real-workspace symbol quality, generated candidate counts, and edit freshness. | Generated-label proof plus additional real-workspace document-symbol receipts before generated, dynamic, or broader symbol cutover |
| Semantic tokens | `partial live token-class pilot` | Existing parser/token provider remains the broad live source; semantic-shadow fixtures trace parser/HIR, compiler-backed, dynamic-boundary, and stale fact candidates; runtime quality receipts capture live token count, shadow state, no-token-output-change proof, and one source-backed compiler-fact subroutine-declaration class matched to existing live `function` token output; Mojolicious scenario 34 and Dancer2 scenario 38 record project-shaped token/span validity and edit freshness | Additional compiler-backed token-class receipts for generated, dynamic, stale, and fallback boundaries before broader cutover |

## Cutover Rules

- Do not cut a provider over just because a fact exists.
- Every provider answer that uses compiler facts should be able to identify
  source, provenance, confidence, and fallback state where relevant.
- Generated and dynamic-boundary answers must be labeled honestly.
- Provider regressions should first appear in shadow compare or scorecard
  receipts, not as live editor behavior.

## Tracking

- Provider cutover umbrella: [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197)
- Hover live provenance slice: [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369)
- Completion live visible-symbol slice: [#8374](https://github.com/EffortlessMetrics/perl-lsp/issues/8374)
- Definition/reference real-workspace quality receipts: [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382)
- Definition/reference runtime integration and live-provider quality receipts: [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462)
- Definition exact/imported live cutover lane: [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803)
- References exact/static live cutover lane: [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828)
- References imported/exported live cutover lane: [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836)
- Rename/safe-delete runtime blocker UX receipts: [#8464](https://github.com/EffortlessMetrics/perl-lsp/issues/8464)
- Workspace-symbol source/freshness proof: [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353)
- Workspace-symbol real-workspace quality receipt: [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378)
- Document-symbol source/freshness proof: [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359)
- Semantic-token source/freshness proof: [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360)
- Fact-source trace receipt slice: [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305)
- Compiler facts: [compiler_facts.md](compiler_facts.md)
- Semantic scorecard: [semantic_scorecard.md](semantic_scorecard.md)
- Semantic shadow compare: [semantic_shadow_compare.md](semantic_shadow_compare.md)
- Provider confidence matrix:
  [provider_confidence_matrix.md](provider_confidence_matrix.md)
- Real-workspace baseline tracker: [#7949](https://github.com/EffortlessMetrics/perl-lsp/issues/7949)
