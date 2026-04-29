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

## Fixture family coverage

| Family | Fixture count |
|---|---:|
| AUTOLOAD dynamic boundary | 1 |
| dynamic require boundary | 1 |
| empty import suppression | 1 |
| eval STRING dynamic boundary | 1 |
| export tag expansion | 1 |
| generated accessor | 1 |
| imported function visibility | 1 |
| inherited method | 1 |
| qualified vs bare references | 1 |
| role method | 1 |
| same bare sub in two packages | 1 |
| typeglob alias | 1 |

## Rows

| Row | Status | Available | Total | Exact | High confidence | Heuristic | Dynamic boundary |
|---|---|---|---:|---:|---:|---:|---:|
| declaration_facts | adapter_pending | yes | 0 | 0 | 0 | 0 | 0 |
| definition_candidates | adapter_pending | yes | 0 | 0 | 0 | 0 | 0 |
| export_facts | adapter_pending | yes | 0 | 0 | 0 | 0 | 0 |
| import_specs | unavailable | no | 0 | 0 | 0 | 0 | 0 |
| occurrence_facts | adapter_pending | yes | 0 | 0 | 0 | 0 | 0 |
| package_graph | unavailable | no | 0 | 0 | 0 | 0 | 0 |
| reference_edges | adapter_pending | yes | 0 | 0 | 0 | 0 | 0 |
| rename_plan | unavailable | no | 0 | 0 | 0 | 0 | 0 |
| safe_delete_plan | unavailable | no | 0 | 0 | 0 | 0 | 0 |

Semantic fact adapters are optional in v1: supported rows emit deterministic zero coverage until adapters land; future-plan rows are explicitly marked unavailable.
