# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 49 fixtures / 29 families; 135 scored lines; 112 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| `symbol_ref_dynamic_boundary_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_ref_dynamic_boundary_f1` |
| `symbol_ref_export_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_ref_export_f1` |
| `symbol_ref_import_f1` | no symbol gold labels are available for this kind | `feat(metrics): wire parser-accuracy measurement for symbol_ref_import_f1` |
| `whitespace_invariance_rate` | metamorphic whitespace fixtures are not wired yet | `feat(metrics): wire parser-accuracy measurement for whitespace_invariance_rate` |

Use this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.
