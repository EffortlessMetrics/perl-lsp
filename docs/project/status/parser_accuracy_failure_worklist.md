# Parser-accuracy failure worklist

Source: `target/metrics/parser_accuracy.json` (6 failure packets)

| Family | Count | Likely layer | First fixture | Suggested PR |
|---|---:|---|---|---|
| line_tag_mismatch | 5 | parser | `malformed_heredoc_recovery` | `fix(parser-core): resolve parser projection failure packet` |
| missing_symbol_declaration | 1 | semantic_fact_extraction | `generated_accessor` | `fix(semantic): resolve parser-accuracy semantic fact packet` |
