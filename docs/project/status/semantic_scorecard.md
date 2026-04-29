# Semantic Scorecard

Measured: `deterministic-fixture-scorecard-v1`  
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

## Semantic Facts Coverage

| Row | Status | Total | Exact | High confidence | Heuristic | Dynamic boundary | Fixture-family coverage |
|---|---|---:|---:|---:|---:|---:|---:|
| declaration_facts | available_zero | 0 | 0 | 0 | 0 | 0 | 12 |
| definition_candidates | available_zero | 0 | 0 | 0 | 0 | 0 | 12 |
| export_facts | available_zero | 0 | 0 | 0 | 0 | 0 | 12 |
| import_specs | unavailable | 0 | 0 | 0 | 0 | 0 | 0 |
| occurrence_facts | available_zero | 0 | 0 | 0 | 0 | 0 | 12 |
| package_graph | unavailable | 0 | 0 | 0 | 0 | 0 | 0 |
| reference_edges | available_zero | 0 | 0 | 0 | 0 | 0 | 12 |
| rename_plan | unavailable | 0 | 0 | 0 | 0 | 0 | 0 |
| safe_delete_plan | unavailable | 0 | 0 | 0 | 0 | 0 | 0 |

Wave 2 scorecard rows are deterministic and adapter-safe: available rows emit zero counts until fact adapters are wired; future rows are explicitly unavailable.
