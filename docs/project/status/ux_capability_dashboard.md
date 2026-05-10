# Semantic UX Capability Dashboard

This dashboard maps parser accuracy and semantic proof rails to editor-facing
Perl language intelligence.

It answers one question:

> Does the editor feel Perl-aware?

This dashboard is **descriptive**, not generative: parser and semantic metrics
are consumed from existing artifacts (linked below). Updates here record how
those metrics translate into user-facing capability claims.

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
| Completion | `semantic-live-with-fallback` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) `completion_import_pass_rate = 100%`; [semantic_scorecard.md](semantic_scorecard.md) `completion_import_fixture_pass_rate = pass`; [semantic_shadow_compare.md](semantic_shadow_compare.md) `completion_live_visible_import_candidates`; [#8374](https://github.com/EffortlessMetrics/perl-lsp/issues/8374) runtime visible-symbol fixtures | Import/export visibility passes the deterministic fixtures, including empty-import suppression and export-tag expansion. Runtime completion can now surface high-confidence imported/exported compiler visible-symbol candidates with source/provenance/confidence/freshness labels and legacy fallback. | Real-workspace coverage is one small CPAN-style family (4 files, 2 baseline tests). Generated, dynamic-boundary, method, and workspace-wide completion candidates remain shadowed or separately gated. | Prove ranking stability and real-workspace candidate quality before broader completion cutover |
| Method completion | `semantic-shadow` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) `method_completion_shadow_or_cutover_status` (guarded cutover; 0 regressions, 0 unavailable receipts); [#7901](https://github.com/EffortlessMetrics/perl-lsp/pull/7901) literal `bless` receiver inference; [#7917](https://github.com/EffortlessMetrics/perl-lsp/pull/7917) typed receiver-evidence provenance; [#7920](https://github.com/EffortlessMetrics/perl-lsp/pull/7920) receiver-evidence detail text; [#7926](https://github.com/EffortlessMetrics/perl-lsp/pull/7926) medium-confidence receiver detail labels; [#7930](https://github.com/EffortlessMetrics/perl-lsp/pull/7930) bounded low-confidence unknown-receiver fallback | Exact receiver evidence (static package calls `Foo->method`, `$self->` / `$this->`, `Package->new` constructor assignments, type-engine inferred receivers, literal `bless ..., "Package"` forms) still drives package-scoped method completions, with semantic candidates surfacing own / inherited / generated context when they fully cover the legacy method set. Method-completion `detail` text explains the receiver-evidence source — e.g. `receiver: static package`, `receiver: self/this`, `receiver: constructor assignment`. Medium-confidence evidence (literal `bless`, type-engine inference) carries an explicit `medium confidence` label — e.g. `receiver: literal bless, medium confidence`. True Unknown receivers (sub parameters, undeclared variables, harmless `bless` identifiers/comments/hash-keys) now receive bounded low-confidence fallback candidates from used modules plus the current package graph; fallback `detail` text says `receiver: unknown, low confidence` and the candidates sort below all exact-receiver tiers. | High-confidence evidence remains unlabelled by design (the common case stays clean). Dynamic receivers (`bless {}, $class`, expression-tail forms, nested calls, non-builtin `bless`-prefixed identifiers) remain fail-closed and do not receive fallback. There is no all-workspace fallback — fallback is bounded to used modules plus the current package graph. There is no fallback for Dynamic / non-literal `bless` forms. Exact receiver completions remain higher-ranked than fallback candidates. #7930 changes candidate inclusion only for true Unknown receivers; exact-receiver behavior, sort, detail, and the #7920 / #7926 detail contracts are unchanged. | Validate unknown-receiver fallback quality on real-workspace baselines |
| Hover | `semantic-live-with-fallback` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records four schema-fixture `Hover` provenance receipts: imported-symbol, framework-generated, dynamic-boundary, and fallback paths; [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369) adds the runtime imported-symbol hover fixture. | Hover provenance is fixture-backed with typed source / provenance / confidence / freshness traces and source labels. Runtime hover now uses fresh compiler-fact cutover output for traced imported/generated/dynamic paths and preserves legacy hover as fallback. | Broader real-workspace hover quality is not yet separately proven. Runtime proof currently covers the imported visible-symbol path. | Add real-workspace hover quality receipts before broader generated/dynamic expansion |
| Diagnostics | `semantic-live-with-fallback` | [semantic_capability_dashboard.md](semantic_capability_dashboard.md#live-semantic-diagnostics) `dynamic_diagnostics_live`; [semantic_scorecard.md](semantic_scorecard.md) `undefined_symbol_false_positive_fixture_rate = 0%`; [dynamic_diagnostics_suppression_tests.rs](../../../crates/perl-lsp-rs/tests/dynamic_diagnostics_suppression_tests.rs) | Push and pull diagnostics suppress false `PL109 UnquotedBareword` results for indexed `eval "sub NAME"` and `Foo->import(@names)` evidence. Legacy diagnostics remain the fallback when semantic data is unavailable. | Suppression is evidence-gated and order-aware. Missing semantic index, unknown ordering, unrelated names, non-literal `eval $code`, cross-file `AUTOLOAD`, symbolic dereference, and truly dynamic sources fail closed unless indexed evidence proves the specific bareword may be visible at the diagnostic point. | Add #7948 order-aware fixtures and #7949 real-workspace semantic baseline |
| Goto definition | `semantic-shadow` | [semantic_scorecard.md](semantic_scorecard.md) `definition_candidates = available` (16/16 fixtures), `definition_shadow_regressions = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) release-readiness `FindDefinition` receipts now trace exact/static, imported, generated, dynamic-boundary, low-confidence fallback, stale, and real-workspace quality candidates; [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) tracks the navigation quality receipt. | Definition candidates are available across the deterministic fixtures with zero release-readiness regressions, and ranked candidate sources are labeled before live navigation cutover. The quality receipt records legacy/compiler candidate counts, rank/noise deltas, generated labels, and stale/dynamic blockers without claiming live behavior. | Live provider runtime integration is not separately proven. Generated and dynamic-boundary candidates remain labeled proof data, not exact source-location promises. | Add runtime integration and live-provider quality receipts before live navigation cutover ([#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462)) |
| Find references | `semantic-shadow` | [semantic_scorecard.md](semantic_scorecard.md) `reference_edges = available`, `reference_shadow_regressions = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) release-readiness `FindReferences` receipts now trace exact/static, imported, generated, dynamic-boundary, low-confidence fallback, stale, and real-workspace quality occurrences; [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382) tracks the navigation quality receipt. | Reference edges are fixture-backed, shadow-compare reports improvements against the legacy path, and occurrence sources are labeled before live navigation cutover. The quality receipt records legacy/compiler occurrence counts, rank/noise deltas, generated labels, and stale/dynamic blockers without claiming live behavior. | Deeper callsite / coderef / typeglob coverage is not yet proven. Live provider runtime integration remains gated. | Add runtime integration and live-provider reference precision/recall receipts ([#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462)) |
| Rename | `semantic-shadow` | [semantic_scorecard.md](semantic_scorecard.md) `rename_plan = 100% pass`, `rename_unsafe_edit_count = 0`; [semantic_shadow_compare.md](semantic_shadow_compare.md) schema-fixture `RenamePlan` receipts trace exact static edits, dynamic-boundary blockers, stale compiler facts, and low-confidence ambiguity | Rename planning is fixture-backed, currently produces no unsafe edits, and now records fact-source traces proving unsafe compiler facts block rather than authorize edits. | Blocker explanations in user-visible LSP responses and real-workspace unsafe-edit receipts are not separately documented. | Add runtime blocker UX and real-workspace unsafe-edit receipts ([#8464](https://github.com/EffortlessMetrics/perl-lsp/issues/8464)) |
| Safe delete | `semantic-shadow` | [semantic_scorecard.md](semantic_scorecard.md) `safe_delete_plan = 100% pass`, `safe_delete_blocker_fixture_pass_rate = 100% pass`; [semantic_shadow_compare.md](semantic_shadow_compare.md) schema-fixture `SafeDeletePlan` receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, and stale compiler facts | Safe-delete blocker planning is fixture-backed and now records fact-source traces proving dynamic, stale, and generated-member facts block deletion. | Blocker explanations in user-visible LSP responses and real-workspace unsafe-delete receipts are not separately documented. | Add runtime blocker UX and real-workspace unsafe-delete receipts ([#8464](https://github.com/EffortlessMetrics/perl-lsp/issues/8464)) |
| Document symbols | `semantic-shadow` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records four schema-fixture `DocumentSymbols` source/freshness receipts: explicit syntax candidate, framework-generated candidate, dynamic-boundary blocker, and stale compiler fact blocker. | Document-symbol provider cutover now has typed source / provenance / confidence / freshness traces without claiming live provider behavior. | Existing document-symbol provider remains the live source. Runtime integration and real-workspace quality are not yet separately proven. | Add runtime integration and real-workspace document-symbol quality receipts before live cutover |
| Workspace symbols | `semantic-shadow` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records `WorkspaceSymbols` source/freshness receipts plus `workspace_symbol_real_workspace_quality`; [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378) tracks the real-workspace quality receipt. | Workspace-symbol provider cutover now has typed source / provenance / confidence / freshness traces and a shadow quality receipt for legacy/compiler candidate counts, rank/noise deltas, generated labels, and stale/dynamic blockers without claiming live provider behavior. | Existing workspace index remains the live provider source. Runtime integration and live-provider quality are not yet separately proven. | Add runtime integration and live-provider workspace-symbol quality receipts before live cutover |
| Semantic tokens | `semantic-shadow` | [semantic_shadow_compare.md](semantic_shadow_compare.md) records four schema-fixture `SemanticTokens` source/freshness receipts: explicit parser/HIR classification, compiler-backed classification, dynamic-boundary blocker, and stale compiler fact blocker. | Semantic-token provider cutover now has typed source / provenance / confidence / freshness traces without claiming live provider behavior. | Existing parser/token provider remains the live source. Runtime integration and token/span invariants are not yet separately proven. | Add runtime integration and token/span invariant receipts before live cutover |

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

Validate low-confidence unknown-receiver fallback on real-workspace baselines.

#7930 added bounded fallback for true Unknown method receivers. The next step is to measure whether the fallback is useful without becoming noisy before broadening candidate sources:

- true Unknown receiver — may receive bounded fallback from used modules plus current package graph
- Dynamic receiver — remains fail-closed and does not receive fallback
- fallback candidates — always low confidence and sort below exact receiver evidence
- all-workspace fallback — still out of scope until real-workspace noise is measured
- next proof target — real-workspace fixtures that count useful fallback hits, unrelated-method leakage, and exact-receiver non-regression
