# Release-Readiness Bundle - 2026-06 Campaign

**Status**: release-staged, maintainer-held. Do not tag, publish, or dispatch a release without Steven's explicit approval.
**Updated**: 2026-06-21
**Release payload SHA**: `dc604307420f9563b4f6dd240888e7cd2cfd81af`
**Current `origin/main` SHA at refresh**: `dc604307420f9563b4f6dd240888e7cd2cfd81af`
**Previous readiness bundle merge SHA**: `dc604307420f9563b4f6dd240888e7cd2cfd81af`
**Workspace version (`Cargo.toml`)**: `0.16.0`
**Last release tag**: `v0.16.0` at `b6d9f12b995ad8ad78ca641940bd73e4b1a3c26d` (2026-06-06)

Note: the release payload SHA is the latest audited merged main commit at this refresh. It includes the #1894 completion gate repair payload plus the post-#1894 main delta already captured by #1946, then the later closeout payload through #2046: #1948 readiness refresh through `4efcd895d81ec87bf8960d148fd40a11efe0e90b`; #1949 Dancer route completion pack; #1950/#2041 coverage for proof helpers; #1951/#1978 shared Perl toolchain profile and cached interpreter-version probing; #1957 Rust 1.95 CI pin alignment; #1845 lexical-sub declarator facts; #1899/#2016 workspace `.perltidyrc` discovery and native formatter consumption; #1838 LSP 3.17 `insertTextModes` capability advertisement; #1834 non-standard POD section extraction; #1928 pending ignored-anchor red test preservation; #2039 PR-fast capability snapshot repair; #1571 first-run doctor CLI setup report; #2045 agent-lease proof-control-plane coverage; and #2046 readiness refresh through the first-run doctor closeout. The current release-staging refresh branch must pass its merge-blocking checks before this refreshed bundle is treated as the current release-stage artifact.

Note: `v0.16.0` is a real tag, but it is not on current `origin/main` ancestry (`git describe origin/main` resolves from `v0.15.0`). Do not use a naive "commits since v0.16.0" count as release evidence without resolving that tag-lineage question.

---

## Release Verdict

| Area | Verdict | Evidence |
|------|---------|----------|
| P0 blockers | PASS | No open P0 surfaced by this closeout pass. |
| P1 blockers | PASS with caveats | The measured multi-root `workspace/symbol` P1 is closed by #1522 and the focused smoke is green. Fresh product/test required-gate aggregates are green through #2045, and the docs-only #2046 readiness refresh passed the required docs-scope gates on release payload `dc604307420f9563b4f6dd240888e7cd2cfd81af`. Full CPAN-scale parser accuracy remains caveated because the full ratchet is still runner-dark. |
| Release dispatch | HELD | This bundle stages the release decision only. No tag, publish, marketplace upload, crates.io publish, Docker image, or GitHub Release is authorized here. |

---

## Gate State

Latest audited `origin/main` payload at refresh: [#2046](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2046), head commit `a1dd1740699bbe7af4cbcf3791ae83c5930882ba`, merge commit `dc604307420f9563b4f6dd240888e7cd2cfd81af`, merged 2026-06-21T00:44:43Z. The previous readiness bundle was [#2046](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2046), merge commit `dc604307420f9563b4f6dd240888e7cd2cfd81af`; it superseded [#1948](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1948), merge commit `0b5251d0a25b326b485782cd5242f05d3a275706`, merged 2026-06-20T19:31:48Z. Post-#1948 payload now included here covers the Dancer completion pack, shared and cached Perl toolchain profile, Rust 1.95 CI pin alignment, lexical-sub declarator facts, workspace `.perltidyrc` formatter discovery/consumption, LSP 3.17 `insertTextModes`, non-standard POD section extraction, inherited package-method completion, PIR v0 tooling IR, semantic-token fact-class cutover, first-run doctor reporting, and the #2038/#2039/#2041/#2045 gate-receipt repairs.

Post-#1894 merged payload included in this bundle: [#1890](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1890), [#1891](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1891), [#1893](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1893), [#1895](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1895), [#1896](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1896), [#1898](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1898), [#1925](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1925), [#1932](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1932), [#1938](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1938), [#1926](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1926), [#1927](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1927), [#1939](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1939), [#1945](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1945), [#1946](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1946), [#1948](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1948), [#1949](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1949), [#1950](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1950), [#1951](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1951), [#1957](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1957), [#1845](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1845), [#1899](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1899), [#1900](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1900), [#1907](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1907), [#1909](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1909), [#1920](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1920), [#1928](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1928), [#1978](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1978), [#2016](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2016), [#2038](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2038), [#2039](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2039), [#2041](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2041), [#1838](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1838), [#1841](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1841), [#1834](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1834), [#1571](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1571), [#2045](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2045), and [#2046](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/2046). Release claims below remain limited to the receipts in this file.

The earlier #1759 stale-branch PR-fast caveat is superseded for release-gate purposes by #1894's fresh `PR Smoke (Fast Feedback)` and `CI Gate (Merge-Blocking)` passes on payload `89a1225e48852583f6662da43257dae6a651a96b`, plus fresh #1926, #1939, #1945, #1946, #2039, #2041, #1571, and #2045 required-gate passes on the current closeout payload. #2046 was a docs-only readiness refresh: its product lanes were intentionally skipped by the docs/assets guard while coverage, workflow-trigger, routed Rust Small, RIPR, and advisory gates passed. The targeted prior-current DAP stack-trace receipt remains listed below as supporting evidence for the original stale-branch diagnosis. This release-staging branch is the required branch-gate receipt for refreshing this bundle through `dc604307420f9563b4f6dd240888e7cd2cfd81af`.

| Required / decision gate | Status | Notes |
|--------------------------|-----------------|-------|
| `Perl LSP Rust Small Result` | PASS | #2046 docs-scope aggregate completed 2026-06-21T00:43:29Z after CX53 Rust Small passed; #2045 product/test payload aggregate completed 2026-06-21T00:05:23Z after the disk-full fallback succeeded. |
| `ripr+ New Gap Gate` | PASS | #2046 docs-scope aggregate completed 2026-06-21T00:41:10Z after CX53 `ripr+` passed; #2045 product/test payload aggregate completed 2026-06-21T00:21:05Z. |
| `Codecov / Patch 95` | PASS | #2046 GitHub check completed 2026-06-21T00:22:19Z; #2045 Patch-95 completed 2026-06-21T00:23:41Z and Codecov patch completed 2026-06-21T00:24:15Z. |
| `Workflow Trigger Lint` | PASS | #2046 GitHub check completed 2026-06-21T00:26:54Z; #2045 completed 2026-06-21T00:08:33Z. |
| `PR Smoke (Fast Feedback)` | PASS with docs-only skip caveat | #2045 product/test payload completed 2026-06-21T00:21:34Z. #2046 skipped by docs-only guard, which is expected for a readiness-doc refresh. |
| `CI Gate (Merge-Blocking)` | PASS with docs-only skip caveat | #2045 aggregate completed 2026-06-21T00:21:41Z after all required shards passed. #2046 skipped by docs-only guard, which is expected for a readiness-doc refresh. |
| `UX Regression Tests` | PASS with docs-only skip caveat | #2045 UX lane completed 2026-06-21T00:11:50Z. #2046 skipped by docs-only guard, which is expected for a readiness-doc refresh. |
| `droid-review` | PASS | #2046 advisory check completed 2026-06-21T00:31:12Z; #2045 completed 2026-06-21T00:15:14Z. |
| `LSP Memory Smoke` | PASS with docs-only skip caveat | #2045 memory-smoke lane completed 2026-06-21T00:09:35Z. #2046 skipped by docs-only guard, which is expected for a readiness-doc refresh. |
| `UB Review Advisory on GitHub Hosted` | PASS | #2046 advisory check completed 2026-06-21T00:43:49Z; #2045 completed 2026-06-21T00:27:27Z. Advisory proof, not product-smoke proof. |

Advisory/non-required state observed during the #2045/#2046 closeout refresh: #2045 initially failed `Perl LSP Rust Small on CX43` because the disk preflight routed to fallback, and the aggregate passed after the disk-full fallback succeeded. #2046 skipped product lanes under the docs/assets-only guard while required trigger, coverage, routed Rust Small, RIPR, and advisory checks passed. Required product/test merge gates were green on #2045; required docs-scope merge gates were green on release payload `dc604307420f9563b4f6dd240888e7cd2cfd81af`. This branch carries the current refreshed-bundle gate before merge.

Coverage semantics after #1482/#1549/#1576/#1581/#1586: coverage verdicts are scoped to coverage shortfall/setup/routing failures. Routed test failures belong to test-named gates, not the Codecov/Patch-95 verdict.

Runner semantics after #1528: self-hosted disk preflight can fail over to GitHub-hosted only for disk-preflight failures. Real test or quality-gate failures are not masked by fallback.

Required workflow trigger semantics after #1816/#1817: required CI workflows stay trigger-visible for PR events. Docs/assets-only PRs skip expensive Rust jobs inside the workflow guard rather than by `paths-ignore`, so Workflow Trigger Lint remains enforced.

---

## Shipping Scope Since The Prior Draft

This table records the late closeout scope that updates the older 2026-06 draft. The changelog PR [#1419](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1419) is merged and carries the user-facing release-note wording.

| Theme | Merged PRs | Release claim |
|-------|------------|---------------|
| Parser and quick-fix robustness | #1457, #1460, #1461, #1463, #1481, #1483, #1893, #1913, #1845 | More valid Perl and malformed mid-edit input stays quiet; `given` blocks accept postfix `when`/`default` modifiers and ordinary statements; UTF-8 mid-codepoint code-action ranges no longer panic; duplicate quick-fix lightbulb entries from overlapping providers are collapsed; `my sub`/`our sub`/`state sub` declarations retain their declarator facts for downstream semantic analysis. |
| Single-root LSP readiness | #1551 | Baseline PR smoke blockers were repaired: hover preservation, workspace file-op/index waits, empty workspace-folder no-op handling, progress harness determinism. |
| Multi-root project model | #1522 | `workspace/symbol` waits for active indexing, includes `workspaceFolderUri`, and returns deterministic multi-root results. |
| Semantic identity and compiler substrate | #1876, #1904, #1891, #1902, #1900, #1895, #1896, #1599, #1910, #1920 | File-scoped semantic fact IDs include `FileId`, preventing identical source in different files from colliding in anchors, entities, occurrences, and file-scoped edges while preserving the file-neutral reference-source sentinel. `FileSemanticBundle` category hashes now include generated-member and eval-sub synthetic entities/anchors before hash computation, and shards carry an explicit producer schema version. AST child-classification flags now match traversal, HIR lowers control-flow shells, PIR v0 tooling IR is available from HIR, compile-state layers and semantic snapshot identity invariants have durable docs/proof, provider-decision schema alignment is restored, parser boundary-detection responsibilities are documented for consumers, and `our` declarations have scoped semantic-token fact classes. |
| Diagnostics accuracy | #1562 | `$self->{name}` / `$ref->{key}` no longer produce false `UnquotedBareword` diagnostics under `use strict`. |
| References, docs links, hover, and folding | #1597, #1638, #1560, #1795, #1882, #1840, #1834 | Partial-index reference fallback avoids documents-lock re-entry; perldoc/MetaCPAN targets share one validated resolver; POD `L<>` document links are exposed and resolved; non-standard `=head1` sections such as ARGUMENTS, RETURN VALUES, EXAMPLES, and SEE ALSO are extracted; cached POD hover content refreshes after external module-file edits; user-supplied hover documentation escapes markdown metacharacters; heredoc/multiline folding boundaries are corrected. |
| DAP honesty | #1430, #1444, #1496, #1498, #1806, #1810, #1759, #1811, #1807 | Variable-reference spaces are typed; evaluate and stack parsing report the real invalid input instead of stale or misleading state; request ordering now rejects out-of-sequence initialize/launch/configurationDone flows; scope and variable responses expose DAP pagination hint fields; capability flags match routed restart-frame, step-in-targets, and terminate-threads handlers. Logpoint message interpolation is available in the breakpoint-store substrate when callers supply variable maps; end-to-end process-layer variable extraction remains deferred. |
| Transport robustness | #1793 | `Content-Length` frame parsing uses checked arithmetic for `body_start` and recovers through the existing invalid-length path on overflow. |
| Deterministic completion value | #1532, #1573, #1579, #1585, #1875, #1839, #1889, #1926, #1945, #1949, #1838, #1841 | Try::Tiny, Mojolicious, Dancer, DBI, inherited package-qualified methods, and indexed package receiver completions are offered only with supporting evidence; context-specific completion families keep stable semantic sort tiers; quoted special-character hash keys are surfaced in hash-key completion; completion items serialize `filterText` and advertise LSP 3.17 insert-text modes; multiline inline-completion candidates are full-document parse-checked and fail closed when ranges cannot be reconstructed. Test::More and Test2::V0 assertion packs now have completion-pack contract fixtures for positive and quiet contexts. |
| Formatting and workspace configuration | #1899, #2016, #1951, #1978 | The server discovers project-local `.perltidyrc` settings when no explicit formatter profile is configured, feeds supported profile options into the native formatter, and resolves Perl interpreter identity through a shared cached toolchain profile. |
| Easy first-run extension path | #1867, #1898, #1571 | VSIX packaging/install smoke is storage-safe and restart-safe; VS Code now has bounded include-path discovery suggestions, optional server-gated AI completion discoverability, and a bundled demo project command; `perllsp --doctor [dir]` prints a read-only setup report with actionable Perl/config/include-root guidance. |
| Measurement and compatibility substrate | #1482, #1520, #1528, #1530, #1539, #1549, #1576, #1581, #1586, #1688, #1689, #1816, #1817, #1833, #1878, #1881, #1886, #1842, #1903, #1894, #1890, #1932, #1938, #1939, #1950, #1957, #1959, #2038, #2039, #2041, #2045 | Coverage/test semantics, CPAN bounded ratchet mode, runner disk failover, workflow privilege analysis, routed-suite expectations, draft-ripr neutrality, trigger-safe docs/assets-only CI skipping, Windows-worktree coverage-baseline recovery, native-tooling git-context recovery, RIPR evidence git-context recovery, self-hosted workspace-target cleanup, sigil-aware completion regression coverage, gold-fixture corpus hygiene, the #1839 fmt/Patch-95 gate repair, PR-summary coverage tests, Rust 1.95 toolchain documentation and CI pins, duplicate hash-key test-name repair, gate-list CLI contract coverage, active-manifest/allocation-tracker coverage receipts, agent-lease proof-control-plane coverage, PR-fast capability snapshot repair, and main fmt drift repair are current. |

Correction from earlier draft: #1524 is closed, not merged. The arrow-deref diagnostic fix landed as #1562.

---

## Product Smoke Receipts

All product-smoke commands below were run locally on Windows against `origin/main`/`c94d50e8` lineage during the 2026-06-20 closeout pass unless noted. Later main commits through release payload `dc604307420f9563b4f6dd240888e7cd2cfd81af` include docs/CI release-staging changes, the targeted POD document-link expansion in #1795, the storage-safe VSIX packaging/reinstall-smoke closeout in #1867, the native-tooling git-context repair in #1878, the context-specific completion sort-tier repair in #1875, the RIPR evidence git-context repair in #1881, the self-hosted workspace-target cleanup in #1886, the POD hover cache refresh in #1882, the file-scoped semantic ID repair in #1876, DAP request-order/scope/capability fixes in #1806/#1810/#1759, the LSP transport checked-arithmetic repair in #1793, the completion/hover/DAP/workspace/corpus closeout merges #1839/#1840/#1889/#1811/#1807/#1903/#1904, the #1894 completion gate repair, post-#1894 PR-level receipts through #1948, and later #1949/#1950/#1951/#1957/#1845/#1899/#1900/#1907/#1909/#1920/#1928/#1978/#2016/#2038/#2039/#2041/#1838/#1841/#1834/#1571/#2045/#2046 closeout payload. Cargo target output was redirected outside the worktree under `D:\cargo-target\perl-lsp-release-smoke*`, the repo safe target wrapper's external cache, or the specific external target dir named in the command.

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
| Corpus gold-fixture syntax hygiene | `perl -c` on `test_corpus/low_frequency_nodekinds.pl` and `test_corpus/statement_modifier_production_enhanced.pl`; parser check with 0 error nodes / 0 missing nodes | PASS in #1903 verification; two invalid-Perl fixture bugs no longer count as parser false negatives |
| `given` block statement recovery | `cargo test -p perl-parser-core --test fix_when_modifier_in_given_1356 --locked` | PASS in #1893 verification; 4 regression cases cover postfix `when` modifiers, repeated modifiers, plain statements mixed with block arms, and classic `when`/`default` forms |
| HIR control-flow substrate | `cargo test -p perl-parser-core`; `cargo test -p xtask --bins`; `cargo test -p perl-workspace --lib semantic::visibility` | PASS in #1902 verification; control-flow HIR shells lower without provider cutover |

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
| Quoted special-character hash-key completion | `./scripts/cargo-safe llvm-cov test --no-report -p perl-lsp-rs-core --lib --profile agent --locked completion::completion -- --nocapture` | PASS locally on #1894; 297 completion tests passed, including single- and double-quoted special hash-key cases |
| Completion `filterText` serialization | `cargo test -p perl-lsp-rs --lib --profile agent --locked filter_text -- --nocapture`; `cargo test -p perl-lsp-rs --lib --profile agent --locked completion_item_serializer -- --nocapture`; `cargo test -p perl-lsp-rs --test lsp_completion_tests --profile agent --locked test_snippet_completion_includes_filter_text -- --nocapture` | PASS in #1889 verification; completion response paths serialize `filterText` when set |
| Multiline inline-completion parse safety | `cargo test -p perl-lsp-rs-core --profile agent --locked parse_safety`; `cargo test -p perl-lsp-rs-core --test inline_completion_ux_fixtures --profile agent --locked inline_completion_fixture_corpus_defines_completion_pack_contract`; `cargo check --all-targets -p perl-lsp-rs-core --profile agent --locked` | PASS in #1926 verification; full-document parse probes cover multiline replacement ranges and fail closed when candidate ranges cannot be reconstructed |
| Hover markdown escaping | Hover utility and integration tests for markdown metacharacter escaping | PASS in #1840 verification; hover docs with `*`, `_`, `#`, brackets, backticks, and related metacharacters render literally |
| FileSemanticBundle synthetic fact hashing | `cargo test -p perl-workspace file_semantic_bundle_tests --profile agent --locked` | PASS in #1904 verification; synthetic generated-member/eval-sub entities and anchors participate in category hashes before shard construction completes |
| Duplicate code-action collapse | `cargo test -p perl-lsp-rs` code-action suites; `cargo clippy -p perl-lsp-rs --lib`; `cargo fmt` | PASS in #1913 verification; 126 lib + 53 integration code-action tests passed and duplicate lightbulb snapshots were reduced without dropping distinct actions |
| Completion capability snapshots | `cargo test -p perl-lsp-rs --locked --test lsp_cap_snap --profile agent`; `cargo test -p perl-lsp-rs --locked --test lsp_capabilities_snapshot --profile agent` | PASS in #2039 verification; 9 `lsp_cap_snap` tests and 5 JSON capability snapshot tests passed after `insertTextModes` snapshots were regenerated. |

### DAP

| Surface | Command | Result |
|---------|---------|--------|
| DAP edge cases | `cargo test -p perl-dap --test dap_edge_cases_test test_dap --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 12 passed |
| Evaluate matrix | `cargo test -p perl-dap --test dap_evaluate_comprehensive_tests test_evaluate --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 42 passed |
| Eval reference cache/stale resume | `cargo test -p perl-dap --test eval_ref_cache_miss_resume_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 10 passed |
| Pause signal delivery | `cargo test -p perl-dap --test pause_signal_delivery_tests --profile agent --locked -- --test-threads=1 --nocapture` | PASS, 2 passed |
| DAP request-order validation | `cargo test -p perl-dap --lib test_launch_before_initialize_should_fail --profile agent --locked`; `cargo test -p perl-dap --lib test_configuration_done_without_launch_should_fail --profile agent --locked` | PASS in #1806 verification; out-of-sequence initialize/launch/configurationDone flows return explicit failures |
| DAP scope pagination fields | `cargo test -p perl-dap --test dap_coverage_audit_tests test_scope_includes_pagination_hints --profile agent --locked` | PASS in #1810 verification; `namedVariables` and `indexedVariables` round-trip and omit when unset |
| DAP variables pagination counts | `cargo test -p perl-dap --test dap_variables_pagination_tests --profile agent --locked` | PASS in #1811 verification; `totalVariables` is present when a count is known and omitted when unavailable |
| DAP capability advertising | `cargo test -p perl-dap --test dap_capability_advertising_tests --profile agent --locked` | PASS in #1759 verification; restart-frame, step-in-targets, and terminate-threads capabilities are advertised when handlers exist |
| DAP logpoint interpolation substrate | `cargo test -p perl-dap breakpoints:: --lib` | PASS in #1807 verification; breakpoint-store interpolation handles supplied variable maps, preserves missing/malformed expressions, and leaves raw behavior unchanged without variables |
| Prior stale-branch stackTrace pagination diagnosis | `./scripts/cargo-safe test -p perl-dap --test stack_trace_provider_tests test_total_frames_is_not_window_size --profile agent --locked -- --nocapture` | PASS on then-current main `03823e40e9bcdfc8cc8418306dce5f877affa3d9`; 1 passed. Fresh #1894 PR Smoke and CI Gate later passed on payload `89a1225e48852583f6662da43257dae6a651a96b`. |

### Easy / First-Run

| Surface | Command | Result |
|---------|---------|--------|
| CLI health/check smoke | `cargo test -p perl-lsp-rs --test cli_smoke --profile agent --locked -- --nocapture` | PASS in #1571 verification, 19 passed |
| First-run doctor CLI report | `cargo test -p perl-lsp-rs doctor --profile agent --locked`; `cargo test -p perl-lsp-rs-core parse_doctor --profile agent --locked` | PASS in #1571 verification; 45 doctor tests and 4 launcher/parse tests covered config, Perl probe stderr/guidance, include roots, `PERL5LIB`, system `@INC`, rejected roots, and `--doctor --check` conflict handling. |
| VS Code extension unit smoke | `npm ci`; `npm test -- --runInBand` in `vscode-extension` | PASS, 22 suites / 597 tests |
| VS Code first-run onboarding helpers | `npx jest`; `npm run compile`; `npm run lint` in `vscode-extension` | PASS in #1898 verification; 613 tests / 22 suites cover include-path suggestions, AI completion discoverability gating, walkthrough updates, and the bundled demo-project command |
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
| DAP logpoint process-layer variable extraction | Deferred | #1807 adds interpolation substrate and tests, but end-to-end logpoint substitution still requires the debug adapter/process layer to pass frame variable maps. No release claim is made for full editor-visible logpoint substitution. |
| Broad feature waves / dependabot | Open queue | Deferred by release-closeout scope. |

---

## Release Channel Checklist

- [x] Release payload SHA and current `origin/main` SHA at refresh recorded.
- [x] Required gate state recorded from latest merged release-scope PRs, including fresh #2045 product, CI, coverage, routed Rust Small, RIPR, UX, LSP Memory, and advisory-review passes plus #2046 docs-only required-gate passes/skips.
- [x] Coverage/test gate semantics recorded after #1482/#1549 and follow-ups.
- [x] Parser, LSP, DAP, and easy-path smoke receipts recorded.
- [x] Bounded CPAN receipt recorded.
- [x] Full CPAN caveat recorded.
- [x] Release-candidate VSIX package/install smoke receipt recorded.
- [x] Changelog refreshed from merged scope only through payload `dc604307420f9563b4f6dd240888e7cd2cfd81af`.
- [x] Deferred scope listed.
- [x] Explicit maintainer dispatch hold preserved.

---

## Explicit Non-Goal

This document does not authorize or initiate a release. No tag, no `crates.io` publish, no VS Code Marketplace upload, no Docker image, and no GitHub Release will be created until Steven explicitly approves and dispatches the release workflow.

---

Generated from current repo/GitHub state through release payload `dc604307420f9563b4f6dd240888e7cd2cfd81af` and local smoke receipts on 2026-06-20 plus #2045/#2046 GitHub gate receipts on 2026-06-21. Claims above are limited to the commands and receipts named in this file. Verify the current `origin/main` SHA again before dispatch.
