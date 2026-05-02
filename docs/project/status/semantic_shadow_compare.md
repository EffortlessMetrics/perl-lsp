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

## Receipts

| Query | Symbol | Verdict | Old count | New count |
|---|---|---|---:|---:|
| FindDefinition | `Foo::bar` | same | 1 | 1 |
| FindReferences | `Foo::bar` | improved | 1 | 2 |
| CountUsages | `Foo::bar` | regression | 4 | 3 |
| VisibleSymbols | `Foo::bar` | ambiguous | 2 | 2 |
| Hover | `Foo::bar` | unavailable | 0 | 1 |

Deterministic RC2 shadow-compare receipt shape for semantic provider cutover proof.
