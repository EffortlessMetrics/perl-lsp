# Parser-accuracy failure worklist

Source: `target/metrics/parser_accuracy.json` (0 failure packets)

| Family | Count | Likely layer | First fixture | Suggested PR |
|---|---:|---|---|---|
| line_tag_mismatch | 0 | n/a | n/a | `fix(parser-accuracy): classify signature body function-call packet` |
missing_symbol_declaration | 0 | n/a | n/a | `fix(parser-accuracy): triage generated accessor symbol declaration packet` |
missing_ast_node | 0 | ast_projection | n/a | TODO |
missing_symbol_reference | 0 | semantic_fact_extraction | n/a | TODO |
missing_symbol_edge | 0 | semantic_fact_extraction | n/a | TODO |
span_mismatch | 0 | parser | n/a | TODO |
dynamic_false_precision | 0 | provider_query | n/a | TODO |
unsupported_construct | 0 | recovery | n/a | TODO |
gold/schema issue | 0 | gold_schema | n/a | TODO |

## line_tag_mismatch triage notes

| Packet | Fixture | Line | Classification | Suggested next action |
|---|---|---:|---|---|
<!-- No remaining line_tag_mismatch follow-ups. -->

## missing_symbol_declaration triage notes

| Packet | Fixture | Classification | Suggested next action |
|---|---|---|---|
| missing_symbol_declaration | `generated_accessor` | `resolved_in_parser_accuracy` | Tracked via `SemanticAnalyzer` generated-member extraction in parser-accuracy scoring; keep for runtime canonical-fact follow-up if `perl-workspace` remains gap |
