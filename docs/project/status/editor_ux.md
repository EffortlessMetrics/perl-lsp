# Editor UX Scorecard

Measured: `2026-04-24T05:58:11.786689210+00:00`  
Scenarios: `5`

## Correctness

| Metric | Value |
|---|---:|
| completion_top1_pct | 100.00% |
| completion_top5_pct | 100.00% |
| cross_file_success_pct | 100.00% |
| definition_exact_hit_pct | 100.00% |
| hover_correctness_pct | 100.00% |
| symbol_correctness_pct | 100.00% |

## Latency (ms)

| Request class | p50 | p95 |
|---|---:|---:|
| completion | 27.00 | 35.00 |
| definition | 36.00 | 44.00 |
| hover | 24.00 | 31.00 |
| workspace_symbols | 58.00 | 70.00 |

## Ratchet policy

Regression-only ratchet: floor metrics may improve or stay flat; any statistically meaningful regression fails `cargo xtask ux-scorecard --ratchet-check`.
