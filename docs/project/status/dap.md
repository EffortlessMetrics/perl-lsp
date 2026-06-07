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

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
