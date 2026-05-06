# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `workspace_insert_ms_p95` | workspace insert timing is not isolated yet | `feat(metrics): wire parser-accuracy timing for workspace_insert_ms_p95` |
| `allocated_bytes` | allocation telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for allocated_bytes` |
| `allocation_count` | allocation telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for allocation_count` |
| `content_hash_hit_rate` | content hash telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for content_hash_hit_rate` |
| `lexer_checkpoint_reuse_rate` | lexer checkpoint telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for lexer_checkpoint_reuse_rate` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
