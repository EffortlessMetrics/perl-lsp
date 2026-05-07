# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `dynamic_boundary_precision` | no dynamic-boundary fact predictions are available in fully labeled fixtures | `feat(metrics): wire parser-accuracy measurement for dynamic_boundary_precision` |
| `gold_changed_symbol_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_changed_symbol_count` |
| `gold_dynamic_expectation_change_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_dynamic_expectation_change_count` |
| `gold_removed_expectation_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_removed_expectation_count` |
| `gold_weakening_explanation_required_count` | gold weakening explanation checks require a baseline diff in CI | `feat(metrics): wire parser-accuracy measurement for gold_weakening_explanation_required_count` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
