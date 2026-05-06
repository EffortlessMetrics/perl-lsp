# Semantic Scorecard

Measured: `deterministic-fixture-baseline`  
Fixture family version: `1`  
Fixtures loaded: `16`

## Fact Coverage

| Row | Status | Facts | Coverage | Exact | High | Heuristic | Dynamic boundary |
|---|---|---:|---:|---:|---:|---:|---:|
| declaration_facts | available | 42 | 16/16 | 154 | 152 | 1 | 5 |
| definition_candidates | available | 42 | 16/16 | 154 | 152 | 1 | 5 |
| export_facts | available | 3 | 16/16 | 154 | 152 | 1 | 5 |
| import_specs | available | 11 | 16/16 | 154 | 152 | 1 | 5 |
| inheritance_edges | available | 1 | 16/16 | 154 | 152 | 1 | 5 |
| occurrence_facts | available | 26 | 16/16 | 154 | 152 | 1 | 5 |
| package_graph_edges | available | 2 | 16/16 | 154 | 152 | 1 | 5 |
| reference_edges | available | 1 | 16/16 | 154 | 152 | 1 | 5 |
| role_composition_edges | available | 1 | 16/16 | 154 | 152 | 1 | 5 |

## Readiness Rows

| Row | Status | Value | Threshold | Evidence |
|---|---|---:|---:|---|
| completion_import_fixture_pass_rate | pass | 100% | 100% | import/export visibility fixtures |
| definition_shadow_regressions | pass | 0 | 0 | semantic shadow compare release-readiness receipts |
| method_candidates_fixture_pass_rate | pass | 100% | 100% | method candidate query fixtures |
| package_graph | pass | 2 | > 0 | package graph fixture edges |
| reference_shadow_regressions | pass | 0 | 0 | semantic shadow compare release-readiness receipts |
| rename_plan | pass | 100% | 100% | rename plan query fixtures |
| rename_unsafe_edit_count | pass | 0 | 0 | rename plan query fixtures |
| safe_delete_blocker_fixture_pass_rate | pass | 100% | 100% | safe-delete plan query fixtures |
| safe_delete_plan | pass | 100% | 100% | safe-delete plan query fixtures |
| semantic_fact_counts_nonzero | pass | 85 | > 0 | semantic fixture indexing |
| undefined_symbol_false_positive_fixture_rate | pass | 0% | 0% | diagnostics fixture receipts |
| visible_symbols_fixture_pass_rate | pass | 100% | 100% | workspace scorecard fixtures |

## Unavailable Rows

| Row | Status | Reason |
|---|---|---|

## Fixture IDs

- `autoload_dynamic_boundary`
- `dynamic_import_via_variable`
- `dynamic_require_boundary`
- `empty_import_suppression`
- `eval_string_dynamic_boundary`
- `export_tag_expansion`
- `generated_accessor`
- `imported_function_visibility`
- `inherited_method`
- `normal_static_missing_symbol`
- `qualified_vs_bare_references`
- `role_method`
- `same_bare_sub_two_packages`
- `static_class_dynamic_import`
- `symbolic_deref_assign`
- `typeglob_alias`

0.13.2 semantic proof rail: scorecard rows are deterministic and fixture-backed; semantic expansion remains conservative for unavailable rows.
