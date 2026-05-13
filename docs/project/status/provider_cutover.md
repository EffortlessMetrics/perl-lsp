# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current semantic-shadow compare artifact records forty-eight deterministic
  receipts across definition, references, completion, hover, diagnostics, workspace-symbol,
  document-symbol, semantic-token, rename, and safe-delete surfaces.
- Definition/reference shadow proof now records imported-symbol,
  framework-generated, dynamic-boundary, low-confidence fallback, stale fact, and
  real-workspace quality candidate/occurrence traces.
- Definition now has a narrow live exact-syntax runtime slice: a single fresh,
  high-confidence, source-backed `ExactAst` candidate can drive
  `textDocument/definition`; imported/exported, generated/no-source, dynamic,
  low-confidence, and ambiguous candidates retain legacy fallback.
- References remain shadowed. Definition/reference runtime quality receipts
  exercise the live `textDocument/definition` and `textDocument/references`
  handlers against compiler-fact receipts in the same runtime workspace.
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
  and generated-member blockers. Runtime blocker UX receipts compare live
  rename / symbol safe-delete request paths with compiler plans for exact
  static, dynamic-boundary, generated-member, stale-fact, and low-confidence
  cases. These receipts record user-facing blocker reasons without broadening
  live refactor behavior.
- Workspace symbols now have source/freshness and real-workspace quality shadow
  proof for fresh compiler facts, framework-generated candidates,
  dynamic-boundary blockers, stale compiler facts, and candidate/noise deltas.
  These receipts do not broaden live workspace-symbol behavior.
- Document symbols now have source/freshness shadow proof for explicit syntax
  facts, framework-generated candidates, dynamic-boundary blockers, and stale
  compiler facts. These receipts do not broaden live document-symbol behavior.
- Document symbols and workspace symbols now have runtime quality receipts that
  call the live `textDocument/documentSymbol` and `workspace/symbol` handlers
  and capture live provider counts and results without changing live behavior.
  Seven BDD receipt tests cover document symbols (provider field,
  no-live-behavior-change, count integrity, symbol presence, shadow state, notes
  proof, unknown-URI graceful handling) and eight tests cover workspace symbols
  (provider field, no-live-behavior-change, count integrity, query echo, shadow
  state, notes proof, empty-query, no-match query). These receipts complete the
  runtime integration proof step for both surfaces.
- Semantic tokens now have source/freshness shadow proof for explicit
  parser/HIR classifications, compiler-backed classifications,
  dynamic-boundary blockers, and stale compiler facts. These receipts do not
  broaden live semantic-token behavior.
- Semantic tokens now have runtime integration quality receipts
  (`semantic_tokens_runtime_quality_receipt`) that exercise the live
  `textDocument/semanticTokens/full` handler and capture token count, shadow
  state, and a quality-proof note. Nine BDD tests confirm receipt correctness,
  no-live-behavior-change invariant, and token-count parity with the live
  handler. Compiler-fact candidates remain pending staged cutover.
- Other provider surfaces remain trace/proof infrastructure only until their
  own cutover proof lands.

## State Definitions

Provider states are acceptance tiers, not release labels. A provider can move to
the next tier only when the evidence for that tier is committed or generated by
the relevant receipt command.

| State | Meaning | Exit gate |
| --- | --- | --- |
| `unavailable` | No provider-specific compiler-fact proof exists yet, or the surface has no owner issue. | Add an owner issue plus fixture-backed fact evidence. |
| `fixture-backed` | The fact layer has deterministic fixtures, but provider behavior is still legacy or unproven. | Add provider-specific shadow receipts with source, provenance, confidence, freshness, fallback, and dynamic-boundary state. |
| `shadowed` | Legacy and compiler-fact outcomes are compared in receipts without changing live runtime behavior. `ranked-shadowed` and `boundary-shadowed` are shadow subtypes for candidate ranking and refactor-blocker proof. | Show zero unacceptable fixture regressions, explicit stale/dynamic handling, and a scoped live-cutover plan. |
| `provenance-backed` | Runtime or receipt output can explain the fact source, provenance, confidence, and freshness for the scoped answer. | Prove fallback behavior and real-workspace quality before broadening beyond the scoped path. |
| `partial live` | One or more high-confidence fact families are live with legacy fallback and fail-closed stale/dynamic handling. | Add real-workspace quality receipts, rollback/fallback proof, and provider-specific noise or precision deltas before expanding. |
| `live` | Compiler facts are the default provider source for the scoped surface while traces, fallback, and dynamic-boundary blockers remain available. | Keep receipts fresh when fact families or provider behavior change. |
| `blocked` | Proof found a safety, freshness, noise, or precision issue that prevents live behavior. | Close the blocker with a targeted fix and rerun the provider proof lane. |

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives, plus high-confidence imported/generated visible-symbol facts; fallback diagnostics remain available | Broader false-positive / false-negative fixture receipts before any additional diagnostic families move live |
| Completion | `partial live / shadowed` | Existing completion paths remain live; high-confidence imported/exported compiler visible-symbol facts can contribute live candidates with legacy fallback; semantic-shadow fixtures still trace generated labels, rank deltas, and dynamic-boundary blockers without promoting those families | Ranking stability and real-workspace candidate quality before any broader live cutover |
| Hover | `partial live / provenance-backed` | Runtime hover uses compiler-fact cutover for traced compiler fact, framework-adapter, and dynamic-boundary paths when fresh workspace facts are available; legacy hover remains fallback; hover cutover/shadow code labels imported, generated, dynamic-boundary, and fallback paths with fact-source traces and source/confidence text | Real-workspace hover quality receipts before broader generated/dynamic expansion |
| Definition / goto | `partial live / ranked-shadowed` | A single fresh, high-confidence, source-backed `ExactAst` candidate can drive live `textDocument/definition` with legacy fallback. Imported/exported, generated/no-source, dynamic-boundary, low-confidence, ambiguous, stale, and broader real-workspace candidates remain traced as fallback/shadow proof. | Imported/exported high-confidence definition slice before any broader navigation migration |
| References | `ranked-shadowed` | Reference shadow compare tracks imported, generated, dynamic-boundary, low-confidence fallback, stale fact, and real-workspace quality occurrence traces before live migration. Runtime quality receipts compare the live `textDocument/references` result with the compiler cutover receipt without changing live behavior. | Narrow live cutover proof for exact/imported high-confidence occurrences before any broader navigation migration |
| Rename | `boundary-shadowed` | Rename plan receipts trace exact static edits, dynamic-boundary blockers, stale compiler facts, low-confidence ambiguity, runtime blocker UX notes, and live-vs-compiler exact-static receipt data before any live compiler-backed refactor behavior | Real-workspace unsafe-edit receipts and a narrow lexical/package rename live-cutover proof before any broader refactor migration |
| Safe delete | `boundary-shadowed` | Safe-delete receipts trace exact static allow decisions, dynamic-boundary blockers, framework-generated blockers, stale compiler facts, runtime blocker UX notes, and live-vs-compiler exact-static receipt data before any live compiler-backed refactor behavior | Real-workspace unsafe-delete receipts and explicit blocker UX in live safe-delete surfaces before any broader refactor migration |
| Workspace symbols | `shadowed` | Existing workspace index remains the live provider source; semantic-shadow fixtures trace fresh compiler, generated, dynamic-boundary, stale fact, and real-workspace quality candidates; runtime quality receipts capture live provider counts/results without changing live behavior | Real-workspace workspace-symbol quality receipts ([#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378)) before any live cutover |
| Document symbols | `shadowed` | Existing document-symbol provider remains the live source; semantic-shadow fixtures trace explicit syntax, generated, dynamic-boundary, and stale fact candidates; runtime quality receipts capture live provider counts/results without changing live behavior | Real-workspace document-symbol quality receipts before any live cutover |
| Semantic tokens | `shadowed` | Existing parser/token provider remains the live source; semantic-shadow fixtures trace parser/HIR, compiler-backed, dynamic-boundary, and stale fact candidates; runtime quality receipts capture live token count and shadow state without changing live behavior | Token/span invariant receipts and real-workspace token quality before any live cutover |

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
- Definition/reference real-workspace quality receipts: [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382)
- Definition/reference runtime integration and live-provider quality receipts: [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462)
- Definition exact/imported live cutover lane: [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803)
- Rename/safe-delete runtime blocker UX receipts: [#8464](https://github.com/EffortlessMetrics/perl-lsp/issues/8464)
- Workspace-symbol source/freshness proof: [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353)
- Workspace-symbol real-workspace quality receipt: [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378)
- Document-symbol source/freshness proof: [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359)
- Semantic-token source/freshness proof: [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360)
- Fact-source trace receipt slice: [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305)
- Compiler facts: [compiler_facts.md](compiler_facts.md)
- Semantic scorecard: [semantic_scorecard.md](semantic_scorecard.md)
- Semantic shadow compare: [semantic_shadow_compare.md](semantic_shadow_compare.md)
- Real-workspace baseline tracker: [#7949](https://github.com/EffortlessMetrics/perl-lsp/issues/7949)
