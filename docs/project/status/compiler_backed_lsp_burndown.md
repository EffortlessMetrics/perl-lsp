# Compiler-Backed LSP Burndown

## Claim Boundary

This board starts after the Perl core harness reached green advisory status for
parse/compile receipts, real upstream compile ratchets, selected execute-base,
runtime bucket naming, and provider-promotion gates.

The next phase is not another harness scaffolding lane. It is the control
surface for using those receipts to deepen selected runtime execution, reduce
compiler/runtime buckets, choose one provider fact class at a time, shadow it,
and promote only when the provider gates are satisfied.

This page does not claim broad runtime conformance or broad compiler-backed LSP
cutover. Provider behavior changes still require provider-specific proof.

## Required PR Proof

Normal PRs are governed by:

- `Perl LSP Rust Small Result`
- `ripr+ New Gap Gate`

Coverage is advisory/manual/scheduled only and must not block normal PR work.

## Current Substrate

| Substrate | State | Evidence |
|---|---:|---|
| Real upstream `base`/`comp`/`run` compile ratchets | Green / advisory | [Perl core harness burndown](perl_core_harness_burndown.md), `.ci/perl-core-harness/upstream-{base,comp,run}-compile-baseline.json` |
| Selected execute-base | Green / selected ratchet | `base/if.t`, `base/cond.t`, `base/num.t`, `base/pat.t`, `base/translate.t`, and `base/while.t` execute 6/6 files with 325/325 TAP assertions |
| Runtime bucket model | Green / model | [Perl core harness burndown](perl_core_harness_burndown.md#runtime-model) |
| Provider promotion gates | Green / gated plan | [Perl core harness burndown](perl_core_harness_burndown.md#provider-promotion-gates), [provider cutover](provider_cutover.md), [provider confidence matrix](provider_confidence_matrix.md) |
| Semantic shadow substrate | Available | [semantic shadow compare](semantic_shadow_compare.md) records deterministic provider comparisons |

## Phase 2 Board

| ID | Work item | State | Current evidence | Next action | Stop condition |
|---|---|---:|---|---|---|
| P0 | Phase 2 board | Green after #3459 | This file | None | Board exists and is linked from the harness and provider status pages |
| P1 | First provider candidate selection | Green / selected | `textDocument/references` PIR-A initialized same-file lexical references selected below; #2674 tracks the measurement-to-promotion roadmap; #2635/#3461 supply the guarded shadow/refusal prerequisites | Refresh the selected candidate's P5/P6/P7 evidence before behavior promotion | One provider surface and fact class is selected with harness, scorecard/oracle, shadow, fallback, and rollback evidence named |
| P2 | Selected runtime expansion | Green / expanded | Selected execute-base covers `base/if.t`, `base/cond.t`, `base/num.t`, `base/pat.t`, `base/translate.t`, and `base/while.t` with 325/325 TAP assertions | None | Selected execute receipt expanded beyond the previous three files without broad profile-wide execute |
| P3 | Compile bucket burn-down | Yellow / ongoing | #3474 focused runner proof moves `base/lex.t` from `parse_recovery` to `compile_effect`; #3481 focused proof moves `base/term.t` from `compile_effect` to compile pass; #3483 focused proof moves `base/rs.t` from `compile_effect` to compile pass; #3485 focused proof moves `comp/require.t` from `parse_recovery` to `compile_effect`; #3487 focused proof moves `comp/use.t` from `parse_recovery` to `compile_effect`; #3490 focused proof moves `comp/proto.t` from `parse_recovery` to `compile_effect`; #3492 focused proof moves `comp/decl.t` from `parse_recovery` to compile pass; #3494 focused proof moves `comp/line_debug.t` from compile-mode `parse_recovery` to `compile_effect`; #3496 focused proof moves `base/lex.t` from `compile_effect` to compile pass; #3498 focused proof moves `comp/final_line_num.t` from compile-mode `parse_recovery` to `compile_effect`; #3500 focused proof moves `comp/parser.t` from compile-mode `parse_recovery` to `compile_effect`; #3502 focused proof moves `comp/our.t` from `compile_effect` to compile pass; current real upstream receipts still show `parse_recovery` in `run` and `compile_effect` in `comp`/`run` | Reduce one named bucket or tight cluster | One compile bucket shrinks without new `unknown` or unbucketed failures |
| P4 | Runtime bucket burn-down | Yellow / ongoing | `runtime_control_flow` has one selected-file burn-down through `base/while.t`; `runtime_value_model` has selected-file burn-downs through `base/num.t` and `base/translate.t`; `runtime_regex` has its first selected slice through `base/pat.t`; broader runtime remains selected only | Return to P3 compile bucket burn-down before selecting another execute candidate | One runtime bucket shrinks and the selected execute baseline is updated |
| P5 | Curated-gold / oracle alignment | Green / curated corpus exists | `references_promotion_test::p5_curated_expected_lsp_range_corpus_for_initialized_lexicals` checks expected LSP `Range` sets for the PIR-A initialized same-file lexical slice; existing provider references receipts remain support evidence only | Feed this corpus into P6 provider shadow comparison | Candidate fact class has independent correctness evidence beyond harness receipts |
| P6 | Provider shadow comparison | Green / receipt exists | `references_promotion_test::p6_provider_shadow_receipt_for_curated_references_slice` compares the selected PIR-A candidate to the P5 curated ranges and asserts the PIR shadow receipt records fallback, confidence, freshness, and dynamic-boundary blocker behavior | None | Shadow receipt records candidate, fallback, blockers, confidence, freshness, and dynamic-boundary behavior |
| P7 | First provider promotion prep | Green / prep proof | `references_promotion_test::p7_promotion_prep_preserves_feature_gate_and_no_output_change` proves the rollback/off anchor, legacy-output preservation in shadow mode, and dynamic-boundary fallback for the selected PIR-A references slice; the References PIR-A row is recorded in the provider promotion ledger | None | Promotion-prep proof lands without broad live behavior change |
| P8 | First gated provider promotion | Green / bounded live | `textDocument/references` now allows the selected lexical slice to reach the semantic source-backed tier when `includeDeclaration=false`; `handle_references_lexical_variable_without_declaration_uses_source_backed_tier` proves the live receipt, `handle_references_workspace_variable_answering_tier_in_trace` proves declaration-including lexical fallback, and `p7_promotion_prep_preserves_feature_gate_and_no_output_change` preserves the rollback/off proof | Return to P3/P4 bucket burn-down before any broader provider promotion | One provider surface/fact class changes live behavior with rollback/fallback proof |

## Selected Provider Candidate

P1 selects one candidate only. This is not a live-behavior promotion.

| Field | Selection |
|---|---|
| Provider surface | `textDocument/references` |
| Fact class | PIR-A initialized same-file lexical references |
| Roadmap issue | [#2674](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/2674) |
| Prerequisite context | [#2635](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/2635) guarded shadow/promotion history; [#3461](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3461) dynamic-boundary refusal prerequisite |
| Claim boundary | Prepare promotion evidence only for fresh, source-backed, high-confidence, same-file initialized lexical variable references when declaration inclusion is off and the request has no stale, ambiguous, generated/no-source, coderef, typeglob, declaration-including, or dynamic-boundary shape |
| Why first | References already have a partial live/ranked-shadowed source-backed tier; the selected slice is same-file and lexical; #2674 preserves the corrected measurement-first roadmap without reviving the dark wrapper path |
| Harness evidence | Real upstream `base`/`comp`/`run` parse/compile receipts and ratchets are green advisory substrate for source/compiler fact work; selected execute-base is not required for this static reference slice |
| Provider evidence already present | [provider cutover](provider_cutover.md#cutover-matrix) and [provider confidence matrix](provider_confidence_matrix.md#matrix) record references as partial-live exact/imported with legacy fallback and dynamic/stale blockers; this is support evidence, not PIR-A lexical promotion proof |
| Current live boundary | The P8 cutover is live only for fresh, source-backed, high-confidence same-file lexical variable references when `includeDeclaration=false`; declaration-including and unsupported shapes keep fallback behavior |
| Explicit non-goals | No generated/no-source, coderef, typeglob, declaration-including, stale, ambiguous, dynamic-boundary, module/import, workspace-wide, rename, or runtime-derived references |

## Selected Correctness Evidence Source

P5 supplies the independent correctness evidence source for the references
candidate. It does not claim provider shadow comparison or promotion readiness.

| Field | Selection |
|---|---|
| Evidence source | Curated expected LSP `Range` sets for PIR-A initialized same-file lexical references |
| Execution issue | [#2674](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/2674) |
| Why this source | #2674's corrected roadmap calls for a curated correctness corpus with expected ranges, not equality against legacy references behavior |
| Corpus location | `crates/perl-lsp-rs-core/tests/references_promotion_test.rs` |
| Corpus test | `p5_curated_expected_lsp_range_corpus_for_initialized_lexicals` |
| Fixture shape covered | Same-file lexical references with initialized declarations, reads, writes, same-name shadowing, same-name separate sub bodies, sigil identity, CRLF, Unicode, and UTF-16 range conversion coverage |
| Negative shape covered | Declaration filtering is asserted by the corpus; package-qualified fallback and dynamic-boundary fallback remain covered by adjacent promotion tests |
| Still out of corpus | Bare declarations until separately proven, generated/no-source candidates, coderef/typeglob candidates, stale facts, ambiguous facts, workspace-wide references, and declaration-including promotion requests |
| Shadow relationship | `references_promotion_test::p6_provider_shadow_receipt_for_curated_references_slice` compares the selected PIR-A candidate against these expected ranges, preserves legacy output in shadow mode, and asserts PIR shadow receipt fallback/blocker/freshness/dynamic-boundary behavior |
| Existing support evidence | Provider cutover and confidence matrix references rows, semantic-shadow `FindReferences`, Mojolicious navigation quality receipt, #2635 guarded history, and #3461 dynamic-boundary refusal |
| Not enough by itself | Legacy equality, broad existing references pass/fail, or harness parse/compile receipts alone cannot authorize promotion |

## Current Gap Inputs

The Phase 2 burn-down starts from the latest harness board, not from guesses.

| Profile | Current gaps | Candidate work |
|---|---|---|
| `base` parse/compile | All nine upstream `base/*.t` files compile cleanly; `base/rs.t`, `base/term.t`, and `base/lex.t` are compile-clean but not selected execute | Defer `base/rs.t`, `base/term.t`, and `base/lex.t` execute until the runtime lane intentionally pulls IO/backtick and lexical edge-case behavior |
| `comp` parse/compile | compile-mode `parse_recovery` is cleared; `compile_effect` remains on fifteen files including `comp/parser.t`, `comp/final_line_num.t`, `comp/line_debug.t`, `comp/require.t`, `comp/use.t`, and `comp/proto.t` | Reduce `comp/parser.t`, `comp/line_debug.t`, `comp/require.t`, or `comp/hints.t` compile effects |
| `run` parse/compile | `parse_recovery` on run-script and switch tests; `compile_effect` on switch and fresh-perl tests | Start with one switch cluster such as `run/switchM.t` or `run/switch-I-and-M.t` |
| selected execute-base | `base/if.t`, `base/cond.t`, `base/num.t`, `base/pat.t`, `base/translate.t`, and `base/while.t` pass selected execute | Remaining `base/*.t` candidates are now runtime-entry decisions; return to P3/P4 before widening selected execute |

## Provider Promotion Gates

No provider behavior changes unless all gates are satisfied for exactly one
provider surface and one fact class:

| Gate | Required evidence | Source |
|---|---|---|
| Harness substrate | Relevant parse/compile or selected execute receipt is ratcheted or explicitly bucketed with no `unknown` / unbucketed failures | [Perl core harness burndown](perl_core_harness_burndown.md), `.ci/perl-core-harness/*baseline.json` |
| Semantic correctness | Curated-gold, semantic scorecard, or oracle evidence covers the fact class | [semantic scorecard](semantic_scorecard.md), [semantic shadow compare](semantic_shadow_compare.md), oracle receipts where applicable |
| Provider shadow | Shadow comparison records candidate, fallback, blocker, confidence, freshness, and dynamic-boundary behavior | [semantic shadow compare](semantic_shadow_compare.md), provider-specific receipt tests |
| Live safety | The live provider has an explicit fallback, blocker, feature gate, rollback strategy, or no-output-change invariant | [provider cutover](provider_cutover.md), [provider confidence matrix](provider_confidence_matrix.md) |
| Real workspace | At least one project-shaped receipt proves useful behavior or preserves fallback for risky generated/dynamic/stale shapes | UX scenario receipts and real-workspace baselines linked from provider status docs |

## Initial Candidate Guidance

Start provider work with low-risk, already-shadowed surfaces. The candidate PR
must pick one exact surface/fact class and cite its receipts before any behavior
change.

Prefer first:

- Diagnostics explanation or suppression classes with existing dynamic-boundary
  and false-positive proof.
- Semantic-token classes that preserve existing output and prove span parity.
- Source-backed definition/reference slices only when declaration, freshness,
  fallback, and precision/recall boundaries are explicit.

Avoid first:

- Module/import definition or hover that depends on unresolved
  `compile_effect` / module-resolution gaps.
- Broad completion, rename, workspace-symbol, package/stash, generated/no-source,
  dynamic, or runtime-derived facts.

## Burndown Order

1. Publish this Phase 2 board and link it from the harness and provider status pages. Active issue: #3459.
2. Select the first provider/fact-class candidate with explicit gates and no behavior change. Done for `textDocument/references` PIR-A initialized same-file lexical references.
3. Build the P5 curated expected LSP range corpus for the selected references slice. Done in `references_promotion_test::p5_curated_expected_lsp_range_corpus_for_initialized_lexicals`.
4. Add or refresh provider shadow comparison for the selected references slice. Done in `references_promotion_test::p6_provider_shadow_receipt_for_curated_references_slice`.
5. Land promotion-prep proof with rollback, fallback, or no-output-change invariant. Done in `references_promotion_test::p7_promotion_prep_preserves_feature_gate_and_no_output_change`.
6. Expand selected execute-base by one parse/compile-clean `base/*.t` file. Done for `base/num.t`, `base/pat.t`, and `base/translate.t`.
7. Reduce one compile bucket or tight cluster and update the harness board/receipts.
8. Promote one provider surface/fact class only after the gates are green. Done for the bounded `textDocument/references` lexical slice.
9. Return to this board and choose the next red.

## PR Rules

- One issue, one PR, one merge, then return to this board.
- Every PR must name the board row it changes.
- Every PR must cite the receipt or proof that demonstrates the change.
- Do not mix provider promotion with runtime expansion.
- Do not mix parser/compiler bucket burn-down with provider behavior.
- Do not turn the advisory Perl core harness workflow into a required PR gate.
