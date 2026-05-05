# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 46 fixtures / 28 families; 123 scored lines; 102 scored symbols.

Failure packets: 6 active.

| Field | Value |
|---|---|
| Pointer | `line_tag_mismatch` |
| Packet count | 5 |
| Likely layer | `parser` |
| First fixture | `malformed_heredoc_recovery` |
| Suggested PR | `fix(parser-core): resolve parser projection failure packet` |

Use this pointer only after open measurement/tracking PRs are settled. If the pointed lane has already landed, regenerate this file and take the next row.
