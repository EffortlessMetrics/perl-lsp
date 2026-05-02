# Design Document: Parser Accuracy Observability

## Overview

Parser Accuracy Observability layers gold scoring on top of the existing parser scorecard. The current scorecard answers whether perl-lsp can ingest real Perl without erroring and whether recovery is useful. This design adds the next question: when parsing succeeds, did it produce the right line tags, AST shape, symbols, spans, dynamic-boundary behavior, and editor-facing facts?

The system is intentionally layered:

1. Denominator inventory
2. Clean parse and recovery
3. Line-level construct accuracy
4. AST structural accuracy
5. Symbol and edge accuracy
6. False-positive and false-precision tracking
7. Span and coordinate correctness
8. Incremental equivalence
9. Confidence, unsupported constructs, and dynamic boundaries
10. Provider impact
11. Real-project partial labels
12. Cost, scale, cache, determinism, failure attribution, gold drift, and metric runtime

Do not collapse these into one health number. The scorecard must show separate rows with sample counts, confidence, and metric state.

## Data Flow

```
gold fixtures / partial real-project labels
  |
  v
parser accuracy scorer
  |
  +--> target/metrics/parser_accuracy.json
  |
  +--> target/receipts/parser-accuracy/*.json
  |
  v
schema validation
  |
  v
xtask update-status --only parser
  |
  v
docs/project/status/parser.md
```

Committed contracts live in:

```
.ci/schemas/parser-accuracy.schema.json
.ci/metrics/baselines/parser_accuracy.json
docs/project/metrics/parser.md
.kiro/specs/parser-accuracy-observability/
```

Generated artifacts live in:

```
target/metrics/parser_accuracy.json
target/receipts/parser-accuracy/*.json
```

## Artifact Shape

The JSON scorecard uses one top-level artifact:

```jsonc
{
  "schema_version": 1,
  "subsystem": "parser_accuracy",
  "generated_at": "2026-05-02T00:00:00Z",
  "commit": "<sha>",
  "cadence": "pr|merge_gate|nightly|release",
  "denominator": { "...": "..." },
  "families": [],
  "metrics": [],
  "failure_packets": [],
  "gold_drift": {},
  "metric_runtime": {}
}
```

Each metric row has:

```jsonc
{
  "metric": "symbol_ref_f1",
  "state": "measured",
  "value": 0.837,
  "previous": 0.821,
  "delta": 0.016,
  "floor": 0.75,
  "threshold": 0.85,
  "direction": "up",
  "sample_count": 428,
  "confidence": "medium",
  "cadence": "merge_gate",
  "macro_value": 0.801,
  "micro_value": 0.837
}
```

Insufficient data is represented as a metric state:

```jsonc
{
  "metric": "provider_goto_definition_hit_rate",
  "state": "insufficient_data",
  "reason": "provider gold fixtures are not wired yet",
  "sample_count": 0
}
```

## Denominator Model

The denominator block records what was scored before interpreting accuracy:

```jsonc
{
  "fixture_count": 120,
  "fixture_family_count": 28,
  "scored_line_count": 8400,
  "scored_symbol_count": 1250,
  "fully_labeled_region_count": 640,
  "partial_labeled_region_count": 210,
  "unknown_region_count": 88,
  "negative_region_count": 93,
  "dynamic_boundary_case_count": 82,
  "unsupported_construct_case_count": 37,
  "real_project_file_count": 43,
  "generated_fixture_count": 70,
  "hand_labeled_fixture_count": 50
}
```

Fixture families are explicit and stable. Required families include package declarations, subroutines, methods, lexical variables, globals, imports, exports, qualified references, bare references, typeglob aliases, AUTOLOAD, eval string, dynamic require, generated accessors, roles, inheritance, heredocs, regexes, quote-like operators, POD, format statements, Moose/Moo DSL, signatures/invocants, postderef, do-while/until, and given/when/default.

## Scoring Layers

### Clean Parse

Clean parse stays as a top-level parser score, but it is not treated as accuracy by itself.

Rows:

- `clean_parse_file_rate`
- `clean_parse_line_rate`
- `parse_error_density_per_kloc`
- `first_error_bucket`
- `files_with_recovery`
- `strict_clean_rate`
- `partial_clean_rate`

Failure clusters:

- `quote_transliteration`
- `heredoc_delimiter`
- `declaration_package`
- `recovery_only`
- `encoding_multibyte`
- `regex`
- `operator_precedence`
- `incremental_edit_application`

### Line-Level Construct Accuracy

Line scoring compares expected tag sets with observed tag sets:

```text
TP = expected intersect actual
FP = actual - expected
FN = expected - actual
precision = TP / (TP + FP)
recall = TP / (TP + FN)
f1 = 2PR / (P + R)
```

Rows:

- `line_exact_match_rate`
- `line_construct_precision`
- `line_construct_recall`
- `line_construct_f1`
- `line_error_false_positive_rate`
- `line_error_false_negative_rate`
- `line_dynamic_boundary_correct_rate`
- `unsupported_line_detection_rate`

### AST Structural Accuracy

AST scoring validates node kinds, spans, and relationships.

Rows:

- `node_kind_precision`
- `node_kind_recall`
- `node_kind_f1`
- `node_span_exact_rate`
- `node_span_near_rate`
- `parent_child_edge_accuracy`
- `tree_depth_accuracy`
- `operator_precedence_accuracy`
- `delimiter_pairing_accuracy`
- `unexpected_error_node_count`
- `missing_expected_node_count`

### Symbol and Edge Accuracy

Symbol scoring consumes canonical facts when available.

Rows:

- `symbol_decl_precision`
- `symbol_decl_recall`
- `symbol_decl_f1`
- `symbol_ref_precision`
- `symbol_ref_recall`
- `symbol_ref_f1`
- `definition_edge_precision`
- `definition_edge_recall`
- `definition_edge_f1`
- `reference_edge_precision`
- `reference_edge_recall`
- `reference_edge_f1`
- `span_exact_rate`
- `span_near_rate`
- `semantic_match_rate`

Breakdowns include package, subroutine, method, lexical variable, global variable, import, export, typeglob alias, generated accessor, role method, inherited method, and dynamic boundary.

### False Positives and False Precision

False positives are reported separately from misses. The hard safety floor is:

```text
dynamic_false_precision_count == 0
fast_path_wrong_result_count == 0
```

Rows include false symbols, false declarations, false references, false imports, false exports, false parse errors, false exact resolutions, false dynamic resolutions, and symbols emitted in comments, POD, strings, or unknown regions.

### Recovery Quality

Recovery scoring measures containment and salvage:

- `first_error_line_accuracy`
- `error_region_precision`
- `error_region_recall`
- `recovery_spillover_mean`
- `recovery_spillover_p95`
- `recovery_spillover_max`
- `salvaged_lines_after_error`
- `salvaged_symbols_after_error`
- `post_error_symbol_recall`
- `post_error_line_f1`

### Incremental Equivalence

Incremental scoring compares:

```text
full_parse(final_source)
==
incremental_parse(base_source + edit_sequence)
```

Rows:

- `incremental_full_parse_equivalence_rate`
- `incremental_edit_apply_equivalence_rate`
- `incremental_no_panic_rate`
- `incremental_no_progress_count`
- `incremental_timeout_count`
- `incremental_full_reparse_fallback_rate`
- `incremental_checkpoint_hit_rate`
- `incremental_checkpoint_miss_rate`
- `incremental_reparse_byte_ratio`
- `incremental_reused_token_ratio`
- `incremental_reused_node_ratio`
- `incremental_changed_range_accuracy`

The command that verifies these rows must enable the incremental feature.

### Span and Coordinate Correctness

Rows:

- `byte_span_exact_rate`
- `line_span_exact_rate`
- `utf16_range_exact_rate`
- `span_near_rate`
- `span_invalid_count`
- `span_out_of_bounds_count`
- `span_inverted_count`
- `span_non_char_boundary_count`
- `crlf_position_error_count`
- `unicode_position_error_count`
- `tab_column_mismatch_count`

### Cost, Scale, Cache, and Determinism

Cost rows track phase timings, memory, and allocation shape. Scale rows track file size, token count, AST node count, symbol count, nesting, regex length, heredoc size, quote-like count, and dynamic boundary count. Cache rows explain whether speed comes from reuse. Determinism rows track hash stability and metamorphic invariants.

Speed improvements must be interpreted with scale rows. A faster parser on tiny files only is not a general parser speed win.

## Cadence

| Cadence | Contents | Blocking |
| --- | --- | --- |
| PR fast | schema validation, denominator inventory, tiny line/AST/symbol fixture set, incremental smoke | yes |
| Merge gate | sharded line/AST/symbol/incremental/cost fixture set | yes after stable |
| Nightly | full gold corpus, real-project partial labels, macro/micro family breakdowns | report first, gate later |
| Release | full parser accuracy dashboard with trend and baseline comparison | release decision input |

## First Implementation Slice

The first PR after this spec should be intentionally small:

1. Add `.ci/schemas/parser-accuracy.schema.json`.
2. Add a minimal fixture manifest with denominator fields and two or three fixture families.
3. Add an xtask command that emits `target/metrics/parser_accuracy.json`.
4. Emit only denominator rows and placeholder `insufficient_data` rows for line/AST/symbol metrics.
5. Add schema validation tests.
6. Render a small parser status note pointing to the generated artifact.

Do not implement the entire metric list in the first PR.

## Review Rules

- No generated `target/metrics` or `target/receipts` artifacts are committed.
- Missing data is `insufficient_data`, not zero.
- Real-project unlabeled regions are not negative space.
- Dynamic Perl is scored conservatively; false exact precision is a blocker.
- Incremental rows are not accepted unless run with the incremental feature enabled.
- Runtime and sample count must be present before any floor is ratcheted.
