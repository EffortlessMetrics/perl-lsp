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
| Integration tests (`perl-dap`) | 21 test targets |
| Scorecard fixtures | 5 |
<!-- END: DAP_TEST_COUNTS -->

## Deferred Metrics

The following metrics are defined in #4069 but deferred to follow-up PRs:

| Metric | Issue | Reason |
|---|---|---|
| Attach success rate | #4069 | Requires live Perl process on TCP socket — deferred post-alpha |
| Variables pane correctness | #3487 | Needs integration test with real `perl -d` variables |
| Evaluate correctness (session) | #3481 | Existing mocked tests; real-session E2E deferred |
| Truncation/pagination on deep data | #3487 | No test with 200+ array elements yet |
| Memory footprint baseline | #4069 | OS-level RSS measurement non-trivial to make portable |

## How to Update

```bash
cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
cargo xtask update-status --only dap
```

---

*Last updated by builder agent (issue #4069, PR 1 of 3).*
