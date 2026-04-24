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
| cold_launch_p95 | 18 ms | ≤ 5 000 ms | PASS |
<!-- END: DAP_LAUNCH_SCORECARD -->

## Test Coverage

<!-- BEGIN: DAP_TEST_COUNTS -->
| Suite | Count |
|---|---|
| Integration tests (`perl-dap`) | 58 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## Session Quality Scorecard

<!-- BEGIN: DAP_SESSION_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Attach success rate | 2/2 (100 %) | 100 % (2/2) | PASS |
| Variables pane correctness (session) | FAIL | Locals include computed vars | FAIL |
| Evaluate correctness (session) | PASS | evaluate($sum) includes 30 | PASS |
| Deep truncation/pagination correctness | FAIL | Distinct pages (start=0 vs 200, count=5) | FAIL |
| Memory footprint baseline (best effort) | rss_before=10948 KiB, rss_after=10972 KiB, delta=24 KiB | Informational baseline | MEASURED |
<!-- END: DAP_SESSION_SCORECARD -->

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
