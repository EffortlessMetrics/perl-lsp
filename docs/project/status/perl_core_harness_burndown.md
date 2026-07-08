# Perl Core Harness Burndown

## Current Claim Boundary

The Perl core harness is a compiler-testing integration lane. It can prepare a
pinned upstream Perl tree on Linux, discover upstream Perl core tests, run
parse-mode and compile-mode synthetic TAP receipts, and produce advisory
real-tree `base`, `comp`, and `run` smoke and gap-map receipts. Execute-base is
scaffolded as an explicit selected-subset runtime receipt for allowlisted
`base/*.t` files.

It does not claim broad runtime conformance. Profile-wide execute remains
fail-closed until the runtime gap map proves it can be widened safely.

The next phase is tracked in the
[compiler-backed LSP burndown](compiler_backed_lsp_burndown.md), which starts
after this board's harness rows are green and governs selected runtime
expansion, compiler/runtime bucket burn-down, provider candidate selection,
shadow comparison, and gated provider promotion.

## Required PR Proof

Normal PRs are governed by:

- `Perl LSP Rust Small Result`
- `ripr+ New Gap Gate`

Coverage is advisory/manual/scheduled only and must not block normal PR work.

## Board

| ID | Work item | State | Last evidence | Active next issue / PR | Stop condition |
|---|---|---:|---|---|---|
| H0 | Discovery scaffold | Green | #3260 | None | Landed |
| H1 | Parse-mode runner | Green | #3262, #3266, #3267 | None | Landed |
| H2 | Compile-mode receipts | Green | #3273 | None | Landed |
| H3 | Generated base compile ratchet | Green | #3302 | None | Landed |
| H4 | Real upstream `base` smoke wiring | Green / advisory | #3316, #3323, #3379, #3384, [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088) | None | Runner-backed `base` parse/compile receipts recorded |
| H5 | CI policy and PR Smoke hygiene | Green | #3292, #3325, #3327 | None | Required PR proof is RIPR+ plus Rust Small |
| H6 | Shared harness receipt types | Green | #3375 | None | Landed |
| H7 | First advisory real upstream `base` receipt | Green | [run 28703494602](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28703494602), #3379 | None | Ref, counts, buckets, and artifact link recorded |
| H8 | Linux-only upstream prepare | Yellow / future | #3316 | Keep explicit until non-Linux prepare exists | Board names platform boundary |
| H9 | Real upstream `base` runner invocation | Green | #3384, [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088) | None | Real upstream `base` parse/compile records come from `perl-core-test-runner` |
| H10 | `comp` compile smoke | Yellow / bucketed compiler gaps | #3387, #3394, [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840) | None | `comp` smoke writes runner-backed discovery/parse/compile/smoke/gap-map receipts |
| H11 | Harness orchestration crate | Green | #3420 | None | `crates/perl-core-harness` owns orchestration; `xtask` is CLI glue |
| H12 | `run` compile smoke | Yellow / bucketed compiler gaps | #3422, #3424, [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803) | None | `run` smoke writes runner-backed discovery/parse/compile/smoke/gap-map receipts |
| H13 | Real upstream compile ratchets | Green / advisory | #3426 | None | `base`/`comp`/`run` compile receipts are ratcheted separately from the generated fixture ratchet |
| H14 | First bucket burn-down | Green / advisory | #3428, #3429, #3474, #3481, #3483, #3485, #3487, #3490, #3492, #3494, #3496, #3498, #3500, #3502, #3506, #3508, #3510, #3512, #3514, #3516, #3518, #3520, #3522, #3524, #3526, #3528, #3530, #3532, #3534, #3536, #3538, #3540, #3543, #3545, [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077) | None | `base` `parse_recovery` reduced to 0 in the accepted compile ratchet; all nine upstream `base/*.t` files now compile cleanly; `comp/decl.t`, `comp/filter_exception.t`, `comp/fold.t`, `comp/line_debug.t`, `comp/multiline.t`, `comp/our.t`, `comp/parser_run.t`, `comp/redef.t`, `comp/retainedlines.t`, and `comp/utf.t` now compile cleanly; `comp/require.t`, `comp/use.t`, `comp/proto.t`, `comp/final_line_num.t`, and `comp/parser.t` moved from compile-mode `parse_recovery` to `compile_effect`; `run/switch-I-and-M.t`, `run/switchM.t`, `run/switch0.t`, `run/switchF2.t`, `run/switcht.t`, `run/switcha.t`, `run/switchF.t`, `run/noswitch.t`, `run/switchn.t`, `run/switchp.t`, `run/switchx.t`, `run/switchI.t`, `run/switchd-78586.t`, `run/cloexec.t`, `run/runenv_hashseed.t`, `run/switchDx.t`, and `run/fresh_perl.t` now compile cleanly |
| H15 | Execute-one | Green / advisory | #3432 | None | `base/if.t` executes real TAP through an explicit one-file run selector |
| H16 | Execute-base | Green / selected ratchet | #3446, #3448, #3450, #3454, #3479 | None | Selected `base/if.t`, `base/cond.t`, `base/num.t`, `base/pat.t`, `base/translate.t`, and `base/while.t` execute receipt is ratcheted at 6/6 files and 325/325 TAP assertions |
| H17 | Runtime model | Green / model | #3452, #3454, #3479 | None | Runtime buckets are named; `runtime_value_model` is represented by `base/num.t` and `base/translate.t`, `runtime_control_flow` by `base/while.t`, and the first selected `runtime_regex` slice by `base/pat.t` |
| H18 | Compiler-backed LSP provider promotion | Green / gated plan | #3457 | Select one provider fact class only after matching evidence is present | Promotion gates are named and tied to harness receipts, provider confidence docs, shadow/oracle proof, and rollback strategy |

## Latest Receipt Slots

| Receipt | Latest status | Link / artifact |
|---|---:|---|
| `target/perl-core/prepare/<ref>/prepare.json` | Pass for `b62845c7186b0b6a8e4e83419e6b5ef64ceef3ed` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), artifact `perl-core-harness-db5f879540e2d31d39e975ddb1228d12fa5cb838` |
| `target/perl-core/smoke/base/discovery.json` | 9 files discovered | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), `base/cond.t`, `base/if.t`, `base/lex.t`, `base/num.t`, `base/pat.t`, `base/rs.t`, `base/term.t`, `base/translate.t`, `base/while.t` |
| `target/perl-core/smoke/base/parse.json` | 9/9 passed | Focused compile-runner proof in #3474 moves `base/lex.t` past parse recovery; next advisory workflow refresh should publish the full smoke receipt |
| `target/perl-core/smoke/base/compile.json` | 9/9 passed, no buckets | Focused compile-runner proof in #3496; accepted by `.ci/perl-core-harness/upstream-base-compile-baseline.json`; all upstream `base/*.t` files now compile cleanly |
| `target/perl-core/smoke/base/gap-map.json` | 18/18 mode-file entries passed; no buckets | Focused runner records confirm `base/term.t`, `base/rs.t`, and `base/lex.t` moved from `compile_effect` to compile pass; next advisory workflow refresh should publish the full gap-map receipt |
| `target/perl-core/smoke/base/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077) |
| `target/perl-core/smoke/comp/discovery.json` | 25 files discovered | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840); examples include `comp/require.t`, `comp/use.t`, `comp/parser.t`, `comp/proto.t`, `comp/utf.t` |
| `target/perl-core/smoke/comp/parse.json` | 18/25 passed, 7 `parse_recovery` | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840); failures: `comp/decl.t`, `comp/final_line_num.t`, `comp/line_debug.t`, `comp/parser.t`, `comp/proto.t`, `comp/require.t`, `comp/use.t` |
| `target/perl-core/smoke/comp/compile.json` | 18/25 passed, 0 `parse_recovery`, 7 `compile_effect` | Focused runner proofs for #3485, #3487, #3490, #3492, #3494, #3498, #3500, and #3502 move `comp/decl.t` and `comp/our.t` to compile pass and `comp/require.t`, `comp/use.t`, `comp/proto.t`, `comp/final_line_num.t`, and `comp/parser.t` to `compile_effect`; focused runner proofs for #3530, #3532, #3534, #3536, #3538, #3540, #3543, and #3545 move `comp/line_debug.t`, `comp/filter_exception.t`, `comp/redef.t`, `comp/multiline.t`, `comp/fold.t`, `comp/utf.t`, `comp/parser_run.t`, and `comp/retainedlines.t` to compile pass; compile-effect failures include `comp/final_line_num.t`, `comp/form_scope.t`, `comp/hints.t`, `comp/parser.t`, `comp/proto.t`, `comp/require.t`, `comp/use.t` |
| `target/perl-core/smoke/comp/gap-map.json` | 36/50 mode-file entries passed; buckets: 7 `parse_recovery`, 7 `compile_effect` | Focused runner proofs for #3485, #3487, #3490, #3492, #3494, #3498, #3500, #3502, #3530, #3532, #3534, #3536, #3538, #3540, #3543, and #3545 update the accepted compile ratchet; parse-mode receipt from [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840) still contributes 7 `parse_recovery` entries |
| `target/perl-core/smoke/comp/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840) |
| `target/perl-core/smoke/run/discovery.json` | 28 files discovered | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803), artifact `perl-core-harness-597feab33627b5b0469434a0eb84b605aaa4fd52`; examples include `run/fresh_perl.t`, `run/script.t`, `run/switch-I-and-M.t`, `run/switchM.t`, `run/switches.t` |
| `target/perl-core/smoke/run/parse.json` | 18/28 passed, 10 `parse_recovery` | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803); failures: `run/dtrace.t`, `run/exit.t`, `run/locale.t`, `run/runenv.t`, `run/runenv_randseed.t`, `run/script.t`, `run/switchC.t`, `run/switchd.t`, `run/switches.t`, `run/todo.t` |
| `target/perl-core/smoke/run/compile.json` | 18/28 passed, 10 `parse_recovery`, no `compile_effect` | Focused runner proofs for #3506 move `run/switch-I-and-M.t` and `run/switchM.t` to compile pass; focused runner proofs for #3508 move `run/switch0.t`, `run/switchF2.t`, and `run/switcht.t` to compile pass; focused runner proofs for #3510 move `run/switcha.t`, `run/switchF.t`, and `run/noswitch.t` to compile pass; focused runner proof for #3512 moves `run/switchn.t` to compile pass; focused runner proof for #3514 moves `run/switchp.t` to compile pass; focused runner proof for #3516 moves `run/switchx.t` to compile pass; focused runner proof for #3518 moves `run/switchI.t` to compile pass; focused runner proof for #3520 moves `run/switchd-78586.t` to compile pass; focused runner proof for #3522 moves `run/cloexec.t` to compile pass; focused runner proof for #3524 moves `run/runenv_hashseed.t` to compile pass; focused runner proof for #3526 moves `run/switchDx.t` to compile pass; focused runner proof for #3528 moves `run/fresh_perl.t` to compile pass; prior advisory workflow [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803) remains the latest full smoke artifact |
| `target/perl-core/smoke/run/gap-map.json` | 36/56 mode-file entries passed; buckets: 20 `parse_recovery` | Focused runner proofs for #3506, #3508, #3510, #3512, #3514, #3516, #3518, #3520, #3522, #3524, #3526, and #3528 update the accepted compile ratchet; next advisory workflow refresh should publish the full gap-map receipt |
| `target/perl-core/smoke/run/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803) |
| `.ci/perl-core-harness/upstream-base-compile-baseline.json` | Ratchets 9/9 compile pass state; buckets: none | Accepted from #3496 focused compile-runner proof; separate from generated fixture baseline |
| `.ci/perl-core-harness/upstream-comp-compile-baseline.json` | Ratchets 18/25 compile pass state; buckets: 7 `compile_effect` | Updated by focused #3485, #3487, #3490, #3492, #3494, #3498, #3500, and #3502 proofs for `comp/decl.t`, `comp/final_line_num.t`, `comp/parser.t`, `comp/require.t`, `comp/use.t`, `comp/proto.t`, and `comp/our.t`, plus focused #3530, #3532, #3534, #3536, #3538, #3540, #3543, and #3545 proofs for `comp/line_debug.t`, `comp/filter_exception.t`, `comp/redef.t`, `comp/multiline.t`, `comp/fold.t`, `comp/utf.t`, `comp/parser_run.t`, and `comp/retainedlines.t`; separate from generated fixture baseline |
| `.ci/perl-core-harness/upstream-run-compile-baseline.json` | Ratchets 18/28 compile pass state; buckets: 10 `parse_recovery` | Updated by focused #3506 proofs for `run/switch-I-and-M.t` and `run/switchM.t`, focused #3508 proofs for `run/switch0.t`, `run/switchF2.t`, and `run/switcht.t`, focused #3510 proofs for `run/switcha.t`, `run/switchF.t`, and `run/noswitch.t`, focused #3512 proof for `run/switchn.t`, focused #3514 proof for `run/switchp.t`, focused #3516 proof for `run/switchx.t`, focused #3518 proof for `run/switchI.t`, focused #3520 proof for `run/switchd-78586.t`, focused #3522 proof for `run/cloexec.t`, focused #3524 proof for `run/runenv_hashseed.t`, focused #3526 proof for `run/switchDx.t`, plus focused #3528 proof for `run/fresh_perl.t`; separate from generated fixture baseline |
| `target/perl-core/reports/base-execute.json` | 6/6 selected files passed, 325/325 TAP assertions, no runtime buckets | Local receipt generated from pinned upstream Perl artifact [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077) and expanded by P2/P4 selected-execute proof; selected files: `base/cond.t` 4/4, `base/if.t` 2/2, `base/num.t` 56/56, `base/pat.t` 2/2, `base/translate.t` 257/257, and `base/while.t` 4/4 |
| `.ci/perl-core-harness/base-execute-baseline.json` | Ratchets selected execute-base state: 6/6 files, 325/325 TAP assertions, no runtime buckets | Accepted from `target/perl-core/reports/base-execute.json`; selected execute only, separate from profile-wide parse/compile ratchets |

## Gap Buckets

| Bucket | Meaning | Workstream | LSP impact |
|---|---|---|---|
| `parse_recovery` | Parser diagnostic or error-node salvage | `parser_recovery` | `diagnostics`, `syntax_tree`, `semantic_tokens` |
| `source_decode` | Could not read or decode file | `source_loading` | `workspace_index`, `diagnostics` |
| `hir_lowering` | HIR lowering failure | `hir` | `definition`, `rename`, `diagnostics` |
| `compile_effect` | Unsupported or dynamic compile-time effect | `compile_time_effects` | `definition`, `references`, `diagnostics` |
| `scope_pad` | Scope or pad model gap | `scope_and_pad` | `rename`, `definition`, `diagnostics` |
| `package_stash` | Package, stash, or typeglob fact gap | `package_stash` | `workspace_symbols`, `completion`, `definition` |
| `pragma_feature` | Feature or pragma state gap | `pragma_model` | `diagnostics`, `semantic_tokens` |
| `module_resolution` | `require`, `use`, or include-path fact gap | `module_resolution` | `definition`, `hover`, `completion` |
| `runtime_value_model` | Runtime value, comparison, or statement execution gap | `runtime_value_model` | `compiler_conformance` |
| `runtime_control_flow` | Runtime branch, short-circuit, loop, or statement sequencing gap | `runtime_control_flow` | `compiler_conformance` |
| `runtime_io` | Runtime output, filehandle, stream, or filesystem behavior gap | `runtime_io` | `compiler_conformance` |
| `runtime_regex` | Runtime regex match or substitution behavior gap | `runtime_regex` | `compiler_conformance` |
| `runtime_require_use` | Runtime `require`, `use`, `@INC`, `%INC`, or `$^X` behavior gap | `runtime_require_use` | `compiler_conformance` |
| `runtime_test_harness` | Execute selector, allowlist, TAP harness, or runner invocation gap | `runtime_test_harness` | `compiler_conformance` |
| `cli_switch` | Harness or runner CLI incompatibility | `harness_cli_compat` | `compiler_conformance` |
| `harness_prepare` | Perl tree or harness preparation failure | `harness_integration` | `compiler_conformance` |
| `unknown` | Unclassified failure | `compiler_conformance` | `compiler_conformance`; must be fixed before ratchet |

## Runtime Model

The runtime lane is selected and receipt-driven. It starts from allowlisted
`base/*.t` files that already pass parse and compile receipts, then widens only
when a selected execute receipt proves the next runtime bucket is classified.
Profile-wide execute remains fail-closed.

| Runtime bucket | Workstream owner | First use | Candidate files |
|---|---|---|---|
| `runtime_value_model` | scalar/string/number values, truthiness, comparisons, assignment, expression results, native/unicode value round trips | value or comparison mismatch after parse/compile-clean input reaches execute mode | `base/if.t`, `base/cond.t`, `base/num.t`, `base/translate.t` |
| `runtime_control_flow` | statement sequencing, conditional predicates, loop control, short-circuiting, exit status | selected file reaches execute mode but branches, loops, or statement order diverge | `base/while.t`, `base/cond.t` |
| `runtime_io` | TAP-safe stdout/stderr, `print`, filehandles, filesystem, layers, environment-sensitive IO | selected file needs emitted TAP, diagnostic text, file IO, or process IO beyond the current runner path | next parse/compile-clean file requiring `print` or file IO |
| `runtime_regex` | match, substitution, transliteration-adjacent runtime behavior | selected file reaches a regex operator that cannot be evaluated correctly | `base/pat.t` |
| `runtime_require_use` | `require`, `use`, `@INC`, `%INC`, `$^X` re-entry | runtime module loading or interpreter re-entry blocks a selected file | defer until `comp`/`run` execute planning |
| `runtime_test_harness` | execute selector, allowlist, TAP harness, runner invocation | the runner cannot invoke, record, or classify a selected execute file | any selected execute receipt missing runner records |
| `unknown` | receipt integrity failure | any unclassified runtime failure | must be classified before ratchet or semantic burn-down |

Runtime burn-down PRs must reduce one named bucket or one tight cluster, rerun
the selected execute receipt, update this board or generated report output, and
avoid provider behavior. Compile precondition reds still stay separate; the
current upstream `base` compile ratchet is clean, so remaining unselected
`base/*.t` files are runtime-entry decisions rather than compile blockers.

Current selected execute-base status:

| File | Parse/compile precondition | Execute receipt | Runtime bucket |
|---|---|---|---|
| `base/if.t` | clean | 2/2 TAP assertions pass | none |
| `base/cond.t` | clean | 4/4 TAP assertions pass | none |
| `base/num.t` | clean | 56/56 TAP assertions pass | first `runtime_value_model` burn-down |
| `base/while.t` | clean | 4/4 TAP assertions pass | first `runtime_control_flow` burn-down |
| `base/pat.t` | clean | 2/2 TAP assertions pass | first selected `runtime_regex` slice |
| `base/translate.t` | clean | 257/257 TAP assertions pass | native/unicode `runtime_value_model` round-trip slice |

## Provider Promotion Gates

Compiler-backed provider promotion is not unlocked by a green harness row alone.
The harness receipts prove compiler/runtime substrate behavior; provider rows
must still prove user-visible LSP behavior before cutover. Existing provider
evidence remains tracked in [provider cutover](provider_cutover.md), the
[provider confidence matrix](provider_confidence_matrix.md), the
[provider promotion ledger](provider_promotion_ledger.md), and
[semantic shadow compare](semantic_shadow_compare.md).

Every provider-promotion PR must name exactly one provider surface and one fact
class. It may change live behavior only when all gates below are satisfied:

| Gate | Required evidence | Source |
|---|---|---|
| Harness substrate | Relevant parse/compile or selected execute receipt is ratcheted or explicitly bucketed with no `unknown` / unbucketed failures | this board; `.ci/perl-core-harness/*baseline.json`; `target/perl-core/*` receipts |
| Semantic correctness | Curated-gold, semantic scorecard, or oracle evidence covers the fact class | [semantic scorecard](semantic_scorecard.md), [semantic shadow compare](semantic_shadow_compare.md), oracle receipts where available |
| Provider shadow | Shadow comparison records the candidate, fallback, blocker, confidence, freshness, and dynamic-boundary behavior | [semantic shadow compare](semantic_shadow_compare.md), provider-specific receipt tests |
| Live safety | The live provider has an explicit fallback, blocker, feature gate, rollback strategy, or no-output-change invariant | [provider cutover](provider_cutover.md), [provider confidence matrix](provider_confidence_matrix.md) |
| Real workspace | At least one project-shaped receipt proves useful behavior or preserves fallback for risky generated/dynamic/stale shapes | UX scenario receipts and real-workspace baselines linked from provider status docs |

Initial promotion candidates should be chosen from already-shadowed, low-risk
surfaces: diagnostics explanation, semantic-token classes that preserve
existing output, source-backed definition/reference slices, or explicitly
labeled generated workspace-symbol/document-symbol pilots. Do not promote
module/import hover, broad completion, broad rename, package-wide edits, or
runtime-derived facts until their fact class has matching harness, scorecard,
shadow, and real-workspace evidence.

The first follow-up should be a provider-specific issue, not another umbrella:

```text
provider(<surface>): promote <fact-class> behind receipt gates
```

## Burndown Order

1. Publish this board. Active issue: #3376.
2. Record the first advisory real upstream `base` smoke receipt. Active issue: #3378.
3. Add `comp` compile-mode smoke. Active issue: #3387.
4. Record the first advisory real upstream `comp` smoke receipt. Active issue: #3394.
5. Extract the harness orchestration crate. Active issue: #3420.
6. Add `run` compile-mode smoke. Active issue: #3422.
7. Record the first advisory real upstream `run` smoke receipt. Active issue: #3424.
8. Ratchet real upstream `base`/`comp`/`run` compile receipts. Active issue: #3426.
9. Burn down receipt-backed compiler buckets; the remaining `base` failure is now `compile_effect`.
10. Execute-one for one tiny upstream `t/base/*.t`.
11. Scaffold execute-base with explicit selected base tests and record the first advisory receipt.
12. Ratchet selected execute-base receipts.
13. Publish the runtime bucket model.
14. Burn down the first receipt-backed runtime bucket.
15. Publish compiler-backed provider-promotion gates.
16. Promote compiler-backed provider facts only one surface/fact class at a time after the gates above are met.

## PR Train

| Order | Issue title | PR title | Scope |
|---:|---|---|---|
| 1 | `compiler(harness): publish Perl core harness burndown board` | `docs(perl-core-harness): publish burndown board` | Add this board and link it from the harness status page |
| 2 | `compiler(harness): record first real upstream base smoke receipt` | `docs(perl-core-harness): record first base smoke receipt` | Record ref, discovered count, parse/compile totals, top buckets, and artifact link |
| 3 | `compiler(harness): make real upstream base smoke invoke test runner` | `fix(perl-core-harness): invoke runner for real base smoke` | Landed in #3384; latest receipt has runner-backed parse/compile records |
| 4 | `compiler(harness): add Perl core comp compile-mode smoke receipts` | `feat(perl-core-harness): add comp compile smoke receipts` | Add `profile=comp` discovery/parse/compile/smoke/gap-map receipts |
| 5 | `compiler(harness): record first real upstream comp smoke receipt` | `docs(perl-core-harness): record first comp smoke receipt` | Record ref, discovered count, parse/compile totals, top buckets, and artifact link |
| 6 | `compiler(harness): extract Perl core harness orchestration crate` | `refactor(perl-core-harness): extract orchestration crate` | Move discovery/prepare/run/baseline/smoke orchestration from `xtask` into a private crate |
| 7 | `compiler(harness): add Perl core run compile-mode smoke receipts` | `feat(perl-core-harness): add run compile smoke receipts` | Add `profile=run` discovery/parse/compile/smoke/gap-map receipts |
| 8 | `compiler(harness): record first real upstream run smoke receipt` | `docs(perl-core-harness): record first run smoke receipt` | Record ref, discovered count, parse/compile totals, top buckets, and artifact link |
| 9 | `compiler(harness): ratchet real upstream compile receipts` | `feat(perl-core-harness): ratchet upstream compile smoke receipts` | Ratchet real upstream `base`/`comp`/`run` compile receipts |
| 10 | `compiler(parser): reduce base parse-recovery bucket` | `fix(parser): reduce base parse-recovery harness gaps` | Landed in #3429/#3430 and #3474; burns down `base/term.t` and `base/lex.t` parse recovery, then #3481 moves `base/term.t` from `compile_effect` to compile pass |
| 11 | `compiler(harness): execute one tiny Perl core base test` | `feat(perl-core-harness): execute one base test` | Execute `base/if.t` through explicit `--test base/if.t`, emit real TAP, and keep execute-base future |
| 12 | `compiler(harness): add advisory execute-base receipt scaffold` | `feat(perl-core-harness): add execute-base receipt scaffold` | Execute an explicit selected `base` subset, preserve profile-wide fail-closed behavior, and seed runtime buckets |
| 13 | `compiler(harness): record selected execute-base receipt` | `docs(perl-core-harness): record execute-base receipt` | Record the first selected execute-base receipt for real upstream `base/if.t` and `base/cond.t` |
| 14 | `compiler(harness): ratchet selected execute-base receipts` | `feat(perl-core-harness): ratchet execute-base receipts` | Add the selected execute-base baseline and manual/advisory ratchet command |
| 15 | `compiler(runtime): publish execute-base runtime bucket model` | `docs(runtime): publish execute-base runtime bucket model` | Name runtime buckets, workstreams, selected-file entry rules, and next candidate files |
| 16 | `compiler(runtime): execute base while control-flow receipt` | `feat(runtime): execute base while control flow` | Add selected `base/while.t` execute support, burn down the first `runtime_control_flow` candidate, and ratchet 10/10 selected TAP assertions |
| 17 | `compiler(runtime): execute base pat regex receipt` | `feat(runtime): execute base pat regex slice` | Add selected `base/pat.t` execute support, prove the first bounded `runtime_regex` slice, and ratchet 68/68 selected TAP assertions |
| 18 | `compiler(runtime): execute base translate unicode round-trip receipt` | `feat(runtime): execute base translate unicode round trip` | Add selected `base/translate.t` execute support, prove the native/unicode `runtime_value_model` slice, and ratchet 325/325 selected TAP assertions |
| 19 | `compiler(lsp): publish compiler-backed provider promotion gates` | `docs(lsp): publish provider promotion gates` | Turn H18 from future red into a gated promotion plan tied to harness receipts and provider proof docs |
