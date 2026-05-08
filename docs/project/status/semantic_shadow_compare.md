# Semantic Shadow Compare

Measured: `deterministic-fixture-baseline`

Receipts: `5`

## Verdict Counts

| Verdict | Count |
|---|---:|
| ambiguous | 1 |
| improved | 1 |
| regression | 1 |
| same | 1 |
| unavailable | 1 |

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
| ambiguous | 1 |
| improved | 0 |
| regression | 1 |
| same | 0 |
| unavailable | 1 |

## Receipts

| Scope | Query | Symbol | Verdict | Old count | New count |
|---|---|---|---|---:|---:|
| release-readiness | FindDefinition | `Foo::bar` | same | 1 | 1 |
| release-readiness | FindReferences | `Foo::bar` | improved | 1 | 2 |
| schema-fixture | CountUsages | `Foo::bar` | regression | 4 | 3 |
| schema-fixture | VisibleSymbols | `Foo::bar` | ambiguous | 2 | 2 |
| schema-fixture | Hover | `Foo::bar` | unavailable | 0 | 1 |

## Fact Source Traces

| Scope | Query | Surface | Source | Provenance | Confidence | Freshness | State |
|---|---|---|---|---|---|---|---|
| release-readiness | FindDefinition | Definition | CompilerFact | SemanticAnalyzer | High | Fresh | Shadow |
| release-readiness | FindReferences | References | SemanticFact | SemanticAnalyzer | High | Fresh | Shadow |
| schema-fixture | CountUsages | References | SemanticFact | SemanticAnalyzer | Medium | Fresh | Shadow |
| schema-fixture | VisibleSymbols | Completion | CompilerFact | ImportExportInference | Medium | Fresh | Shadow |
| schema-fixture | Hover | Hover | Fallback | SearchFallback | Low | NotApplicable | Unavailable |

0.13.2 semantic shadow proof: release-readiness counts include provider-gating receipts only; schema fixture receipts exercise non-gating verdict shapes.
