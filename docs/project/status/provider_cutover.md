# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current shadow receipt records thirty-two fact-source traces across definition,
  references, completion, hover, diagnostics, rename, and safe-delete surfaces.
- Definition/reference shadow proof now records imported-symbol,
  framework-generated, dynamic-boundary, and low-confidence fallback candidate
  traces without changing live navigation behavior.
- Completion shadow proof now records compiler visible-symbol candidate deltas,
  generated-member labels, and dynamic-boundary blockers without changing live
  completion behavior.
- Hover provenance proof now records imported-symbol, framework-generated,
  dynamic-boundary, and fallback paths with typed fact-source traces and source /
  confidence labels.
- Diagnostics now have a narrow live cutover for high-confidence imported and
  generated visible-symbol facts. Ambiguous, low-confidence, and
  dynamic-boundary cases remain fallback or blocked instead of being silently
  suppressed.
- Rename and safe-delete now have boundary-shadowed proof for exact static
  allow decisions plus dynamic-boundary, stale compiler fact, low-confidence,
  and generated-member blockers. These receipts do not broaden live refactor
  behavior.
- Other provider surfaces remain trace/proof infrastructure only until their
  own cutover proof lands.

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives, plus high-confidence imported/generated visible-symbol facts; fallback diagnostics remain available | Broader false-positive / false-negative fixture receipts before any additional diagnostic families move live |
| Completion | `partial live / shadowed` | Existing completion paths remain live; semantic-shadow fixtures trace compiler visible-symbol candidates, generated labels, rank deltas, and dynamic-boundary blockers | Ranking stability and real-workspace candidate quality before any broader live cutover |
| Hover | `provenance-backed` | Hover cutover/shadow code labels imported, generated, dynamic-boundary, and fallback paths with fact-source traces and source/confidence text; broad live behavior remains gated | Real-workspace hover quality and runtime integration receipts before broader live cutover |
| Definition / goto | `ranked-shadowed` | Definition shadow compare tracks imported, generated, dynamic-boundary, and low-confidence fallback candidate traces before live migration | Runtime integration and real-workspace quality receipts before any live navigation cutover |
| References | `ranked-shadowed` | Reference shadow compare tracks imported, generated, dynamic-boundary, and low-confidence fallback occurrence traces before live migration | Reference precision/recall fixtures from real-workspace compiler facts |
| Rename | `boundary-shadowed` | Rename plan receipts trace exact static edits, dynamic-boundary blockers, stale compiler facts, and low-confidence ambiguity before any live compiler-backed refactor behavior | Runtime blocker UX and real-workspace unsafe-edit receipts |
| Safe delete | `boundary-shadowed` | Safe-delete receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, and stale compiler facts before any live compiler-backed refactor behavior | Runtime blocker UX and real-workspace unsafe-delete receipts |
| Workspace symbols | `legacy workspace index` | Existing workspace index remains provider source | Compiler fact merge and source/freshness trace |
| Semantic tokens | `syntax/legacy` | Parser/token facts remain source | Compiler facts only after token/span invariants are proven |

## Cutover Rules

- Do not cut a provider over just because a fact exists.
- Every provider answer that uses compiler facts should be able to identify
  source, provenance, confidence, and fallback state where relevant.
- Generated and dynamic-boundary answers must be labeled honestly.
- Provider regressions should first appear in shadow compare or scorecard
  receipts, not as live editor behavior.

## Tracking

- Provider cutover umbrella: [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197)
- Fact-source trace receipt slice: [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305)
- Compiler facts: [compiler_facts.md](compiler_facts.md)
- Semantic scorecard: [semantic_scorecard.md](semantic_scorecard.md)
- Semantic shadow compare: [semantic_shadow_compare.md](semantic_shadow_compare.md)
- Real-workspace baseline tracker: [#7949](https://github.com/EffortlessMetrics/perl-lsp/issues/7949)
