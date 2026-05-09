# Semantic Shadow Compare

Measured: `deterministic-fixture-baseline`

Receipts: `36`

## Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 2 |
| improved | 7 |
| regression | 1 |
| same | 26 |
| unavailable | 0 |

## Release-Readiness Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 0 |
| improved | 1 |
| regression | 0 |
| same | 9 |
| unavailable | 0 |

## Schema Fixture Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 2 |
| improved | 6 |
| regression | 1 |
| same | 17 |
| unavailable | 0 |

## Receipts

| Scope | Query | Symbol | Verdict | Old count | New count |
|---|---|---|---|---:|---:|
| release-readiness | FindDefinition | `Foo::bar` | same | 1 | 1 |
| release-readiness | FindReferences | `Foo::bar` | improved | 1 | 2 |
| release-readiness | FindDefinition | `imported_func` | same | 1 | 1 |
| release-readiness | FindDefinition | `generated_accessor` | same | 1 | 1 |
| release-readiness | FindDefinition | `dynamic_symbol` | same | 0 | 0 |
| release-readiness | FindDefinition | `low_confidence_candidate` | same | 1 | 1 |
| release-readiness | FindReferences | `imported_func` | same | 1 | 1 |
| release-readiness | FindReferences | `generated_accessor` | same | 1 | 1 |
| release-readiness | FindReferences | `dynamic_symbol` | same | 0 | 0 |
| release-readiness | FindReferences | `low_confidence_candidate` | same | 1 | 1 |
| schema-fixture | CountUsages | `Foo::bar` | regression | 4 | 3 |
| schema-fixture | VisibleSymbols | `Foo::bar` | ambiguous | 2 | 2 |
| schema-fixture | CompletionVisibility | `completion_import_candidates` | improved | 1 | 2 |
| schema-fixture | CompletionVisibility | `completion_generated_candidates` | improved | 0 | 1 |
| schema-fixture | CompletionVisibility | `completion_dynamic_boundary` | same | 0 | 0 |
| schema-fixture | Hover | `hover_imported_symbol` | same | 1 | 1 |
| schema-fixture | Hover | `hover_generated_member` | same | 1 | 1 |
| schema-fixture | Hover | `hover_dynamic_boundary` | same | 0 | 0 |
| schema-fixture | Hover | `hover_fallback` | same | 1 | 1 |
| schema-fixture | DiagnosticsCheck | `imported_func` | improved | 0 | 1 |
| schema-fixture | DiagnosticsCheck | `generated_accessor` | improved | 0 | 1 |
| schema-fixture | DiagnosticsCheck | `genuinely_missing` | same | 1 | 1 |
| schema-fixture | DiagnosticsCheck | `ambiguous_import` | ambiguous | 1 | 1 |
| schema-fixture | DiagnosticsCheck | `symbolic_ref_boundary` | improved | 0 | 1 |
| schema-fixture | WorkspaceSymbols | `workspace_symbol_imported` | same | 1 | 1 |
| schema-fixture | WorkspaceSymbols | `workspace_symbol_generated` | improved | 0 | 1 |
| schema-fixture | WorkspaceSymbols | `workspace_symbol_dynamic_boundary` | same | 0 | 0 |
| schema-fixture | WorkspaceSymbols | `workspace_symbol_stale_fact` | same | 0 | 0 |
| schema-fixture | RenamePlan | `rename_exact_static` | same | 1 | 1 |
| schema-fixture | RenamePlan | `rename_dynamic_boundary` | same | 1 | 1 |
| schema-fixture | RenamePlan | `rename_stale_compiler_fact` | same | 1 | 1 |
| schema-fixture | RenamePlan | `rename_low_confidence` | same | 1 | 1 |
| schema-fixture | SafeDeletePlan | `safe_delete_exact_static` | same | 1 | 1 |
| schema-fixture | SafeDeletePlan | `safe_delete_dynamic_boundary` | same | 1 | 1 |
| schema-fixture | SafeDeletePlan | `safe_delete_generated_member` | same | 1 | 1 |
| schema-fixture | SafeDeletePlan | `safe_delete_stale_compiler_fact` | same | 1 | 1 |

## Fact Source Traces

| Scope | Query | Surface | Source | Provenance | Confidence | Freshness | State |
|---|---|---|---|---|---|---|---|
| release-readiness | FindDefinition | Definition | CompilerFact | SemanticAnalyzer | High | Fresh | Shadow |
| release-readiness | FindReferences | References | SemanticFact | SemanticAnalyzer | High | Fresh | Shadow |
| release-readiness | FindDefinition | Definition | CompilerFact | ImportExportInference | High | Fresh | Shadow |
| release-readiness | FindDefinition | Definition | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |
| release-readiness | FindDefinition | Definition | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| release-readiness | FindDefinition | Definition | Fallback | NameHeuristic | Low | Fresh | Fallback |
| release-readiness | FindReferences | References | CompilerFact | ImportExportInference | High | Fresh | Shadow |
| release-readiness | FindReferences | References | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |
| release-readiness | FindReferences | References | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| release-readiness | FindReferences | References | Fallback | NameHeuristic | Low | Fresh | Fallback |
| schema-fixture | CountUsages | References | SemanticFact | SemanticAnalyzer | Medium | Fresh | Shadow |
| schema-fixture | VisibleSymbols | Completion | CompilerFact | ImportExportInference | Medium | Fresh | Shadow |
| schema-fixture | CompletionVisibility | Completion | CompilerFact | ImportExportInference | High | Fresh | Shadow |
| schema-fixture | CompletionVisibility | Completion | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |
| schema-fixture | CompletionVisibility | Completion | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | Hover | Hover | CompilerFact | ImportExportInference | High | Fresh | Primary |
| schema-fixture | Hover | Hover | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Primary |
| schema-fixture | Hover | Hover | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | Hover | Hover | Fallback | SearchFallback | Low | NotApplicable | Fallback |
| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | ImportExportInference | High | Fresh | Primary |
| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | FrameworkSynthesis | High | Fresh | Primary |
| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | SemanticAnalyzer | High | Fresh | Primary |
| schema-fixture | DiagnosticsCheck | Diagnostics | CompilerFact | ImportExportInference | Low | Fresh | Fallback |
| schema-fixture | DiagnosticsCheck | Diagnostics | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | CompilerFact | ImportExportInference | High | Fresh | Shadow |
| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | FrameworkAdapter | FrameworkSynthesis | Medium | Fresh | Shadow |
| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | WorkspaceSymbols | WorkspaceSymbols | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |
| schema-fixture | RenamePlan | Rename | SemanticFact | ExactAst | High | Fresh | Shadow |
| schema-fixture | RenamePlan | Rename | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | RenamePlan | Rename | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |
| schema-fixture | RenamePlan | Rename | SemanticFact | NameHeuristic | Low | Fresh | Blocked |
| schema-fixture | SafeDeletePlan | SafeDelete | SemanticFact | ExactAst | High | Fresh | Shadow |
| schema-fixture | SafeDeletePlan | SafeDelete | DynamicBoundary | DynamicBoundary | High | Fresh | Blocked |
| schema-fixture | SafeDeletePlan | SafeDelete | FrameworkAdapter | FrameworkSynthesis | High | Fresh | Blocked |
| schema-fixture | SafeDeletePlan | SafeDelete | CompilerFact | SemanticAnalyzer | Low | Stale | Blocked |

0.13.2 semantic shadow proof: release-readiness counts include provider-gating receipts only; schema fixture receipts exercise non-gating verdict shapes.
