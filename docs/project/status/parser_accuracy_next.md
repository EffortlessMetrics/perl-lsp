# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `ast_delimiter_pairing_accuracy` | no expected AST delimiter-pairing labels are available | `feat(metrics): wire parser-accuracy measurement for ast_delimiter_pairing_accuracy` |
| `comment_invariance_rate` | metamorphic comment fixtures are not wired yet | `feat(metrics): wire parser-accuracy measurement for comment_invariance_rate` |
| `dynamic_boundary_precision` | no dynamic-boundary fact predictions are available in fully labeled fixtures | `feat(metrics): wire parser-accuracy measurement for dynamic_boundary_precision` |
| `gold_added_expectation_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_added_expectation_count` |
| `gold_changed_line_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_changed_line_count` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
