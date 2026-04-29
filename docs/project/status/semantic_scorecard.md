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

Initial harness: metrics intentionally baseline_pending until semantic facts land.

## Wave 2 adapter/index migration status

| Area | Status | Notes |
|---|---|---|
| Exact facts emitted | In progress | Canonical fact model is fixed (`AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`, `DiagnosticFact`); adapters are being wired incrementally. |
| Fact shard write-through | Active (write-through only) | Workspace ingestion is substrate-only and intentionally not provider-facing yet. |
| Definition candidate multimap | Staged | Compatibility-layer path exists for staged adoption; not yet cut over as sole definition source. |
| Typed reference index | Staged | Typed-edge/index groundwork is staged behind compatibility behavior. |
| Shadow-compare receipts | Planning scaffold | Receipt shape/planning exists; full behavior parity receipts are pending. |
| Scorecard v1 | Scaffold/baseline pending | Fixture bank is live; metric rows stay `baseline_pending` until adapters/indexes are fully producing stable comparable outputs. |

### Explicit non-goals for this wave

- No provider cutover yet.
- No rename/safe-delete cutover yet.
- No full type inference.

### Wave 3 pointer

Next substantive cutover work is intentionally centered on `ImportSpec` extraction + `visible_symbols_at` implementation.
