# Semantic Scorecard

Measured: `deterministic-fixture-baseline`  
Fixture family version: `1`  
Fixtures loaded: `12`

## Fact Coverage

| Row | Status | Facts | Coverage | Exact | High | Heuristic | Dynamic boundary |
|---|---|---:|---:|---:|---:|---:|---:|
| declaration_facts | available | 31 | 12/12 | 111 | 111 | 1 | 0 |
| definition_candidates | available | 31 | 12/12 | 111 | 111 | 1 | 0 |
| export_facts | available | 3 | 12/12 | 111 | 111 | 1 | 0 |
| import_specs | available | 7 | 12/12 | 111 | 111 | 1 | 0 |
| inheritance_edges | available | 1 | 12/12 | 111 | 111 | 1 | 0 |
| occurrence_facts | available | 14 | 12/12 | 111 | 111 | 1 | 0 |
| package_graph_edges | available | 2 | 12/12 | 111 | 111 | 1 | 0 |
| reference_edges | available | 1 | 12/12 | 111 | 111 | 1 | 0 |
| role_composition_edges | available | 1 | 12/12 | 111 | 111 | 1 | 0 |

## Readiness Rows

| Row | Status | Value | Threshold | Evidence |
|---|---|---:|---:|---|
| completion_import_fixture_pass_rate | pass | 100% | 100% | import/export visibility fixtures |
| definition_shadow_regressions | pass | 0 | 0 | semantic shadow compare release-readiness receipts |
| package_graph | pass | 2 | > 0 | package graph fixture edges |
| reference_shadow_regressions | pass | 0 | 0 | semantic shadow compare release-readiness receipts |
| rename_unsafe_edit_count | pass | 0 | 0 | rename blocker fixtures |
| safe_delete_blocker_fixture_pass_rate | pass | 100% | 100% | safe-delete blocker fixtures |
| semantic_fact_counts_nonzero | pass | 58 | > 0 | semantic fixture indexing |
| undefined_symbol_false_positive_fixture_rate | pass | 0% | 0% | diagnostics fixture receipts |
| visible_symbols_fixture_pass_rate | pass | 100% | 100% | workspace scorecard fixtures |

## Unavailable Rows

| Row | Status | Reason |
|---|---|---|
| rename_plan | unavailable | planned for future scorecard waves |
| safe_delete_plan | unavailable | planned for future scorecard waves |

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

0.13.2 semantic proof rail: scorecard rows are deterministic and fixture-backed; semantic expansion remains conservative for unavailable rows.
