# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current shadow receipt records five fact-source traces across definition,
  references, completion, and hover surfaces.
- This is trace/proof infrastructure only. It does not make a provider consume
  compiler facts in live LSP behavior.

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives; fallback diagnostics remain available | False-positive / false-negative fixture receipts that include fact-source traces |
| Completion | `partial live / shadowed` | Existing completion paths and semantic-shadow fixtures can use visible symbols and generated members | Provider-impact rows for compiler facts, ranking stability, and fact-source traces |
| Hover | `shadowed` | Hover shadow code can query visible symbols and provenance | Promote only after provenance labels, fallback behavior, and trace receipts are fixture-backed |
| Definition / goto | `shadowed` | Definition shadow compare tracks regressions before live migration | Ranked compiler candidates with exact/static/generated/dynamic source labels |
| References | `shadowed` | Reference shadow compare tracks regressions before live migration | Reference precision/recall fixtures from compiler facts |
| Rename | `fixture-backed queries` | Rename plan semantic fixtures exist; broad live compiler cutover remains deferred | Dynamic-boundary blockers and unsafe-edit receipts |
| Safe delete | `fixture-backed queries` | Safe-delete plan semantic fixtures exist; broad live compiler cutover remains deferred | Dynamic-boundary blockers and generated-member blockers |
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
