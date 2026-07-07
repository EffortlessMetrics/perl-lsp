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
| Selected execute-base | Green / selected ratchet | `base/if.t`, `base/cond.t`, and `base/while.t` execute 3/3 files with 10/10 TAP assertions |
| Runtime bucket model | Green / model | [Perl core harness burndown](perl_core_harness_burndown.md#runtime-model) |
| Provider promotion gates | Green / gated plan | [Perl core harness burndown](perl_core_harness_burndown.md#provider-promotion-gates), [provider cutover](provider_cutover.md), [provider confidence matrix](provider_confidence_matrix.md) |
| Semantic shadow substrate | Available | [semantic shadow compare](semantic_shadow_compare.md) records deterministic provider comparisons |

## Phase 2 Board

| ID | Work item | State | Current evidence | Next action | Stop condition |
|---|---|---:|---|---|---|
| P0 | Phase 2 board | Green after #3459 | This file | None | Board exists and is linked from the harness and provider status pages |
| P1 | First provider candidate selection | Green / selected | `textDocument/references` PIR-A initialized same-file lexical references selected below; #2674 tracks the measurement-to-promotion roadmap; #2635/#3461 supply the guarded shadow/refusal prerequisites | Refresh the selected candidate's P5/P6/P7 evidence before behavior promotion | One provider surface and fact class is selected with harness, scorecard/oracle, shadow, fallback, and rollback evidence named |
| P2 | Selected runtime expansion | Red | Selected execute-base covers `base/if.t`, `base/cond.t`, and `base/while.t` | Verify current parse/compile receipts, then add one clean `base/*.t` candidate such as `base/num.t` if still clean | Selected execute receipt expands beyond the current three files or records a deliberate deferral |
| P3 | Compile bucket burn-down | Yellow / ongoing | Current real upstream receipts still show `parse_recovery` and `compile_effect` buckets in `base`, `comp`, and `run` | Reduce one named bucket or tight cluster | One compile bucket shrinks without new `unknown` or unbucketed failures |
| P4 | Runtime bucket burn-down | Yellow / ongoing | `runtime_control_flow` has one selected-file burn-down through `base/while.t`; broader runtime remains selected only | Add runtime behavior only when a selected execute receipt demands it | One runtime bucket shrinks and the selected execute baseline is updated |
| P5 | Curated-gold / oracle alignment | Red | Provider gates require semantic scorecard, curated-gold, or oracle proof before cutover | Pick the evidence source required by P1's chosen fact class | Candidate fact class has independent correctness evidence beyond harness receipts |
| P6 | Provider shadow comparison | Red | Shadow substrate exists, but no Phase 2 candidate has a board row here | Add or refresh shadow comparison for the selected provider/fact class | Shadow receipt records candidate, fallback, blockers, confidence, freshness, and dynamic-boundary behavior |
| P7 | First provider promotion prep | Red | Provider gates exist; no Phase 2 promotion-prep PR is recorded here | Prepare rollback/feature-gate/no-output-change proof for the selected candidate | Promotion-prep proof lands without broad live behavior change |
| P8 | First gated provider promotion | Red / future | No Phase 2 provider promotion is selected yet | Promote only after P1, P5, P6, and P7 are green for one fact class | One provider surface/fact class changes live behavior with rollback/fallback proof |

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
| Still required before promotion | P5 semantic scorecard/curated/oracle evidence for this exact slice; P6 shadow receipt with candidate/fallback/blocker/freshness/dynamic-boundary behavior; P7 rollback or feature-gate proof |
| Explicit non-goals | No generated/no-source, coderef, typeglob, declaration-including, stale, ambiguous, dynamic-boundary, module/import, workspace-wide, rename, or runtime-derived references |

## Current Gap Inputs

The Phase 2 burn-down starts from the latest harness board, not from guesses.

| Profile | Current gaps | Candidate work |
|---|---|---|
| `base` parse/compile | `base/lex.t` remains `parse_recovery`; `base/rs.t` and `base/term.t` remain `compile_effect` | Reduce `base/lex.t`, classify or model `base/rs.t` / `base/term.t` |
| `comp` parse/compile | `parse_recovery` on `comp/decl.t`, `comp/final_line_num.t`, `comp/line_debug.t`, `comp/parser.t`, `comp/proto.t`, `comp/require.t`, `comp/use.t`; `compile_effect` on ten files | Start with `comp/require.t` or `comp/use.t` parser gaps, then `comp/our.t` / `comp/hints.t` compile effects |
| `run` parse/compile | `parse_recovery` on run-script and switch tests; `compile_effect` on switch and fresh-perl tests | Start with one switch cluster such as `run/switchM.t` or `run/switch-I-and-M.t` |
| selected execute-base | `base/if.t`, `base/cond.t`, and `base/while.t` pass selected execute | Consider `base/num.t` after confirming parse/compile cleanliness |

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
3. Refresh P5 semantic scorecard/curated/oracle evidence for the selected references slice.
4. Add or refresh provider shadow comparison for the selected references slice.
5. Land promotion-prep proof with rollback, fallback, or no-output-change invariant.
6. Expand selected execute-base by one parse/compile-clean `base/*.t` file, or record a deliberate deferral.
7. Reduce one compile bucket or tight cluster and update the harness board/receipts.
8. Promote one provider surface/fact class only after the gates are green.
9. Return to this board and choose the next red.

## PR Rules

- One issue, one PR, one merge, then return to this board.
- Every PR must name the board row it changes.
- Every PR must cite the receipt or proof that demonstrates the change.
- Do not mix provider promotion with runtime expansion.
- Do not mix parser/compiler bucket burn-down with provider behavior.
- Do not turn the advisory Perl core harness workflow into a required PR gate.
