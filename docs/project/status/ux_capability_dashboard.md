# Semantic UX Capability Dashboard

This dashboard maps parser accuracy and semantic proof rails to editor-facing
Perl language intelligence.

It answers one question:

> Does the editor feel Perl-aware?

This dashboard is **descriptive**, not generative: parser and semantic metrics
are consumed from existing artifacts (linked below). Updates here record how
those metrics translate into user-facing capability claims.

For the proof-command support claim map, see
[SUPPORT_TIERS.md](SUPPORT_TIERS.md).

## Status vocabulary

| Status | Meaning |
|---|---|
| `legacy` | Provider still uses legacy/local behavior. |
| `semantic-shadow` | Semantic path is measured but not primary. |
| `semantic-live` | Semantic path drives user-visible behavior. |
| `semantic-live-with-fallback` | Semantic path drives behavior when available; legacy path remains fallback. |
| `insufficient_data` | Not enough proof to make a durable claim. |

## Data ownership

This dashboard consumes existing parser accuracy and semantic scorecard
artifacts. It does not recompute metrics and does not own their source values.

Source-of-truth artifacts:

| Input family | Source of truth |
|---|---|
| Parser accuracy | [parser.md](parser.md) and parser-accuracy artifacts |
| Semantic facts / readiness | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) and semantic scorecard artifacts |
| Shadow comparison | semantic shadow-compare receipts |
| Provider confidence / cutover | [provider_confidence_matrix.md](provider_confidence_matrix.md) and [provider_cutover.md](provider_cutover.md) |
| Support claim map | [SUPPORT_TIERS.md](SUPPORT_TIERS.md) |
| UX status | this dashboard, manually maintained from source artifacts |

When a source value changes, update the source artifact first, then refresh
the corresponding row here. Never edit a row in this dashboard to "fix" a
number that disagrees with its source — fix the source.

## TBD policy

`TBD` means the row shape is defined but no durable value has been assigned
yet. A `TBD` row should become one of:

- `legacy`
- `semantic-shadow`
- `semantic-live`
- `semantic-live-with-fallback`
- `insufficient_data`

during a follow-up population PR. `TBD` is not a permanent status — it is a
placeholder that signals "structure is set, value pending."

## Status transition rules

| From | To | Requirement |
|---|---|---|
| `legacy` | `semantic-shadow` | Semantic path exists and has shadow / proof receipts |
| `semantic-shadow` | `semantic-live` | Provider uses semantic path as primary behavior |
| `semantic-live` | `semantic-live-with-fallback` | Provider uses semantic path when indexed data exists and preserves legacy fallback |
| any | `insufficient_data` | Proof source is missing, stale, or too thin |
| any | `legacy` | Semantic path removed or disabled |

A transition is a documentation change in this dashboard *plus* a link to
the receipt that proves the new state. Promotions without a receipt should
remain at the previous level.

## First population pass

The first population PR fills only rows backed by durable artifacts:

- existing scorecard receipts
- existing shadow-compare receipts
- existing parser-accuracy receipts
- merged provider behavior already in production

It does **not**:

- infer values from code inspection alone
- promote rows to `semantic-live` without a runtime receipt
- copy numbers that may go stale; prefer linking to the source artifact
- expand the dashboard's scope into other rails

## Parser accuracy inputs

Compact summary only. Full detail lives in the parser accuracy status
artifact; this table consumes it without mirroring numbers that live elsewhere.

| Input | Current read | Why it matters |
|---|---:|---|
| `fixture_count` / `family_count` | 25 / 25 | Denominator quality |
| `line_construct_f1` | 0.9 (n=81) | Source-shape understanding |
| `ast_node_kind_f1` | 1.0 (n=9) | AST structural accuracy |
| `symbol_decl_f1` | 1.0 (n=18) | Declaration extraction |
| `symbol_ref_f1` | 1.0 (n=2) | Reference extraction |
| `dynamic_false_precision_count` | 0 (n=1) | Perl dynamic safety |
| `fast_path_wrong_result_count` | 0 (n=1) | Incremental / fast-path safety |
| `failure_packet_count` | `insufficient_data` | Not surfaced as a named row in [parser.md](parser.md) |
| `insufficient_data_count` | 52 rows preserved | Honesty about unproven rows |

See [parser.md](parser.md) for the canonical parser corpus and coverage view.

## Semantic scorecard inputs

Compact summary only. Full detail lives in the semantic scorecard and
release-readability dashboards.

| Input | Current read | Why it matters |
|---|---:|---|
| `declaration_facts` | 42 (16/16 fixtures) | Symbol declarations |
| `occurrence_facts` | 26 (16/16 fixtures) | Uses / references |
| `definition_candidates` | 42 (16/16 fixtures) | Goto / hover / rename substrate |
| `reference_edges` | 1 (16/16 fixtures) | References and safe edits |
| `import_specs` | 11 (16/16 fixtures) | Visibility and diagnostics |
| `export_facts` | 3 (16/16 fixtures) | Completion / rename safety |
| `package_graph_edges` | 2 (16/16 fixtures) | Inheritance / roles / methods |
| `method_candidates_fixture_pass_rate` | 100% (pass) | Method completion |
| `rename_plan_pass_rate` | 100% (pass; `rename_unsafe_edit_count = 0`) | Safe rename |
| `safe_delete_plan_pass_rate` | 100% (pass) | Safe delete |
| `undefined_symbol_false_positive_fixture_rate` | 0% (pass) | Diagnostic trust |
| `visible_symbols_fixture_pass_rate` | 100% (pass) | Completion and hover visibility |

See [semantic_capability_dashboard.md](semantic_capability_dashboard.md) for the
release-readable view, and `semantic_scorecard.md` / `semantic_scorecard.json`
for the underlying receipts.

## Editor UX capability rows

One row per LSP surface. Each row names its proof source and a concrete next
improvement so the dashboard identifies leverage as well as state.

| UX surface | Status | Proof source | Current user-facing claim | Current limits | Next improvement |
|---|---|---|---|---|---|
| Completion | `semantic-live-with-fallback` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) `completion_import_pass_rate = 100%`; [semantic_scorecard.md](semantic_scorecard.md) `completion_import_fixture_pass_rate = pass`; [semantic_shadow_compare.md](semantic_shadow_compare.md) `completion_live_visible_import_candidates`; [#8374](https://github.com/EffortlessMetrics/perl-lsp/issues/8374) runtime visible-symbol fixtures; [#9502](https://github.com/EffortlessMetrics/perl-lsp/pull/9502) receiver-fact pilot | Import/export visibility passes the deterministic fixtures, including empty-import suppression and export-tag expansion. Runtime completion can surface high-confidence imported/exported compiler visible-symbol candidates with source/provenance/confidence/freshness labels and legacy fallback. The receiver-fact pilot can surface exact method candidates from fresh high-confidence source-backed receiver facts while preserving fallback. | Real-workspace coverage is still bounded. Generated/no-source, dynamic-boundary, unknown, stale, low-confidence, broad method, and workspace-wide completion candidates remain shadowed, fallback, or separately gated. | Add real-workspace receiver-quality receipts before broader completion cutover |
| Method completion | `partial-live source-backed receiver pilot` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) `method_completion_shadow_or_cutover_status` (guarded cutover; 0 regressions, 0 unavailable receipts); [#7901](https://github.com/EffortlessMetrics/perl-lsp/pull/7901) literal `bless` receiver inference; [#7917](https://github.com/EffortlessMetrics/perl-lsp/pull/7917) typed receiver-evidence provenance; [#7920](https://github.com/EffortlessMetrics/perl-lsp/pull/7920) receiver-evidence detail text; [#7926](https://github.com/EffortlessMetrics/perl-lsp/pull/7926) medium-confidence receiver detail labels; [#7930](https://github.com/EffortlessMetrics/perl-lsp/pull/7930) bounded low-confidence unknown-receiver fallback; [#9502](https://github.com/EffortlessMetrics/perl-lsp/pull/9502) source-backed receiver pilot | Exact receiver evidence drives package-scoped method completions where the request has proven source-backed/high-confidence receiver facts. The source-backed hash-slot pilot ranks exact receiver candidates above fallback and labels them with receiver evidence. True Unknown receivers keep bounded low-confidence fallback candidates from used modules plus the current package graph, and dynamic hash keys preserve fallback instead of becoming exact hash-slot evidence. | High-confidence evidence remains unlabelled by design except where receiver detail is already part of the method-candidate explanation. Dynamic receivers remain fail-closed or fallback-preserving according to their proven class. There is no all-workspace fallback; fallback is bounded to used modules plus the current package graph. Generated/no-source, stale, low-confidence, ambiguous, and broader receiver forms remain gated until separate proof lands. | Add real-workspace receiver-quality receipts and additional receiver-form proof before broader method completion expansion |
| Hover | `semantic-live-with-fallback` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records four schema-fixture `Hover` provenance receipts: imported-symbol, framework-generated, dynamic-boundary, and fallback paths; [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369) adds the runtime imported-symbol hover fixture; [Mojolicious hover provenance receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_29_mojolicious_hover_provenance.rs) records project-shaped hover surfaces. | Hover provenance is fixture-backed with typed source / provenance / confidence / freshness traces and source labels. Runtime hover now uses fresh compiler-fact cutover output for traced imported/generated/dynamic paths and preserves legacy hover as fallback. Mojolicious scenario 29 records exact, imported, generated/framework, dynamic-shaped, module-resolution, and fallback/missing-fact hover surfaces without behavior changes. | One Mojolicious hover receipt is not broader project or live-cutover proof; stale and wider generated/dynamic quality remain separately gated. | Add additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Diagnostics | `semantic-live-with-fallback` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md#live-semantic-diagnostics) `dynamic_diagnostics_live`; [semantic_scorecard.md](semantic_scorecard.md) `undefined_symbol_false_positive_fixture_rate = 0%`; [dynamic_diagnostics_suppression_tests.rs](../../../crates/perl-lsp-rs/tests/dynamic_diagnostics_suppression_tests.rs) | Push and pull diagnostics suppress false `PL109 UnquotedBareword` results for indexed `eval "sub NAME"` and `Foo->import(@names)` evidence. Legacy diagnostics remain the fallback when semantic data is unavailable. | Suppression is evidence-gated and order-aware. Missing semantic index, unknown ordering, unrelated names, non-literal `eval $code`, cross-file `AUTOLOAD`, symbolic dereference, and truly dynamic sources fail closed unless indexed evidence proves the specific bareword may be visible at the diagnostic point. | Add #7948 order-aware fixtures and #7949 real-workspace semantic baseline |
| Goto definition | `partial-live exact/imported` | [semantic_scorecard.md](semantic_scorecard.md) `definition_candidates = available` (16/16 fixtures), `definition_shadow_regressions = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) release-readiness `FindDefinition` receipts trace exact/static, imported, generated, dynamic-boundary, low-confidence fallback, stale, and real-workspace quality candidates; [Mojolicious navigation quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_30_mojolicious_navigation_quality.rs) records project-shaped definition probes; [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) tracks the navigation quality receipt; [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462) tracks runtime quality receipts; [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803) tracks the exact/imported live cutover lane; [provider_cutover.md](provider_cutover.md#navigation-live-quality-dashboard) records the live navigation quality dashboard. | A single fresh, high-confidence, source-backed `ExactAst`, explicit import, default export, or export-tag compiler candidate can now drive live `textDocument/definition` with legacy fallback. Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes without behavior changes. | Generated/no-source, dynamic-boundary, stale, low-confidence, and ambiguous candidates remain fallback/shadow data, not exact source-location promises. | Add additional generated/dynamic project-shape receipts before broadening navigation migration |
| Find references | `partial-live exact/imported` | [semantic_scorecard.md](semantic_scorecard.md) `reference_edges = available`, `reference_shadow_regressions = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) release-readiness `FindReferences` receipts now trace exact/static, imported, generated, dynamic-boundary, low-confidence fallback, stale, and real-workspace quality occurrences; [Mojolicious navigation quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_30_mojolicious_navigation_quality.rs) records project-shaped references probes; [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) tracks the navigation quality receipt; [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462) tracks runtime quality receipts; [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828) tracks the exact/static live cutover lane; [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836) tracks the imported/exported live cutover lane; [provider_cutover.md](provider_cutover.md#navigation-live-quality-dashboard) records the live navigation quality dashboard. | Fresh, high-confidence, source-backed `ExactAst`, `ImportExportInference`, or `LiteralRequireImport` occurrence references can now drive live `textDocument/references` when declaration inclusion is off, with legacy fallback for anything uncertain. Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes without behavior changes. | Generated/no-source, dynamic-boundary, stale, low-confidence, ambiguous, broader declaration-including, coderef, and typeglob cases remain fallback/shadow data. | Add precision/recall receipts for generated, coderef, typeglob, and broader declaration-including cases |
| Rename | `partial-live lexical + package-local pilot / semantic-shadow` | [semantic_scorecard.md](semantic_scorecard.md) `rename_plan = 100% pass`, `rename_unsafe_edit_count = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) schema-fixture `RenamePlan` receipts trace exact static edits, dynamic-boundary blockers, stale compiler facts, and low-confidence ambiguity; runtime blocker UX tests compare live rename with compiler plans; [Mojolicious rename unsafe-edit receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_35_mojolicious_rename_unsafe_edit.rs) and [Dancer2 rename unsafe-edit receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_37_dancer2_rename_unsafe_edit.rs) record project-shaped rename safety; [#8915](https://github.com/EffortlessMetrics/perl-lsp/pull/8915) proves the scoped lexical live slice; package-local live-pilot receipts prove exact source-backed edit application, partial-plan fallback, RealBaseline and Dancer2 edit-freshness fallback, Catalyst ambiguous-identity refusal, and generated/dynamic/stale/low-confidence blockers. | Same-file sigiled lexical rename can use source-backed current-document AST proof when the request proves exactly one `my` or `state` declaration edit. Narrow package-local rename can use fresh source-backed definition/reference proof only when the materialized semantic edit set exactly matches the workspace source/ambiguity guard. Rename planning remains fixture-backed with fact-source traces proving unsafe compiler facts block rather than authorize edits, and blocker reason / UX notes cover dynamic, generated, stale-fact, ambiguous, and low-confidence blockers. | Broad compiler-backed, package-wide, generated, dynamic, stale, low-confidence, ambiguous, and missing-proof rename remain blocked or fallback/shadow data. The package-local live pilot is not broad workspace rename proof and `perl.previewPackageRename` remains no-edit planned-edit/blocker/fallback UX for broader package/compiler-backed shapes. | Keep project-shaped unsafe-edit and edit-freshness receipts fresh; broader package/compiler-backed rename remains deferred |
| Safe delete | `partial-live source-backed pilot` | [semantic_scorecard.md](semantic_scorecard.md) `safe_delete_plan = 100% pass`, `safe_delete_blocker_fixture_pass_rate = 100% pass`; [semantic_shadow_compare.md](semantic_shadow_compare.md) schema-fixture `SafeDeletePlan` receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, and stale compiler facts; runtime blocker UX receipts cover Dancer2 stale, generated, dynamic-boundary, and low-confidence blockers plus RealBaseline imported-symbol and allowed-symbol paths; non-subroutine/package-wide source-guard receipts prove package variables and package declarations return no edits; RealBaseline `reset` and Dancer2 `to_psgi` receipts prove source-backed delete WorkspaceEdits with inverse rollback proof; Dancer2 `header` and post-`didChange` `to_psgi` receipts prove referenced source-backed methods block with zero returned edits; cross-file `used_target` proves workspace-index references block returned edits; Catalyst `get_action` proves ambiguous workspace identity returns no edits; the generated/dynamic live-command blocker receipt proves `routes` and `plugin_keywords` return zero edits through `perl.safeDeleteSymbol` with persisted explain-provider receipts | Safe-delete blocker planning is fixture-backed and records fact-source traces plus blocker reason / UX notes proving dynamic, stale, low-confidence, generated-member, imported-symbol, referenced-symbol, current-source referenced, workspace-index referenced, non-subroutine, package-wide, and ambiguous-identity cases block deletion or return no edits. `perl.safeDeleteSymbol` may return a client-applied delete WorkspaceEdit only for the narrow source-backed subroutine pilot when compiler allow proof, the exact source guard, current-source/workspace reference guards, workspace identity guard, and rollback proof all pass; the server does not apply the edit. | The live slice is limited to source-backed unreferenced subroutines with rollback proof, zero current-source/workspace-index references, and accepted workspace identity. Generated/dynamic deletion, non-subroutine deletion, package-wide deletion, fallback/no-source deletion, imported/exported symbols, current-source or workspace-index referenced symbols, ambiguous identities, and server-applied edits remain blocked or unsupported. | Keep generated/no-source and dynamic blocker receipts fresh before any broader symbol-delete promotion |
| Document symbols | `partial-live source-backed` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records four schema-fixture `DocumentSymbols` source/freshness receipts: explicit syntax candidate, framework-generated candidate, dynamic-boundary blocker, and stale compiler fact blocker; [Mojolicious document-symbol quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_32_mojolicious_document_symbols_quality.rs) records project-shaped live document-symbol quality. | Fresh, high-confidence, source-backed parser-syntax document symbols can drive live `textDocument/documentSymbol` results with fallback for astless or uncertain documents. Mojolicious scenario 32 records explicit symbols, generated `has` candidate counts, dynamic-boundary-shaped names, valid LSP symbol shapes, and edit freshness. | Generated/no-source, stale, dynamic, low-confidence, and ambiguous candidates stay gated; generated labels are not yet live in this project-shaped receipt; one Mojolicious receipt is not broad project support. | Add generated-label proof and additional project-shape document-symbol receipts before generated, dynamic, or broader symbol cutover |
| Workspace symbols | `partial-live source-backed + generated-label pilot` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records `WorkspaceSymbols` source/freshness receipts plus `workspace_symbol_real_workspace_quality`; [Mojolicious workspace-symbol noise receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_33_mojolicious_workspace_symbol_noise.rs), [Dancer2 workspace-symbol noise receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_39_dancer2_workspace_symbol_noise.rs), [Catalyst workspace-symbol noise receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_41_catalyst_workspace_symbol_noise.rs), and [Modern OO workspace-symbol noise receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_43_modern_oo_workspace_symbol_noise.rs) record project-shaped live-provider noise; runtime generated expansion rank/noise proof records source-backed exact symbols ahead of labeled generated/framework pilot symbols; runtime false-exact/edit-freshness proof records generated-pilot labels, source-anchor semantics, post-edit generated-pilot refresh, and gated dynamic/stale shadow blockers; runtime scoped cutover proof ties the live generated/framework response to source-anchor semantics and gated false-exact/dynamic/stale boundaries; runtime Moo predicate proof adds another labeled generated-member class; runtime generated/no-source proof records an unanchored framework/runtime candidate as blocked; [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378) tracks the real-workspace quality lane. | Workspace-symbol provider cutover now has typed source / provenance / confidence / freshness traces plus four project-shaped live-provider receipts for query latency, repeated-query count stability, useful/noisy hits, generated candidate gating, dynamic-boundary-shaped names, edit freshness, generated/no-source zero-live exact promotion, mixed source-backed/generated rank proof, generated/dynamic false-exact shadow proof, generated-pilot edit-freshness proof, scoped cutover proof, Moo predicate generated-class proof, and generated/no-source blocker proof. A narrow runtime pilot can return source-backed generated/framework members only with explicit generated labels and framework-declaration anchors. | Existing workspace index remains the main live provider source. Generated/framework pilot symbols are virtual and labeled, not exact generated method-body locations. Compiler-fact workspace-symbol candidates remain gated where generated/no-source, dynamic, stale, partial-index, open-document fallback, ambiguous, or fallback/noise; four receipts plus runtime rank/noise, false-exact/edit-freshness, scoped cutover, predicate-class proof, generated/no-source proof, and support review are not broad project support. | Additional generated/no-source project variants and explicit-label rank/noise proof before broader generated-symbol expansion |
| Semantic tokens | `partial-live source-backed token slice + scoped subroutine/method/package/phase-block/field/method-call/self-method-call/lexical-variable declaration/use traces` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records schema-fixture `SemanticTokens` source/freshness receipts, including generated/no-source, dynamic, stale, broader-class false-exact, and fallback boundaries; runtime quality receipts capture live handler token counts, shadow state, no-token-output-change proof, positive in-range monotonic non-overlapping spans, both synthetic source-backed and RealBaseline project-shaped source-backed compiler-fact subroutine-declaration classes matched to existing live `function` token output, live-output parity across synthetic, Catalyst-shaped, and RealBaseline receipts, a scoped subroutine-declaration proof that allows only source-backed `token:function:` identities matching existing live `function` tokens without adding output, a scoped method-declaration proof that allows only source-backed `token:method_declaration:` identities matching existing live `method` tokens and refreshing after `didChange`, a scoped package-declaration proof that allows only source-backed `token:package_declaration:` identities matching existing live `namespace` tokens and refreshing after `didChange`, a scoped phase-block declaration proof that allows only source-backed `token:phase_block_declaration:` identities matching existing live `macro` tokens and refreshing after `didChange`, a scoped field-declaration proof that allows only source-backed `token:field_declaration:` identities matching existing live `variable` tokens and refreshing after `didChange`, and a scoped method-call proof that allows only source-backed `token:method_call:` identities matching existing live `method` tokens and refreshing after `didChange`, and a scoped self-method-call proof that allows only source-backed `token:self_method_call:` identities matching existing live `method` tokens and refreshing after `didChange`; [Mojolicious semantic-token quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_34_mojolicious_semantic_tokens_quality.rs) records project-shaped token/span validity and edit freshness; [Dancer2 semantic-token quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_38_dancer2_semantic_tokens_quality.rs) records second-project token quality; [Catalyst false-exact/freshness receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_42_catalyst_semantic_tokens_false_exact_freshness.rs) records generated/dynamic-looking false-exact token boundaries and edit freshness. | Semantic-token provider cutover now has typed source / provenance / confidence / freshness traces, plus Mojolicious, Dancer2, RealBaseline, Catalyst, subroutine-declaration, method-declaration, package-declaration, phase-block declaration, field-declaration, method-call, self-method-call, lexical-variable declaration/use, and span-invariant receipts, without claiming broad compiler-backed live cutover or adding token output. | Existing parser/token provider remains the broad live source. Generated/no-source, fallback, stale, dynamic, broader `token:method:` classes, and unmatched compiler-backed classes do not emit new token output, and the project-shaped receipts are not broad project support. | Another scoped compiler-token class proof before broader live compiler-token output |

## Dynamic Perl honesty

| Row | Current read | Policy |
|---|---:|---|
| dynamic boundary detected | 4 dynamic-boundary fixture families; 5 dynamic-boundary facts in [semantic_scorecard.md](semantic_scorecard.md) confidence breakdown | Prefer conservative `unavailable` / `ambiguous` over false exactness |
| ambiguous result | 0 release-readiness, 2 schema-fixtures ([semantic_shadow_compare.md](semantic_shadow_compare.md)) | Surface uncertainty; do not pretend exactness |
| unavailable result | 0 release-readiness, 0 schema-fixtures ([semantic_shadow_compare.md](semantic_shadow_compare.md)) | Acceptable when dynamic Perl prevents safe resolution |
| low-confidence result | 1 heuristic fact across the fixture family ([semantic_scorecard.md](semantic_scorecard.md) fact coverage) | May inform ranking, not unsafe edits |
| false-exact result count | `dynamic_false_precision_count = 0` ([parser.md](parser.md) accuracy scorers) | Should be zero |
| unsafe-edit count | `rename_unsafe_edit_count = 0` ([semantic_scorecard.md](semantic_scorecard.md)) | Should be zero |

The dashboard rewards conservative honesty. It does not imply full static
resolution of dynamic Perl.

## Reliable user-facing claims

- Imported symbols can be explained when exact import facts exist.
- Dynamic strict-bareword diagnostics are suppressed only when semantic
  evidence supports suppression; missing or ambiguous evidence keeps the legacy
  diagnostic path.
- Rename and safe-delete are conservative and may block unsafe edits with
  explanations.
- Method completion is improving, but unknown dynamic receivers must not
  invent exact methods.

## Current limits

- No full Perl type inference.
- No runtime symbolic evaluator.
- No full Moose / Moo metamodel.
- No complete CPAN metadata resolver.
- Dynamic Perl remains conservative.
- Parser and semantic metrics are consumed from existing artifacts, not
  recomputed here.

## Next recommended UX improvement

Use the live navigation quality dashboard before broadening navigation beyond
source-backed exact/imported facts.

The next navigation slice should remain observational:

- record legacy, compiler, and live result counts
- keep generated, stale, low-confidence, ambiguous, declaration-including,
  coderef, typeglob, and dynamic-boundary occurrences on fallback or shadow-only
  receipts
- report fallback and blocker counts before any broader generated/dynamic
  navigation migration
