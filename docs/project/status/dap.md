# DAP Debugger Status

> Generated metrics are marked with `<!-- BEGIN: DAP_... -->` / `<!-- END: DAP_... -->` markers.
> Run `cargo xtask update-status --only dap` to refresh them.
> Narrative sections below the marker blocks are human-maintained.

## Launch Success Rate

<!-- BEGIN: DAP_LAUNCH_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Launch success rate | 5/5 (100 %) | ≥ 80 % | PASS |
| Fixtures tested | hello, loops, eval, args, begin\_end | 5 | — |
| cold\_launch\_p50 | 35 ms | ≤ 2 000 ms | PASS |
| cold\_launch\_p95 | 161 ms | ≤ 5 000 ms | PASS |
<!-- END: DAP_LAUNCH_SCORECARD -->

## Test Coverage

<!-- BEGIN: DAP_TEST_COUNTS -->
| Suite | Count |
|---|---|
| Integration tests (`perl-dap`) | 58 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## Runtime Scorecard (Real Session)

<!-- BEGIN: DAP_RUNTIME_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Attach success rate | 3/3 (100 %) | ≥ 80 % | PASS |
| Variables pane correctness (real session) | FAIL | PASS required | FAIL |
| Evaluate correctness (real session) | PASS | PASS required | PASS |
| Deep truncation/pagination correctness | FAIL | PASS required | FAIL |
| Memory footprint baseline (best-effort) | 5644 KiB -> 11016 KiB (delta +5372 KiB) | baseline-only | BASELINE |
<!-- END: DAP_RUNTIME_SCORECARD -->

Current harness behavior:
- Always measures launch and attach success rates.
- Measures variables/evaluate/pagination in a real `perl -d` session and reports PASS/FAIL without hard-failing the harness yet.
- Emits a Linux `VmRSS` best-effort baseline from `/proc/self/status`; non-Linux platforms report SKIP.

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
