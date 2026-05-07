# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 48 fixtures / 28 families; 134 scored lines; 109 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `newline_style_invariance_rate` | metamorphic newline fixtures are not wired yet | `feat(metrics): wire parser-accuracy measurement for newline_style_invariance_rate` |
| `recovery_post_error_line_f1` | post-error line precision or recall denominator is unavailable | `feat(metrics): wire parser-accuracy measurement for recovery_post_error_line_f1` |
| `symbol_decl_global_variable_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_decl_global_variable_f1` |
| `symbol_decl_inherited_method_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_decl_inherited_method_f1` |
| `symbol_decl_lexical_variable_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_decl_lexical_variable_f1` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
