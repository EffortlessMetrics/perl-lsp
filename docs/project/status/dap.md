# DAP Debugger Status

> Generated metrics are marked with `<!-- BEGIN: DAP_... -->` / `<!-- END: DAP_... -->` markers.
> Run `cargo xtask update-status --only dap` to refresh them.
> Narrative sections below the marker blocks are human-maintained.

## Launch Success Rate

<!-- BEGIN: DAP_LAUNCH_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Launch success rate | 5/5 (100 %) | ≥ 80 % | PASS |
| Fixtures tested | hello, loops, eval, args, begin_end | 5 | — |
| cold_launch_p50 | 15 ms | ≤ 2 000 ms | PASS |
| cold_launch_p95 | 19 ms | ≤ 5 000 ms | PASS |
<!-- END: DAP_LAUNCH_SCORECARD -->

## Session Correctness & Attach

<!-- BEGIN: DAP_SESSION_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Attach success rate (TCP loopback) | 3/3 (100 %) | ≥ 80 % | PASS |
| Variables pane correctness (real session) | globals scope returned 1 named variables | expected named variables in scope | PASS |
| Evaluate correctness (real session) | evaluate($x + 1) returns 42 | evaluate($x + 1) => 42 | PASS |
| Deep truncation/pagination correctness | no indexedVariables >= 200 found in this real-session scope | page [250..274] over @big | SKIP |
| Memory footprint baseline (portable proxy) | debug_adapter_struct_size=176 bytes; best_effort_process_rss=10924 KiB | best-effort baseline | MEASURED |
<!-- END: DAP_SESSION_SCORECARD -->

## Test Coverage

<!-- BEGIN: DAP_TEST_COUNTS -->
| Suite | Count |
|---|---|
| Integration tests (`perl-dap`) | 58 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
