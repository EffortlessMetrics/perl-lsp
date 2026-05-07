# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `metric_cache_hit_rate` | metric cache telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for metric_cache_hit_rate` |
| `peak_rss_mb` | memory telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for peak_rss_mb` |
| `semantic_fact_cache_hit_rate` | semantic fact cache telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for semantic_fact_cache_hit_rate` |
| `unchanged_file_skip_rate` | unchanged-file skip telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for unchanged_file_skip_rate` |
| `workspace_shard_reuse_rate` | workspace shard reuse telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for workspace_shard_reuse_rate` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
