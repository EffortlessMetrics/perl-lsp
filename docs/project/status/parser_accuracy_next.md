# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 129 scored lines; 103 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `gold_dynamic_expectation_change_count` | gold drift baseline is not wired yet | `feat(metrics): wire parser-accuracy measurement for gold_dynamic_expectation_change_count` |
| `gold_weakening_explanation_required_count` | gold weakening explanation checks require a baseline diff in CI | `feat(metrics): wire parser-accuracy measurement for gold_weakening_explanation_required_count` |
| `heuristic_fact_precision` | no heuristic fact predictions are available in fully labeled fixtures | `feat(metrics): wire parser-accuracy measurement for heuristic_fact_precision` |
| `medium_confidence_precision` | no medium-confidence fact predictions are available in fully labeled fixtures | `feat(metrics): wire parser-accuracy measurement for medium_confidence_precision` |
| `newline_style_invariance_rate` | metamorphic newline fixtures are not wired yet | `feat(metrics): wire parser-accuracy measurement for newline_style_invariance_rate` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
