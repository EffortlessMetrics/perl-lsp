# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `provider_references_precision` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_references_precision` |
| `provider_references_recall` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_references_recall` |
| `provider_rename_safe_edit_accuracy` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_rename_safe_edit_accuracy` |
| `provider_safe_delete_blocker_accuracy` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_safe_delete_blocker_accuracy` |
| `completion_query_ms_p95` | provider query timing is not wired yet | `feat(metrics): wire parser-accuracy timing for completion_query_ms_p95` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
