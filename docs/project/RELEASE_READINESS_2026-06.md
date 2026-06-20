# Release-Readiness Bundle - 2026-06 Campaign

**Status**: release-staged, maintainer-held. Do not tag, publish, or dispatch a release without Steven's explicit approval.
**Updated**: 2026-06-20
**Main SHA**: `c94d50e8b35dbc2511b1a2508f359af2fd47d251`
**Workspace version (`Cargo.toml`)**: `0.16.0`
**Last release tag**: `v0.16.0` at `b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d` (2026-06-06)

Note: `v0.16.0` is a real tag, but it is not on current `origin/main` ancestry (`git describe origin/main` resolves from `v0.15.0`). Do not use a naive "commits since v0.16.0" count as release evidence without resolving that tag-lineage question.

---

## Release Verdict

| Area | Verdict | Evidence |
|------|---------|----------|
| P0 blockers | PASS | No open P0 surfaced by this closeout pass. |
| P1 blockers | PASS with caveats | The measured multi-root `workspace/symbol` P1 is closed by #1522 and the focused smoke is green. Full CPAN-scale parser accuracy remains caveated because the full ratchet is still runner-dark. |
| Release dispatch | HELD | This bundle stages the release decision only. No tag, publish, marketplace upload, crates.io publish, Docker image, or GitHub Release is authorized here. |

---

## Gate State

Latest merged PR checked: [#1603](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1603), merge commit `c94d50e8b35dbc2511b1a2508f359af2fd47d251`, merged 2026-06-20T02:42:44Z.

| Required / decision gate | Status on #1603 | Notes |
|--------------------------|-----------------|-------|
| `Perl LSP Rust Small Result` | PASS | GitHub check completed 2026-06-20T02:24:42Z. |
| `ripr+ New Gap Gate` | PASS | GitHub check completed 2026-06-20T02:40:45Z. |
| `Codecov / Patch 95` | PASS | GitHub check completed 2026-06-20T02:40:48Z; external `codecov/patch` also passed. |
| `PR Smoke (Fast Feedback)` | PASS | GitHub check completed 2026-06-20T02:40:23Z. |
| `CI Gate (Merge-Blocking)` | PASS | GitHub check completed 2026-06-20T02:40:27Z. |

Coverage semantics after #1482/#1549/#1576/#1581/#1586: coverage verdicts are scoped to coverage shortfall/setup/routing failures. Routed test failures belong to test-named gates, not the Codecov/Patch-95 verdict.

Runner semantics after #1528: self-hosted disk preflight can fail over to GitHub-hosted only for disk-preflight failures. Real test or quality-gate failures are not masked by fallback.

---

## Shipping Scope Since The Prior Draft

This table records the late closeout scope that updates the older 2026-06 draft. The changelog PR [#1419](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1419) now carries the user-facing release-note wording.

| Theme | Merged PRs | Release claim |
|-------|------------|---------------|
| Parser and quick-fix robustness | #1457, #1460, #1461, #1463, #1481, #1483 | More valid Perl and malformed mid-edit input stays quiet; UTF-8 mid-codepoint code-action ranges no longer panic. |
| Single-root LSP readiness | #1551 | Baseline PR smoke blockers were repaired: hover preservation, workspace file-op/index waits, empty workspace-folder no-op handling, progress harness determinism. |
| Multi-root project model | #1522 | `workspace/symbol` waits for active indexing, includes `workspaceFolderUri`, and returns deterministic multi-root results. |
| Diagnostics accuracy | #1562 | `$self->{name}` / `$ref->{key}` no longer produce false `UnquotedBareword` diagnostics under `use strict`. |
| References and folding | #1597, #1560 | Partial-index reference fallback avoids documents-lock re-entry; heredoc/multiline folding boundaries are corrected. |
| DAP honesty | #1430, #1444, #1496, #1498 | Variable-reference spaces are typed; evaluate and stack parsing report the real invalid input instead of stale or misleading state. |
| Deterministic completion value | #1532, #1573, #1579, #1585 | Try::Tiny, Mojolicious, DBI, and indexed package receiver completions are offered only with supporting evidence. |
| Measurement substrate | #1482, #1520, #1528, #1530, #1539, #1549, #1576, #1581, #1586 | Coverage/test semantics, CPAN bounded ratchet mode, runner disk failover, workflow privilege analysis, and routed-suite expectations are current. |

Correction from earlier draft: #1524 is closed, not merged. The arrow-deref diagnostic fix landed as #1562.

---

## Product Smoke Receipts

All commands below were run locally on Windows against `origin/main`/`c94d50e8` lineage during the 2026-06-20 closeout pass unless noted. Cargo target output was redirected outside the worktree under `D:\cargo-target\perl-lsp-release-smoke*`.

### Parser / Robustness

| Surface | Command | Result |
|---------|---------|--------|
| UTF-8 and invalid LSP parser ranges | `cargo test -p perl-parser --test lsp_protocol_robustness_tests --profile agent --locked -- --nocapture` | PASS, 3 passed |
| Unicode / emoji / CJK parser cases | `cargo test -p perl-parser --test comprehensive_unicode_edge_cases --profile agent --locked -- --nocapture` | PASS, 8 passed |
| Nested variable-list structures | `cargo test -p perl-parser --test nodekind_combination_data_structures variable_list --profile agent --locked -- --nocapture` | PASS, 1 passed |
| Variable declaration attributes | `cargo test -p perl-parser --test nodekind_combination_edge_cases attribute_shapes --profile agent --locked -- --nocapture` | PASS, 1 passed |
| Custom/sub attributes integration | `cargo test -p perl-parser --test integration_new_features attributes --profile agent --locked -- --nocapture` | PASS, 1 passed |
| Negative keyword barewords before `=>` | `cargo test -p perl-parser --features constant-advanced --test declaration_micro_tests constant --profile agent --locked -- --nocapture` | PASS, 7 passed |
| Code-action UTF-8 diagnostic ranges | `cargo test -p perl-lsp-rs --lib diagnostic_ranges_do_not_panic --profile agent --locked -- --nocapture` | PASS, 2 passed |
| UTF-8 snapping / reversed ranges | `cargo test -p perl-lsp-rs --lib slice_in_range --profile agent --locked -- --nocapture` | PASS, 2 passed |

### LSP / Editor

| Surface | Command | Result |
|---------|---------|--------|
| Multi-root `workspace/symbol` after #1522 | `cargo test -p perl-lsp-rs --features "workspace expose_lsp_test_api" --test multi_root_workspace_tests workspace_symbol --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 6 passed; includes 100-iteration deterministic smoke and `workspaceFolderUri` cases |
| Arrow-deref strict-bareword diagnostics after #1562 | `cargo test -p perl-semantic-analyzer --test scope_and_symbol_tests arrow_deref --profile agent --locked -- --nocapture` | PASS, 4 passed |
| LSP smoke | `cargo test -p perl-lsp-rs --test lsp_smoke --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 17 tests |
| LSP E2E smoke | `cargo test -p perl-lsp-rs --test lsp_smoke_e2e --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 22 passed |
| Rename workflow | `cargo test -p perl-lsp-rs --test navigation_regression_tests rename --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 5 passed |
| Formatting smoke | `cargo test -p perl-lsp-rs --lib formatting --profile agent --locked -- --nocapture` | PASS, 5 passed |
| Workspace configuration/settings | `cargo test -p perl-lsp-rs --test workspace_resolution_tests configuration --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 6 passed |

### DAP

| Surface | Command | Result |
|---------|---------|--------|
| DAP edge cases | `cargo test -p perl-dap --test dap_edge_cases_test test_dap --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 12 passed |
| Evaluate matrix | `cargo test -p perl-dap --test dap_evaluate_comprehensive_tests test_evaluate --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 42 passed |
| Eval reference cache/stale resume | `cargo test -p perl-dap --test eval_ref_cache_miss_resume_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 10 passed |
| Pause signal delivery | `cargo test -p perl-dap --test pause_signal_delivery_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 2 passed |

### Easy / First-Run

| Surface | Command | Result |
|---------|---------|--------|
| CLI health/check smoke | `cargo test -p perl-lsp-rs --test cli_smoke --profile agent --locked -- --nocapture` | PASS, 17 passed |
| VS Code extension unit smoke | `npm ci`; `npm test -- --runInBand` in `vscode-extension` | PASS, 22 suites / 597 tests |
| VS Code extension-host managed binary smoke | `PERL_LSP_SMOKE_RECEIPTS_DIR=D:\tmp\perl-lsp-vscode-smoke-receipts-20260620 npm run test:integration` | PASS, 1 extension-host test; current extension development build opened in VS Code 1.125.1, managed reinstall ran twice, checksum verified, health check found Perl 5.042000 and LSP binary |

Caveat: no release-candidate VSIX install smoke was run. The local VSIX path currently invokes `vscode-extension/scripts/bundle-lsp.js`, which builds into repo-local `target/release`; that violates the storage discipline for this closeout pass. The integration smoke proves the extension development-host and managed-binary path, not final packaged VSIX contents.

---

## CPAN Ratchet Status

| Mode | Status | Evidence |
|------|--------|----------|
| Bounded representative subset | PASS | Workflow run [27857894825](https://github.com/EffortlessMetrics/perl-lsp-swarm/actions/runs/27857894825), head `c94d50e8`, completed success 2026-06-20T02:56:15Z. Receipt `bounded-sweep.json`: 75 roots, 4,172 files, 4,130 clean, 36 with parser errors, 0 catastrophic, elapsed 1.87s. |
| Full CPAN ratchet | UNKNOWN / CAVEATED | Latest inspected full run `27817541126` on head `504b4673` failed 2026-06-19T09:29:43Z with runner shutdown/exit 143 during batch 10 after batch 9 timeout. Do not claim CPAN-scale accuracy from this bundle. |

Release wording may say "bounded CPAN top-50 profile passed" with the counts above. It must not say "CPAN-scale accuracy passed" until the full ratchet has a fresh successful receipt.

---

## Known Deferred Work

| Item | State | Release handling |
|------|-------|------------------|
| [#991](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/991) DAP trust lane | OPEN | Deferred; not required for this staged release because the focused DAP honesty matrix above is green. |
| [#676](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/676) fuzz/config expansion | OPEN | Deferred; not part of this release gate. |
| Full CPAN ratchet | Dark/caveated | Requires restored full-run receipt or explicit release caveat. |
| Release-candidate VSIX install smoke | Not run | Extension-host integration smoke passed; package-level install proof remains caveated unless a storage-safe VSIX path is run. |
| Broad feature waves / dependabot | Open queue | Deferred by release-closeout scope. |

---

## Release Channel Checklist

- [x] Main SHA recorded.
- [x] Required gate state recorded from latest merged PR.
- [x] Coverage/test gate semantics recorded after #1482/#1549 and follow-ups.
- [x] Parser, LSP, DAP, and easy-path smoke receipts recorded.
- [x] Bounded CPAN receipt recorded.
- [x] Full CPAN and VSIX caveats recorded.
- [x] Changelog PR #1419 refreshed from merged scope only.
- [x] Deferred scope listed.
- [x] Explicit maintainer dispatch hold preserved.

---

## Explicit Non-Goal

This document does not authorize or initiate a release. No tag, no `crates.io` publish, no VS Code Marketplace upload, no Docker image, and no GitHub Release will be created until Steven explicitly approves and dispatches the release workflow.

---

Generated from current repo/GitHub state and local smoke receipts on 2026-06-20. Claims above are limited to the commands and receipts named in this file.
