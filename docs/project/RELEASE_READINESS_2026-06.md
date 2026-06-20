# Release-Readiness Bundle - 2026-06 Campaign

**Status**: release-staged, maintainer-held. Do not tag, publish, or dispatch a release without Steven's explicit approval.
**Updated**: 2026-06-20
**Release payload SHA**: `03823e40e9bcdfc8cc8418306dce5f877affa3d9`
**Current `origin/main` SHA at refresh**: `03823e40e9bcdfc8cc8418306dce5f877affa3d9`
**Previous readiness bundle merge SHA**: `b7ae94b322412e4c86052eb3914679e4f002149f`
**Workspace version (`Cargo.toml`)**: `0.16.0`
**Last release tag**: `v0.16.0` at `b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d` (2026-06-06)

Note: the release payload SHA is the latest audited merged closeout commit that changed release-facing product, CI, or changelog scope. It includes #1867, which restored storage-safe VSIX packaging verification and the package-level published-VSIX smoke receipt, #1878, which repaired native-tooling git context in Windows-linked worktrees for release evidence regeneration, #1875, which keeps context-specific completions grouped by semantic family, #1881, which repaired RIPR evidence git context in Windows-linked worktrees, #1886, which cleans stale gitignored workspace `target/` directories before self-hosted runner checkout, #1882, which refreshes cached POD hover content after external module-file edits, #1876, which makes file-scoped semantic fact IDs collision-proof across identical source files, #1806, #1810, and #1759, which tighten DAP request ordering, scope shape, and capability advertising, and #1793, which guards LSP transport body-offset arithmetic. The previous readiness bundle merge SHA records the doc-only staging bundle that captured the closeout claims through #1882. Verify the current `origin/main` SHA again at dispatch time; later doc-only clarification commits do not expand the product claim.

Note: `v0.16.0` is a real tag, but it is not on current `origin/main` ancestry (`git describe origin/main` resolves from `v0.15.0`). Do not use a naive "commits since v0.16.0" count as release evidence without resolving that tag-lineage question.

---

## Release Verdict

| Area | Verdict | Evidence |
|------|---------|----------|
| P0 blockers | PASS | No open P0 surfaced by this closeout pass. |
| P1 blockers | PASS with caveats | The measured multi-root `workspace/symbol` P1 is closed by #1522 and the focused smoke is green. Full CPAN-scale parser accuracy remains caveated because the full ratchet is still runner-dark. #1759's PR-fast aggregate failed on a stale DAP stack-trace branch expectation; the exact failing test passes on current main, but this bundle records the aggregate caveat. |
| Release dispatch | HELD | This bundle stages the release decision only. No tag, publish, marketplace upload, crates.io publish, Docker image, or GitHub Release is authorized here. |

---

## Gate State

Latest merged release-scope product PR checked: [#1759](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1759), merge commit `03823e40e9bcdfc8cc8418306dce5f877affa3d9`, merged 2026-06-20T13:53:47Z. It follows DAP scope pagination hints [#1810](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1810), merge commit `91b4a053ce7014d7f0c9caad839539f81694d4e9`, LSP transport checked arithmetic [#1793](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1793), merge commit `9426183ecf5b8fb290e00055bb80d4efcbadba8b`, DAP request-order validation [#1806](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1806), merge commit `d4342dc5999f47884286d9ba288f73fc37b849c3`, file-scoped semantic IDs [#1876](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1876), merge commit `801f5072df8095355490ec29ce8f692beeac3b16`, the doc-only readiness refresh [#1887](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1887), merge commit `b7ae94b322412e4c86052eb3914679e4f002149f`, and the #1882/#1886/#1881/#1875/#1878/#1867 closeout scope recorded in that prior bundle.

Gate caveat: #1759's PR head `06ce88c1357068e6f4c818c0805eeaead7654854` merged with mixed check state. Coverage, RIPR aggregate, Rust Small aggregate, CI shards, UX, LSP memory, workflow trigger lint, and UB advisory passed; `PR Smoke (Fast Feedback)` failed because `unit_routed_full` still saw stale branch behavior for `test_total_frames_is_not_window_size`, and `CI Gate (Merge-Blocking)` failed only through that PR-smoke dependency. Current `origin/main` includes #1810 before #1759, and the exact failing test passed locally on current main with `./scripts/cargo-safe test -p perl-dap --test stack_trace_provider_tests test_total_frames_is_not_window_size --profile agent --locked -- --nocapture` on 2026-06-20. This targeted receipt proves the observed stale branch failure is not present at payload SHA `03823e40e9bcdfc8cc8418306dce5f877affa3d9`; it is not a substitute for a fresh full PR-fast aggregate on current main.

| Required / decision gate | Status | Notes |
|--------------------------|-----------------|-------|
| `Perl LSP Rust Small Result` | PASS | #1759 GitHub check completed 2026-06-20T13:38:10Z. CX43 primary failed, disk-full fallback passed, and the aggregate result passed. |
| `ripr+ New Gap Gate` | PASS | #1759 GitHub check completed 2026-06-20T13:53:43Z. CX43 primary failed, disk-full fallback passed, and the aggregate result passed. |
| `Codecov / Patch 95` | PASS | #1759 GitHub check completed 2026-06-20T13:51:22Z; Codecov app patch check reported 100.00% diff coverage at 2026-06-20T13:51:45Z. |
| `Workflow Trigger Lint` | PASS | #1759 GitHub check completed 2026-06-20T13:41:21Z after #1816/#1817 removed required-workflow path filters. |
| `PR Smoke (Fast Feedback)` | CAVEATED | #1759 GitHub check failed 2026-06-20T13:59:51Z on stale branch `unit_routed_full` test `test_total_frames_is_not_window_size`; current-main targeted re-smoke passed locally as recorded above. |
| `CI Gate (Merge-Blocking)` | CAVEATED | #1759 GitHub aggregate failed 2026-06-20T14:00:01Z because `pr-smoke` failed; all CI Gate shards passed. |
| `UX Regression Tests` | PASS | #1759 GitHub check completed 2026-06-20T13:44:32Z; `UX Regression Gate` completed 2026-06-20T13:50:36Z. |
| `droid-review` | CANCELLED | #1759 advisory check was cancelled at 2026-06-20T14:04:29Z. CodeRabbit status reported review completed; no droid-review product-smoke claim is made. |
| `LSP Memory Smoke` | PASS | #1759 GitHub check completed 2026-06-20T13:41:45Z. |
| `UB Review Advisory on GitHub Hosted` | PASS | #1759 advisory check completed 2026-06-20T14:01:22Z. Advisory proof, not product-smoke proof. |

Advisory/non-required state observed during this refresh: #1759 routed `Perl LSP Rust Small` and `ripr+` to CX43. Both primary CX43 jobs failed quickly, both disk-full fallback jobs completed successfully, and both aggregate result checks passed. #1759 `workflow-policy-lint` completed successfully at 2026-06-20T13:41:21Z. Required merge gates were not uniformly green on the stale #1759 PR branch because PR-fast failed as described above; the current-main targeted DAP receipt closes the observed stale-branch test failure only.

Coverage semantics after #1482/#1549/#1576/#1581/#1586: coverage verdicts are scoped to coverage shortfall/setup/routing failures. Routed test failures belong to test-named gates, not the Codecov/Patch-95 verdict.

Runner semantics after #1528: self-hosted disk preflight can fail over to GitHub-hosted only for disk-preflight failures. Real test or quality-gate failures are not masked by fallback.

Required workflow trigger semantics after #1816/#1817: required CI workflows stay trigger-visible for PR events. Docs/assets-only PRs skip expensive Rust jobs inside the workflow guard rather than by `paths-ignore`, so Workflow Trigger Lint remains enforced.

---

## Shipping Scope Since The Prior Draft

This table records the late closeout scope that updates the older 2026-06 draft. The changelog PR [#1419](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1419) is merged and carries the user-facing release-note wording.

| Theme | Merged PRs | Release claim |
|-------|------------|---------------|
| Parser and quick-fix robustness | #1457, #1460, #1461, #1463, #1481, #1483 | More valid Perl and malformed mid-edit input stays quiet; UTF-8 mid-codepoint code-action ranges no longer panic. |
| Single-root LSP readiness | #1551 | Baseline PR smoke blockers were repaired: hover preservation, workspace file-op/index waits, empty workspace-folder no-op handling, progress harness determinism. |
| Multi-root project model | #1522 | `workspace/symbol` waits for active indexing, includes `workspaceFolderUri`, and returns deterministic multi-root results. |
| Semantic identity substrate | #1876 | File-scoped semantic fact IDs include `FileId`, preventing identical source in different files from colliding in anchors, entities, occurrences, and file-scoped edges while preserving the file-neutral reference-source sentinel. |
| Diagnostics accuracy | #1562 | `$self->{name}` / `$ref->{key}` no longer produce false `UnquotedBareword` diagnostics under `use strict`. |
| References, docs links, hover, and folding | #1597, #1638, #1560, #1795, #1882 | Partial-index reference fallback avoids documents-lock re-entry; perldoc/MetaCPAN targets share one validated resolver; POD `L<>` document links are exposed and resolved; cached POD hover content refreshes after external module-file edits; heredoc/multiline folding boundaries are corrected. |
| DAP honesty | #1430, #1444, #1496, #1498, #1806, #1810, #1759 | Variable-reference spaces are typed; evaluate and stack parsing report the real invalid input instead of stale or misleading state; request ordering now rejects out-of-sequence initialize/launch/configurationDone flows; scope responses expose DAP pagination hint fields; capability flags match routed restart-frame, step-in-targets, and terminate-threads handlers. |
| Transport robustness | #1793 | `Content-Length` frame parsing uses checked arithmetic for `body_start` and recovers through the existing invalid-length path on overflow. |
| Deterministic completion value | #1532, #1573, #1579, #1585, #1875 | Try::Tiny, Mojolicious, DBI, and indexed package receiver completions are offered only with supporting evidence; context-specific completion families keep stable semantic sort tiers. |
| Measurement substrate | #1482, #1520, #1528, #1530, #1539, #1549, #1576, #1581, #1586, #1688, #1689, #1816, #1817, #1833, #1878, #1881, #1886 | Coverage/test semantics, CPAN bounded ratchet mode, runner disk failover, workflow privilege analysis, routed-suite expectations, draft-ripr neutrality, trigger-safe docs/assets-only CI skipping, Windows-worktree coverage-baseline recovery, native-tooling git-context recovery, RIPR evidence git-context recovery, and self-hosted workspace-target cleanup are current. |

Correction from earlier draft: #1524 is closed, not merged. The arrow-deref diagnostic fix landed as #1562.

---

## Product Smoke Receipts

All product-smoke commands below were run locally on Windows against `origin/main`/`c94d50e8` lineage during the 2026-06-20 closeout pass unless noted. Later main commits through release payload `03823e40e9bcdfc8cc8418306dce5f877affa3d9` include docs/CI release-staging changes, the targeted POD document-link expansion in #1795, the storage-safe VSIX packaging/reinstall-smoke closeout in #1867, the native-tooling git-context repair in #1878, the context-specific completion sort-tier repair in #1875, the RIPR evidence git-context repair in #1881, the self-hosted workspace-target cleanup in #1886, the POD hover cache refresh in #1882, the file-scoped semantic ID repair in #1876, DAP request-order/scope/capability fixes in #1806/#1810/#1759, and the LSP transport checked-arithmetic repair in #1793. Cargo target output was redirected outside the worktree under `D:\cargo-target\perl-lsp-release-smoke*`, the repo safe target wrapper's external cache, or the specific external target dir named in the command.

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
| LSP frame body offset arithmetic | `cargo test -p perl-lsp-rs-core --lib framer_body_start_offset_is_correct_and_state_resets --profile agent --locked` | PASS in #1793 verification; `Content-Length` body slicing stays exact and framer state resets across consecutive frames |

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
| POD hover cache refresh after external edits | `cargo test -p perl-lsp-rs --lib --profile agent --locked pod_hover_cache_refreshes_after_external_file_edit -- --nocapture` | PASS in #1882 verification; merged CI reported 36 passing checks |
| File-scoped semantic IDs | `cargo test -p perl-symbol -p perl-workspace --profile agent --locked` | PASS in #1876 verification; includes reparse-stable IDs, cross-file distinct IDs for identical source, and entity-to-anchor file-id recovery |

### DAP

| Surface | Command | Result |
|---------|---------|--------|
| DAP edge cases | `cargo test -p perl-dap --test dap_edge_cases_test test_dap --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 12 passed |
| Evaluate matrix | `cargo test -p perl-dap --test dap_evaluate_comprehensive_tests test_evaluate --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 42 passed |
| Eval reference cache/stale resume | `cargo test -p perl-dap --test eval_ref_cache_miss_resume_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 10 passed |
| Pause signal delivery | `cargo test -p perl-dap --test pause_signal_delivery_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 2 passed |
| DAP request-order validation | `cargo test -p perl-dap --lib test_launch_before_initialize_should_fail --profile agent --locked`; `cargo test -p perl-dap --lib test_configuration_done_without_launch_should_fail --profile agent --locked` | PASS in #1806 verification; out-of-sequence initialize/launch/configurationDone flows return explicit failures |
| DAP scope pagination fields | `cargo test -p perl-dap --test dap_coverage_audit_tests test_scope_includes_pagination_hints --profile agent --locked` | PASS in #1810 verification; `namedVariables` and `indexedVariables` round-trip and omit when unset |
| DAP capability advertising | `cargo test -p perl-dap --test dap_capability_advertising_tests --profile agent --locked` | PASS in #1759 verification; restart-frame, step-in-targets, and terminate-threads capabilities are advertised when handlers exist |
| Current-main stackTrace pagination regression | `./scripts/cargo-safe test -p perl-dap --test stack_trace_provider_tests test_total_frames_is_not_window_size --profile agent --locked -- --nocapture` | PASS on current main `03823e40e9bcdfc8cc8418306dce5f877affa3d9`; 1 passed |

### Easy / First-Run

| Surface | Command | Result |
|---------|---------|--------|
| CLI health/check smoke | `cargo test -p perl-lsp-rs --test cli_smoke --profile agent --locked -- --nocapture` | PASS, 17 passed |
| VS Code extension unit smoke | `npm ci`; `npm test -- --runInBand` in `vscode-extension` | PASS, 22 suites / 597 tests |
| VS Code extension-host managed binary smoke | `PERL_LSP_SMOKE_RECEIPTS_DIR=D:\tmp\perl-lsp-vscode-smoke-receipts-20260620 npm run test:integration` | PASS, 1 extension-host test; current extension development build opened in VS Code 1.125.1, managed reinstall ran twice, checksum verified, health check found Perl 5.042000 and LSP binary |
| Storage-safe VSIX packaging | `CARGO_TARGET_DIR=D:\cargo-target\perl-lsp-vsix-bundle-target-dir npm run verify:marketplace` in `vscode-extension` on #1867 | PASS, packaged `perl-lsp-rs-0.16.0.vsix` while locating the release binary from the external Cargo target dir |
| Release-candidate VSIX install smoke | `PERL_LSP_PUBLISHED_EXTENSION_SOURCE=vsix PERL_LSP_PUBLISHED_VSIX_PATH=H:\Code\Rust\perl-lsp-vsix-bundle-target-dir\vscode-extension\perl-lsp-rs-0.16.0.vsix PERL_LSP_PUBLISHED_EXTENSION_VERSION=0.16.0 PERL_LSP_REQUIRE_STRUCTURED_COMMANDS=1 PERL_LSP_SMOKE_RECEIPTS_DIR=D:\tmp\perl-lsp-vsix-smoke-receipts-20260620-fixed npm run test:published` in `vscode-extension` on #1867 | PASS, 1 published-extension smoke in VS Code 1.125.1; receipts under `D:\tmp\perl-lsp-vsix-smoke-receipts-20260620-fixed\vsix\windows` show extension `0.16.0`, managed binary `0.16.0`, target `x86_64-pc-windows-msvc`, two structured reinstalls, checksum verified, lock-held second reinstall exercised, Perl 5.042000, and LSP binary health OK |

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
| Fresh full PR-fast aggregate on payload SHA | Caveated | #1759's stale PR branch failed PR-fast; the exact failing DAP stack-trace test passes on current main, but no fresh full PR-fast aggregate is claimed here. |
| Broad feature waves / dependabot | Open queue | Deferred by release-closeout scope. |

---

## Release Channel Checklist

- [x] Release payload SHA and current `origin/main` SHA at refresh recorded.
- [x] Required gate state recorded from latest merged product PR, including the #1759 PR-fast caveat and current-main targeted re-smoke.
- [x] Coverage/test gate semantics recorded after #1482/#1549 and follow-ups.
- [x] Parser, LSP, DAP, and easy-path smoke receipts recorded.
- [x] Bounded CPAN receipt recorded.
- [x] Full CPAN caveat recorded.
- [x] Release-candidate VSIX package/install smoke receipt recorded.
- [x] Changelog PR #1419 refreshed from merged scope only and merged.
- [x] Deferred scope listed.
- [x] Explicit maintainer dispatch hold preserved.

---

## Explicit Non-Goal

This document does not authorize or initiate a release. No tag, no `crates.io` publish, no VS Code Marketplace upload, no Docker image, and no GitHub Release will be created until Steven explicitly approves and dispatches the release workflow.

---

Generated from current repo/GitHub state through release payload `03823e40e9bcdfc8cc8418306dce5f877affa3d9` and local smoke receipts on 2026-06-20. Claims above are limited to the commands and receipts named in this file. Verify the current `origin/main` SHA again before dispatch.
