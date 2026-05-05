# Parser-accuracy failure worklist

Source: `target/metrics/parser_accuracy.json` (12 failure packets)

| Family | Count | Likely layer | First fixture | Suggested PR |
|---|---:|---|---|---|
| ast_shape_mismatch | 6 | ast_projection | `package_basic` | `feat(parser-accuracy): tighten AST projection fixture expectations` |
| line_tag_mismatch | 5 | parser | `malformed_heredoc_recovery` | `fix(parser-core): resolve parser projection failure packet` |
| missing_symbol_declaration | 1 | semantic_fact_extraction | `generated_accessor` | `fix(semantic): resolve parser-accuracy semantic fact packet` |
