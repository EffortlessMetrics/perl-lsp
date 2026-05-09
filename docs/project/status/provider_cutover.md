# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current semantic-shadow compare artifact records forty-six deterministic
  receipts across definition, references, completion, hover, diagnostics, workspace-symbol,
  document-symbol, semantic-token, rename, and safe-delete surfaces.
- Definition/reference shadow proof now records imported-symbol,
  framework-generated, dynamic-boundary, and low-confidence fallback candidate
  traces without changing live navigation behavior.
- Completion shadow proof now records compiler visible-symbol candidate deltas,
  generated-member labels, and dynamic-boundary blockers. A narrow live
  completion slice promotes only high-confidence imported/exported
  visible-symbol facts; generated and dynamic-boundary candidates remain
  shadowed or blocked.
- Hover provenance proof now records imported-symbol, framework-generated,
  dynamic-boundary, and fallback paths with typed fact-source traces and source /
  confidence labels.
- Hover now has a narrow live runtime slice for traced compiler fact,
  framework-adapter, and dynamic-boundary hover paths. Legacy hover remains the
  fallback when compiler facts are absent, stale, or only legacy-equivalent.
- Diagnostics now have a narrow live cutover for high-confidence imported and
  generated visible-symbol facts. Ambiguous, low-confidence, and
  dynamic-boundary cases remain fallback or blocked instead of being silently
  suppressed.
- Rename and safe-delete now have boundary-shadowed proof for exact static
  allow decisions plus dynamic-boundary, stale compiler fact, low-confidence,
  and generated-member blockers. These receipts do not broaden live refactor
  behavior.
- Workspace symbols now have source/freshness and real-workspace quality shadow
  proof for fresh compiler facts, framework-generated candidates,
  dynamic-boundary blockers, stale compiler facts, and candidate/noise deltas.
  These receipts do not broaden live workspace-symbol behavior.
- Document symbols now have source/freshness shadow proof for explicit syntax
  facts, framework-generated candidates, dynamic-boundary blockers, and stale
  compiler facts. These receipts do not broaden live document-symbol behavior.
- Semantic tokens now have source/freshness shadow proof for explicit
  parser/HIR classifications, compiler-backed classifications,
  dynamic-boundary blockers, and stale compiler facts. These receipts do not
  broaden live semantic-token behavior.
- Other provider surfaces remain trace/proof infrastructure only until their
  own cutover proof lands.

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives, plus high-confidence imported/generated visible-symbol facts; fallback diagnostics remain available | Broader false-positive / false-negative fixture receipts before any additional diagnostic families move live |
| Completion | `partial live / shadowed` | Existing completion paths remain live; high-confidence imported/exported compiler visible-symbol facts can contribute live candidates with legacy fallback; semantic-shadow fixtures still trace generated labels, rank deltas, and dynamic-boundary blockers without promoting those families | Ranking stability and real-workspace candidate quality before any broader live cutover |
| Hover | `partial live / provenance-backed` | Runtime hover uses compiler-fact cutover for traced compiler fact, framework-adapter, and dynamic-boundary paths when fresh workspace facts are available; legacy hover remains fallback; hover cutover/shadow code labels imported, generated, dynamic-boundary, and fallback paths with fact-source traces and source/confidence text | Real-workspace hover quality receipts before broader generated/dynamic expansion |
| Definition / goto | `ranked-shadowed` | Definition shadow compare tracks imported, generated, dynamic-boundary, and low-confidence fallback candidate traces before live migration | Runtime integration and real-workspace quality receipts before any live navigation cutover |
| References | `ranked-shadowed` | Reference shadow compare tracks imported, generated, dynamic-boundary, and low-confidence fallback occurrence traces before live migration | Reference precision/recall fixtures from real-workspace compiler facts |
| Rename | `boundary-shadowed` | Rename plan receipts trace exact static edits, dynamic-boundary blockers, stale compiler facts, and low-confidence ambiguity before any live compiler-backed refactor behavior | Runtime blocker UX and real-workspace unsafe-edit receipts |
| Safe delete | `boundary-shadowed` | Safe-delete receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, and stale compiler facts before any live compiler-backed refactor behavior | Runtime blocker UX and real-workspace unsafe-delete receipts |
| Workspace symbols | `shadowed` | Existing workspace index remains the live provider source; semantic-shadow fixtures trace fresh compiler, generated, dynamic-boundary, stale fact, and real-workspace quality candidates | Runtime integration and live-provider workspace-symbol quality receipts before any live cutover |
| Document symbols | `shadowed` | Existing document-symbol provider remains the live source; semantic-shadow fixtures trace explicit syntax, generated, dynamic-boundary, and stale fact candidates | Runtime integration and real-workspace document-symbol quality receipts before any live cutover |
| Semantic tokens | `shadowed` | Existing parser/token provider remains the live source; semantic-shadow fixtures trace parser/HIR, compiler-backed, dynamic-boundary, and stale fact candidates | Runtime integration and token/span invariant receipts before any live cutover |

## Cutover Rules

- Do not cut a provider over just because a fact exists.
- Every provider answer that uses compiler facts should be able to identify
  source, provenance, confidence, and fallback state where relevant.
- Generated and dynamic-boundary answers must be labeled honestly.
- Provider regressions should first appear in shadow compare or scorecard
  receipts, not as live editor behavior.

## Tracking

- Provider cutover umbrella: [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197)
- Hover live provenance slice: [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369)
- Completion live visible-symbol slice: [#8374](https://github.com/EffortlessMetrics/perl-lsp/issues/8374)
- Workspace-symbol source/freshness proof: [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353)
- Workspace-symbol real-workspace quality receipt: [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378)
- Document-symbol source/freshness proof: [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359)
- Semantic-token source/freshness proof: [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360)
- Fact-source trace receipt slice: [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305)
- Compiler facts: [compiler_facts.md](compiler_facts.md)
- Semantic scorecard: [semantic_scorecard.md](semantic_scorecard.md)
- Semantic shadow compare: [semantic_shadow_compare.md](semantic_shadow_compare.md)
- Real-workspace baseline tracker: [#7949](https://github.com/EffortlessMetrics/perl-lsp/issues/7949)
