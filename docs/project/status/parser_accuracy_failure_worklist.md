# Parser-accuracy failure worklist

Source: `target/metrics/parser_accuracy.json` (1 failure packets)

| Family | Count | Likely layer | First fixture | Suggested PR |
|---|---:|---|---|---|
| missing_symbol_declaration | 1 | semantic_fact_extraction | `generated_accessor` | `fix(semantic): resolve parser-accuracy semantic fact packet` |
