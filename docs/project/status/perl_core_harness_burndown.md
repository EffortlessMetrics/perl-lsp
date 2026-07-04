# Perl Core Harness Burndown

## Current Claim Boundary

The Perl core harness is a compiler-testing integration lane. It can prepare a
pinned upstream Perl tree on Linux, discover upstream Perl core tests, run
parse-mode and compile-mode synthetic TAP receipts, and produce advisory
real-tree `base` smoke and gap-map receipts.

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
| H10 | `comp` compile smoke | Yellow / advisory workflow | #3387, #3394 | Record first real upstream `comp` receipt after workflow run | `comp` smoke writes discovery/parse/compile/smoke/gap-map receipts |
| H11 | `run` compile smoke | Red | Not started | Start after H10 real receipt is recorded | `run` smoke writes discovery/parse/compile/smoke/gap-map receipts |
| H12 | Real upstream compile ratchets | Red | Not started | Start after H10/H11 are stable | `base`/`comp`/`run` compile receipts are ratcheted or deferral is explicit |
| H13 | Execute-one | Red / future | Not started | Start after compile receipts are useful | One tiny `base/*.t` executes real TAP |
| H14 | Execute-base | Red / future | Not started | Start after H13 lands | `base` runtime receipt exists |
| H15 | Runtime model | Red / future | Not started | Driven by execute-one receipts | Runtime buckets are named and owned |
| H16 | Compiler-backed LSP provider promotion | Red / future | Not started | Start after compiler facts are proven | Provider promotion plan is gated by receipts |

## Latest Receipt Slots

| Receipt | Latest status | Link / artifact |
|---|---:|---|
| `target/perl-core/prepare/<ref>/prepare.json` | Pass for `b62845c7186b0b6a8e4e83419e6b5ef64ceef3ed` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), artifact `perl-core-harness-db5f879540e2d31d39e975ddb1228d12fa5cb838` |
| `target/perl-core/smoke/base/discovery.json` | 9 files discovered | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088), `base/cond.t`, `base/if.t`, `base/lex.t`, `base/num.t`, `base/pat.t`, `base/rs.t`, `base/term.t`, `base/translate.t`, `base/while.t` |
| `target/perl-core/smoke/base/parse.json` | 7/9 passed, 2 `parse_recovery` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088); failures: `base/lex.t`, `base/term.t` |
| `target/perl-core/smoke/base/compile.json` | 6/9 passed, 2 `parse_recovery`, 1 `compile_effect` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088); failures: `base/lex.t`, `base/rs.t`, `base/term.t` |
| `target/perl-core/smoke/base/gap-map.json` | 13/18 mode-file entries passed; buckets: 4 `parse_recovery`, 1 `compile_effect` | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088) |
| `target/perl-core/smoke/base/smoke.json` | Pass for receipt integrity; structural failures empty | [run 28707735088](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/28707735088) |
| `target/perl-core/smoke/comp/discovery.json` | TBD | Fill after first advisory `comp` workflow run |
| `target/perl-core/smoke/comp/parse.json` | TBD | Fill after first advisory `comp` workflow run |
| `target/perl-core/smoke/comp/compile.json` | TBD | Fill after first advisory `comp` workflow run |
| `target/perl-core/smoke/comp/gap-map.json` | TBD | Fill after first advisory `comp` workflow run |
| `target/perl-core/smoke/comp/smoke.json` | TBD | Fill after first advisory `comp` workflow run |

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
5. Add `run` compile-mode smoke.
6. Ratchet real upstream `base`/`comp`/`run` compile receipts, or record an explicit board decision explaining why ratchet is deferred.
7. Start execute-one for one tiny upstream `t/base/*.t`.
8. Plan execute-base from the runtime buckets found by execute-one.
9. Promote compiler-backed provider facts only after receipt-backed compiler facts are proven.

## PR Train

| Order | Issue title | PR title | Scope |
|---:|---|---|---|
| 1 | `compiler(harness): publish Perl core harness burndown board` | `docs(perl-core-harness): publish burndown board` | Add this board and link it from the harness status page |
| 2 | `compiler(harness): record first real upstream base smoke receipt` | `docs(perl-core-harness): record first base smoke receipt` | Record ref, discovered count, parse/compile totals, top buckets, and artifact link |
| 3 | `compiler(harness): make real upstream base smoke invoke test runner` | `fix(perl-core-harness): invoke runner for real base smoke` | Landed in #3384; latest receipt has runner-backed parse/compile records |
| 4 | `compiler(harness): add Perl core comp compile-mode smoke receipts` | `feat(perl-core-harness): add comp compile smoke receipts` | Add `profile=comp` discovery/parse/compile/smoke/gap-map receipts |
| 5 | `compiler(harness): add Perl core run compile-mode smoke receipts` | `feat(perl-core-harness): add run compile smoke receipts` | Add `profile=run` discovery/parse/compile/smoke/gap-map receipts |
| 6 | `compiler(harness): ratchet real upstream compile receipts` | `feat(perl-core-harness): ratchet upstream compile smoke receipts` | Ratchet real upstream `base`/`comp`/`run` compile receipts |
| 7 | `compiler(harness): execute one tiny Perl core base test` | `feat(perl-core-harness): execute one base test` | Execute one tiny upstream `base/*.t` and record runtime buckets |
