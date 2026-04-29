# Semantic Scorecard

Measured: `deterministic-fixture-baseline`  
Fixture family version: `1`  
Fixtures loaded: `12`

## Fixture IDs

- `autoload_dynamic_boundary`
- `dynamic_require_boundary`
- `empty_import_suppression`
- `eval_string_dynamic_boundary`
- `export_tag_expansion`
- `generated_accessor`
- `imported_function_visibility`
- `inherited_method`
- `qualified_vs_bare_references`
- `role_method`
- `same_bare_sub_two_packages`
- `typeglob_alias`

## Metrics

| Metric | Status | Value |
|---|---|---:|
| completion_top1 | baseline_pending | n/a |
| completion_top5 | baseline_pending | n/a |
| definition_hit_at_1 | baseline_pending | n/a |
| definition_hit_at_5 | baseline_pending | n/a |
| query_latency_p50 | baseline_pending | n/a |
| query_latency_p95 | baseline_pending | n/a |
| reference_precision | baseline_pending | n/a |
| reference_recall | baseline_pending | n/a |
| rename_unsafe_edit_count | baseline_pending | n/a |
| safe_delete_external_ref_detection | baseline_pending | n/a |
| undefined_symbol_false_positive_rate | baseline_pending | n/a |

Initial harness: metrics intentionally baseline_pending while Wave 2 stays in adapter/index + shadow-compare mode.

Wave 2 status snapshot:

- Exact fact adapters are landed (`SymbolDecl -> EntityFact`, `SymbolRef -> OccurrenceFact`, `ExportInfo -> ExportSet`).
- Fact shards are write-through in workspace storage.
- Definition-candidate multimap and typed reference index are compatibility-only.
- Provider cutover, rename/safe-delete cutover, and full type inference remain explicit non-goals.
- Wave 3 migration target is `ImportSpec` + `visible_symbols_at` provider wiring.
