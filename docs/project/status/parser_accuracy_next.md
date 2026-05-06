# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `provider_completion_import_visibility_accuracy` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_completion_import_visibility_accuracy` |
| `provider_completion_visible_symbol_relevance` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_completion_visible_symbol_relevance` |
| `provider_diagnostic_false_negative_rate` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_diagnostic_false_negative_rate` |
| `provider_diagnostic_false_positive_rate` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_diagnostic_false_positive_rate` |
| `provider_document_symbol_precision` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_document_symbol_precision` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
