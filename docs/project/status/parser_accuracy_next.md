# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `definition_query_ms_p95` | provider query timing is not wired yet | `feat(metrics): wire parser-accuracy timing for definition_query_ms_p95` |
| `recovery_ms_p95` | recovery timing is not instrumented separately yet | `feat(metrics): wire parser-accuracy timing for recovery_ms_p95` |
| `reference_query_ms_p95` | provider query timing is not wired yet | `feat(metrics): wire parser-accuracy timing for reference_query_ms_p95` |
| `workspace_insert_ms_p95` | workspace insert timing is not isolated yet | `feat(metrics): wire parser-accuracy timing for workspace_insert_ms_p95` |
| `allocated_bytes` | allocation telemetry is not wired yet | `feat(metrics): wire parser-accuracy measurement for allocated_bytes` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
