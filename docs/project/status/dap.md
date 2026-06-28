# DAP Debugger Status

> Generated metrics are marked with `<!-- BEGIN: DAP_... -->` / `<!-- END: DAP_... -->` markers.
> Run `cargo xtask update-status --only dap` to refresh them.
> Narrative sections below the marker blocks are human-maintained.

## Launch Success Rate

<!-- BEGIN: DAP_LAUNCH_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Launch success rate | receipt missing (`cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture`) | ≥ 80 % | SKIP |
| Fixtures tested | hello, loops, eval, args, begin_end | 5 | — |
| cold_launch_p50 | — | ≤ 2 000 ms | SKIP |
| cold_launch_p95 | — | ≤ 5 000 ms | SKIP |
<!-- END: DAP_LAUNCH_SCORECARD -->

## Session Correctness & Attach

<!-- BEGIN: DAP_SESSION_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Attach success rate (TCP loopback) | receipt missing (`cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture`) | ≥ 80 % | SKIP |
| Variables pane correctness (real session) | receipt missing | expected named variables in scope | SKIP |
| Evaluate correctness (real session) | receipt missing | evaluate($x + 1) => 42 | SKIP |
| Deep truncation/pagination correctness | receipt missing | page [250..274] over @big | SKIP |
| Memory footprint baseline (portable proxy) | receipt missing | best-effort baseline | SKIP |
<!-- END: DAP_SESSION_SCORECARD -->

## Test Coverage

<!-- BEGIN: DAP_TEST_COUNTS -->
| Suite | Count |
|---|---|
| Integration tests (`perl-dap`) | 62 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## Distribution readiness

Receipts that downstream consumers (VS Code / Open VSX, LSP4IJ) depend on. Each
row links to its source of truth so the claim can be re-verified.

| Check | Status | Evidence |
|---|---|---|
| VS Code debugger contribution | PASS | `vscode-extension/package.json` contributes `perl` debugger, breakpoints, and launch/attach schemas |
| Managed `perl-dap` discovery | PASS | extension downloader resolves managed `perl-dap` before `PATH` |
| Launch honors both `perl` and `perlPath` | PASS | `resolve_launch_interpreter` (`crates/perl-dap/src/debug_adapter/process.rs`) + tests |
| Release archives contain `perl-dap` | PASS | built/packaged in `.github/workflows/release.yml`; guarded by `cargo xtask release artifact-check` |
| Native path avoids `Perl::LanguageServer` dependency | PASS | native launch/attach use the in-binary Rust runtime + local Perl only |
| Bridge path documents `Perl::LanguageServer` requirement | PASS | `crates/perl-dap/README.md` "External dependencies" |
| Downstream artifact contract documented | PASS | `docs/reference/DOWNSTREAM_DAP_INTEGRATIONS.md` |

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
