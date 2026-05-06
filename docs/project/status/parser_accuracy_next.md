# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `provider_document_symbol_recall` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_document_symbol_recall` |
| `provider_goto_definition_hit_rate` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_goto_definition_hit_rate` |
| `provider_hover_symbol_origin_accuracy` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_hover_symbol_origin_accuracy` |
| `provider_references_precision` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_references_precision` |
| `provider_references_recall` | provider gold fixtures are not wired yet | `test(parser-accuracy): wire provider gold fixture for provider_references_recall` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
