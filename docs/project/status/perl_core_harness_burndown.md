# Perl Core Harness Burndown

## Current Claim Boundary

The Perl core harness is a compiler-testing integration lane. It can prepare a
pinned upstream Perl tree on Linux, discover upstream Perl core tests, run
parse-mode and compile-mode synthetic TAP receipts, and produce advisory
real-tree `base`, `comp`, and `run` smoke and gap-map receipts.

It does not execute Perl programs as runtime code and does not claim runtime
conformance. Execute mode remains fail-closed until the execute-one slice lands.

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
| H14 | First bucket burn-down | Green / advisory | #3428, #3429, [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077) | None | `base` `parse_recovery` reduced from 2 to 1 in the accepted compile ratchet; `base/term.t` advanced to `compile_effect` |
| H15 | Execute-one | Red / future | Not started | Start after compile receipts are useful and at least one bucket burn-down lands or is explicitly deferred | One tiny `base/*.t` executes real TAP |
| H16 | Execute-base | Red / future | Not started | Start after H15 lands | `base` runtime receipt exists |
| H17 | Runtime model | Red / future | Not started | Driven by execute-one receipts | Runtime buckets are named and owned |
| H18 | Compiler-backed LSP provider promotion | Red / future | Not started | Start after compiler facts are proven | Provider promotion plan is gated by receipts |

## Latest Receipt Slots

| Receipt | Latest status | Link / artifact |
|---|---:|---|
| `target/perl-core/prepare/<ref>/prepare.json` | Pass for `b62845c7186b0b6a8e4e83419e6b5ef64ceef3ed` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), artifact `perl-core-harness-db5f879540e2d31d39e975ddb1228d12fa5cb838` |
| `target/perl-core/smoke/base/discovery.json` | 9 files discovered | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), `base/cond.t`, `base/if.t`, `base/lex.t`, `base/num.t`, `base/pat.t`, `base/rs.t`, `base/term.t`, `base/translate.t`, `base/while.t` |
| `target/perl-core/smoke/base/parse.json` | 8/9 passed, 1 `parse_recovery` | [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077); remaining parse failure: `base/lex.t` |
| `target/perl-core/smoke/base/compile.json` | 6/9 passed, 1 `parse_recovery`, 2 `compile_effect` | [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077); accepted by `.ci/perl-core-harness/upstream-base-compile-baseline.json`; failures: `base/lex.t`, `base/rs.t`, `base/term.t` |
| `target/perl-core/smoke/base/gap-map.json` | 14/18 mode-file entries passed; buckets: 2 `parse_recovery`, 2 `compile_effect` | [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077); focused runner records confirm `base/term.t` moved from `parse_recovery` to `compile_effect` |
| `target/perl-core/smoke/base/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28730071077](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28730071077) |
| `target/perl-core/smoke/comp/discovery.json` | 25 files discovered | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840); examples include `comp/require.t`, `comp/use.t`, `comp/parser.t`, `comp/proto.t`, `comp/utf.t` |
| `target/perl-core/smoke/comp/parse.json` | 18/25 passed, 7 `parse_recovery` | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840); failures: `comp/decl.t`, `comp/final_line_num.t`, `comp/line_debug.t`, `comp/parser.t`, `comp/proto.t`, `comp/require.t`, `comp/use.t` |
| `target/perl-core/smoke/comp/compile.json` | 8/25 passed, 7 `parse_recovery`, 10 `compile_effect` | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840); compile-effect failures include `comp/filter_exception.t`, `comp/fold.t`, `comp/form_scope.t`, `comp/hints.t`, `comp/multiline.t`, `comp/our.t`, `comp/parser_run.t`, `comp/redef.t`, `comp/retainedlines.t`, `comp/utf.t` |
| `target/perl-core/smoke/comp/gap-map.json` | 26/50 mode-file entries passed; buckets: 14 `parse_recovery`, 10 `compile_effect` | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840) |
| `target/perl-core/smoke/comp/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28711942840](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28711942840) |
| `target/perl-core/smoke/run/discovery.json` | 28 files discovered | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803), artifact `perl-core-harness-597feab33627b5b0469434a0eb84b605aaa4fd52`; examples include `run/fresh_perl.t`, `run/script.t`, `run/switch-I-and-M.t`, `run/switchM.t`, `run/switches.t` |
| `target/perl-core/smoke/run/parse.json` | 18/28 passed, 10 `parse_recovery` | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803); failures: `run/dtrace.t`, `run/exit.t`, `run/locale.t`, `run/runenv.t`, `run/runenv_randseed.t`, `run/script.t`, `run/switchC.t`, `run/switchd.t`, `run/switches.t`, `run/todo.t` |
| `target/perl-core/smoke/run/compile.json` | 1/28 passed, 10 `parse_recovery`, 17 `compile_effect` | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803); compile-effect failures include `run/cloexec.t`, `run/fresh_perl.t`, `run/noswitch.t`, `run/runenv_hashseed.t`, `run/switch-I-and-M.t`, `run/switch0.t`, `run/switchDx.t`, `run/switchF.t`, `run/switchM.t`, `run/switchx.t` |
| `target/perl-core/smoke/run/gap-map.json` | 19/56 mode-file entries passed; buckets: 20 `parse_recovery`, 17 `compile_effect` | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803) |
| `target/perl-core/smoke/run/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28726563803](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28726563803) |
| `.ci/perl-core-harness/upstream-base-compile-baseline.json` | Ratchets 6/9 compile pass state; buckets: 1 `parse_recovery`, 2 `compile_effect` | Accepted from post-#3429 `target/perl-core/smoke/base/compile.json`; separate from generated fixture baseline |
| `.ci/perl-core-harness/upstream-comp-compile-baseline.json` | Ratchets 8/25 compile pass state; buckets: 7 `parse_recovery`, 10 `compile_effect` | Accepted from `target/perl-core/smoke/comp/compile.json`; separate from generated fixture baseline |
| `.ci/perl-core-harness/upstream-run-compile-baseline.json` | Ratchets 1/28 compile pass state; buckets: 10 `parse_recovery`, 17 `compile_effect` | Accepted from `target/perl-core/smoke/run/compile.json`; separate from generated fixture baseline |

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
| `cli_switch` | Harness or runner CLI incompatibility | `harness_cli_compat` | `compiler_conformance` |
| `harness_prepare` | Perl tree or harness preparation failure | `harness_integration` | `compiler_conformance` |
| `unknown` | Unclassified failure | `compiler_conformance` | `compiler_conformance`; must be fixed before ratchet |

## Burndown Order

1. Publish this board. Active issue: #3376.
2. Record the first advisory real upstream `base` smoke receipt. Active issue: #3378.
3. Add `comp` compile-mode smoke. Active issue: #3387.
4. Record the first advisory real upstream `comp` smoke receipt. Active issue: #3394.
5. Extract the harness orchestration crate. Active issue: #3420.
6. Add `run` compile-mode smoke. Active issue: #3422.
7. Record the first advisory real upstream `run` smoke receipt. Active issue: #3424.
8. Ratchet real upstream `base`/`comp`/`run` compile receipts. Active issue: #3426.
9. Burn down the first receipt-backed compiler bucket; prefer the `base` `parse_recovery` cluster before runtime work.
10. Start execute-one for one tiny upstream `t/base/*.t`.
11. Plan execute-base from the runtime buckets found by execute-one.
12. Promote compiler-backed provider facts only after receipt-backed compiler facts are proven.

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
| 10 | `compiler(parser): reduce base parse-recovery bucket` | `fix(parser): reduce base parse-recovery harness gaps` | Landed in #3429 and accepted in #3430; burns down `base/term.t` parse recovery and advances `base/lex.t` to the next parser gap |
| 11 | `compiler(harness): execute one tiny Perl core base test` | `feat(perl-core-harness): execute one base test` | Execute one tiny upstream `base/*.t` and record runtime buckets |
