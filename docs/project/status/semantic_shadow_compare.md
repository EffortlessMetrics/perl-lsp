# Semantic Shadow Compare

Measured: `deterministic-fixture-baseline`

Receipts: `16`

## Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 2 |
| improved | 6 |
| regression | 1 |
| same | 7 |
| unavailable | 0 |

## Release-Readiness Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 0 |
| improved | 1 |
| regression | 0 |
| same | 1 |
| unavailable | 0 |

## Schema Fixture Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 2 |
| improved | 5 |
| regression | 1 |
| same | 6 |
| unavailable | 0 |

## Receipts

| Scope | Query | Symbol | Verdict | Old count | New count |
|---|---|---|---|---:|---:|
| release-readiness | FindDefinition | `Foo::bar` | same | 1 | 1 |
| release-readiness | FindReferences | `Foo::bar` | improved | 1 | 2 |
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

## Fact Source Traces

| Scope | Query | Surface | Source | Provenance | Confidence | Freshness | State |
|---|---|---|---|---|---|---|---|
| release-readiness | FindDefinition | Definition | CompilerFact | SemanticAnalyzer | High | Fresh | Shadow |
| release-readiness | FindReferences | References | SemanticFact | SemanticAnalyzer | High | Fresh | Shadow |
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

0.13.2 semantic shadow proof: release-readiness counts include provider-gating receipts only; schema fixture receipts exercise non-gating verdict shapes.
