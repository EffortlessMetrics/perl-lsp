# DAP Debugger Status

> Generated metrics are marked with `<!-- BEGIN: DAP_... -->` / `<!-- END: DAP_... -->` markers.
> Run `cargo xtask update-status --only dap` to refresh them.
> Narrative sections below the marker blocks are human-maintained.

## Runtime Scorecard

<!-- BEGIN: DAP_LAUNCH_SCORECARD -->
| Metric | Value | Target | Status |
|---|---|---|---|
| Launch success rate | 5/5 (100 %) | ≥ 80 % | PASS |
| cold_launch_p50 | measured (harness output) | ≤ 2 000 ms | PASS |
| cold_launch_p95 | measured (harness output) | ≤ 5 000 ms | PASS |
| Attach success rate | 5/5 (100 %) | ≥ 80 % | PASS |
| Variables pane correctness (session) | SKIP (best-effort probe inconclusive) | best effort | SKIP |
| Evaluate correctness (session) | PASS | PASS | PASS |
| Deep truncation/pagination correctness | SKIP (best-effort probe inconclusive) | best effort | SKIP |
| Memory footprint baseline proxy | best-effort sample captured | observability baseline | INFO |
<!-- END: DAP_LAUNCH_SCORECARD -->

## Test Coverage

<!-- BEGIN: DAP_TEST_COUNTS -->
| Suite | Count |
|---|---|
| Integration tests (`perl-dap`) | 58 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## Notes

- `attach_success_rate` is measured via process-id attach mode (`attach` with `processId`).
- Memory footprint is intentionally a best-effort proxy and may report `SKIP` on platforms where portable RSS is unavailable.

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).* 
