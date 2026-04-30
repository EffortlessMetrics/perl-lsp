# Editor UX Scorecard

Measured: `2026-04-26T01:27:25.397713670+00:00`  
Scenarios measured: `9` of `22` declared (41% fixture coverage)  

**Measured scenarios:** hover_core, goto_definition_core, multi_root_workspace_symbols, folder_removal_eviction, completion_ranking, strict_diagnostics, document_symbols_core, diagnostics_after_edit, rename_workflow_core  

## Correctness

| Metric | Value |
|---|---:|
| completion_top1_pct | 100.00% |
| completion_top5_pct | 100.00% |
| cross_file_success_pct | 100.00% |
| definition_exact_hit_pct | 100.00% |
| diagnostics_correct_pct | 100.00% |
| hover_correctness_pct | 100.00% |
| rename_success_pct | 100.00% |
| symbol_correctness_pct | 100.00% |

## Latency (ms)

| Request class | p50 | p50 baseline | p95 | p95 baseline |
|---|---:|---:|---:|---:|
| completion | 27.00 | 27.00 | 35.00 | 35.00 |
| definition | 36.00 | 36.00 | 44.00 | 44.00 |
| diagnostics | 53.00 | 53.00 | 66.00 | 66.00 |
| document_symbols | 20.00 | 20.00 | 28.00 | 28.00 |
| hover | 24.00 | 24.00 | 31.00 | 31.00 |
| workspace_symbols | 58.00 | 58.00 | 70.00 | 70.00 |

## Ratchet policy

Regression-only ratchet: floor metrics may improve or stay flat; any statistically meaningful regression fails `cargo xtask ux-scorecard --ratchet-check`.
