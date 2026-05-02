# Implementation Plan: Parser Accuracy Observability

## Overview

Build a layered parser accuracy scorecard that starts with denominator visibility and then adds line, AST, symbol, recovery, incremental, span, cost, and trust metrics in reviewable slices. This is not a request to implement every metric at once. Each task should produce a small, schema-valid artifact and focused tests.

## Tasks

- [x] 1. Define the scorecard contract
  - [x] 1.1 Add `.ci/schemas/parser-accuracy.schema.json`
    - Include top-level fields: `schema_version`, `subsystem`, `generated_at`, `commit`, `cadence`, `denominator`, `families`, `metrics`, `failure_packets`, `gold_drift`, and `metric_runtime`
    - Require each metric row to be either `measured` or `insufficient_data`
    - Require measured rows to include value, sample_count, direction, and confidence
    - _Verify: schema validation test in xtask or a dedicated fixture test_
  - [x] 1.2 Add a tiny example fixture artifact
    - Create a committed example under a test fixture path, not under `target/`
    - Include denominator rows and `insufficient_data` line/AST/symbol rows
    - _Verify: JSON parses and validates_

- [x] 2. Add denominator inventory
  - [x] 2.1 Define fixture metadata shape
    - Track fixture ID, family, label mode, source path, scored lines, scored symbols, dynamic boundaries, unsupported constructs, negative regions, and generated-vs-hand-labeled source
    - Keep unlabeled regions distinct from negative regions
  - [x] 2.2 Emit denominator-only scorecard
    - Add an xtask command that writes `target/metrics/parser_accuracy.json`
    - Report fixture_count, fixture_family_count, scored_line_count, scored_symbol_count, fully_labeled_region_count, partial_labeled_region_count, unknown_region_count, negative_region_count, dynamic_boundary_case_count, unsupported_construct_case_count, real_project_file_count, generated_fixture_count, and hand_labeled_fixture_count
    - _Verify: targeted xtask tests and schema validation_

- [x] 3. Wire parser status visibility
  - [x] 3.1 Extend parser status rendering with accuracy artifact summary
    - Show denominator rows and `insufficient_data` rows without pretending they are zero
    - Link to parser accuracy artifact location and spec
    - _Verify: `cargo test -p xtask update_status::parser --profile agent --locked`_
  - [x] 3.2 Preserve existing clean-parse status rows
    - Do not remove current clean parse, recovery, token health, or failure worklist rows
    - _Verify: existing parser status marker tests_

- [x] 4. Add line-level construct scoring
  - [x] 4.1 Define line tag vocabulary
    - Include package_decl, sub_decl, method_decl, variable_decl, import, export, function_call, method_call, regex, quote_like, heredoc_opener, heredoc_body, heredoc_terminator, pod, format_decl, given_when, do_while, until_loop, dynamic_boundary, parse_error, recovery_region, unsupported_construct
  - [x] 4.2 Implement line tag scorer
    - Compare expected and actual tag sets
    - Emit TP/FP/FN, exact match rate, precision, recall, F1, error false positive rate, error false negative rate, dynamic boundary correct rate, and unsupported detection rate
    - _Verify: unit tests with at least one false positive and one false negative_

- [x] 5. Add AST structural scoring
  - [x] 5.1 Define AST gold fixture expectations
    - Track node kind, span, parent-child edge, and operator-precedence expectations
  - [x] 5.2 Implement AST scorer
    - Emit node-kind precision/recall/F1, span exact/near rates, parent-child edge accuracy, tree depth accuracy, operator precedence accuracy, delimiter pairing accuracy, unexpected error node count, and missing expected node count
    - _Verify: fixture with correct line tag but wrong parent-child edge fails AST score_

- [ ] 6. Add symbol and edge scoring
  - [ ] 6.1 Define symbol expectation shape
    - Track declarations, references, imports, exports, scopes, packages, spans, definition edges, reference edges, provenance, and confidence
  - [ ] 6.2 Consume canonical fact shards where available
    - Use anchors, entities, occurrences, edges, real byte spans, provenance, confidence, and hashes
  - [ ] 6.3 Emit symbol precision/recall/F1 rows by kind
    - Include package, subroutine, method, lexical variable, global variable, import, export, typeglob alias, generated accessor, role method, inherited method, and dynamic boundary
    - _Verify: at least one fixture each for generated accessor and typeglob alias_

- [ ] 7. Add false-positive and dynamic-boundary gates
  - [ ] 7.1 Emit false-positive counts
    - false symbols, false declarations, false references, false imports, false exports, false parse errors, false exact resolutions, false dynamic resolutions, and symbols emitted in comments/POD/strings/unknown regions
  - [ ] 7.2 Add false precision safety rows
    - Emit `dynamic_false_precision_count`
    - Prepare floor contract for `dynamic_false_precision_count == 0`
    - _Verify: dynamic fixture returns fallback/unavailable rather than false exact result_

- [ ] 8. Add recovery quality scoring
  - [ ] 8.1 Define malformed fixture labels
    - Track first error line, expected error region, recovery boundary, and post-error symbols
  - [ ] 8.2 Emit recovery containment rows
    - first_error_line_accuracy, error_region_precision/recall, spillover mean/p95/max, salvaged lines, salvaged symbols, post_error_symbol_recall, post_error_line_f1
    - _Verify: malformed heredoc fixture distinguishes local recovery from EOF spillover_

- [ ] 9. Add incremental equivalence scoring
  - [ ] 9.1 Add full-vs-incremental comparison fixtures
    - Compare full parse of final source with incremental parse after edit sequence
    - Run with `--features incremental`
  - [ ] 9.2 Emit incremental rows
    - equivalence rate, edit apply equivalence, no panic, no-progress count, timeout count, fallback rate, checkpoint hit/miss, reparse byte ratio, reused token/node ratios, changed range accuracy
    - _Verify: `cargo test -p perl-parser --features incremental --locked incremental`_

- [ ] 10. Add span and coordinate scoring
  - [ ] 10.1 Add span fixture families
    - UTF-8 multibyte, emoji/surrogate-style code points, CRLF, mixed newline styles, tabs, BOM, empty spans, and cross-line spans
  - [ ] 10.2 Emit span rows
    - byte span exact, line span exact, UTF-16 range exact, span near, invalid/out-of-bounds/inverted/non-char-boundary counts, CRLF errors, Unicode errors, tab column mismatches

- [ ] 11. Add confidence, unsupported construct, and provider-impact rows
  - [ ] 11.1 Emit confidence calibration rows
    - exact/high/medium/low/heuristic/dynamic precision and calibration error
  - [ ] 11.2 Emit unsupported construct rows
    - detected, missed, family count, false exact, and salvaged counts
  - [ ] 11.3 Add provider-impact placeholders
    - document symbols, goto definition, references, hover, completion, rename, safe delete, diagnostics
    - Use `insufficient_data` until provider gold fixtures are wired

- [ ] 12. Add cost, scale, cache, determinism, and metric-runtime rows
  - [ ] 12.1 Emit scale shape
    - bytes, lines, tokens, AST nodes, symbols, imports, exports, subs, packages, nesting, brace depth, regex length, heredoc bytes, quote-like count, dynamic boundary count
  - [ ] 12.2 Emit cost rows
    - lex/parse/AST projection/recovery/semantic/index/query timings and memory/allocation rows where available
  - [ ] 12.3 Emit cache and reuse rows
    - lexer/parser checkpoint reuse, semantic fact cache hit, workspace shard reuse, unchanged file skip, content hash hit, fast path attempts/success/fallback/wrong result
  - [ ] 12.4 Emit determinism rows
    - parse/token/AST/fact/diagnostic hash stability and whitespace/comment/newline/incremental/repeated-parse invariants
  - [ ] 12.5 Emit metric runtime rows
    - runtime ms, timeout count, flake count, artifact size, CI runner failures, orphan process count, cache hit rate

- [ ] 13. Add gold drift audit
  - [ ] 13.1 Validate gold fixtures before scoring
    - schema errors, span errors, duplicate symbol IDs, missing resolves-to targets
  - [ ] 13.2 Emit gold change counts
    - changed lines, changed symbols, removed expectations, added expectations, dynamic expectation changes
  - [ ] 13.3 Require explanation for weakening changes
    - Removing expected symbols, widening spans, lowering thresholds, changing dynamic expectations, or removing fixture families requires PR text

- [ ] 14. Add ratchet candidates after stable measurements
  - [ ] 14.1 Start with safety floors only
    - `dynamic_false_precision_count == 0`
    - `fast_path_wrong_result_count == 0`
  - [ ] 14.2 Defer precision/recall floors until sample counts stabilize
    - No floor raise from a single run
    - Require current/previous/delta/floor/threshold/direction/sample_count/confidence for all floor candidates

- [ ] 15. Final verification checkpoint
  - [ ] 15.1 Parser accuracy command
    - `cargo xtask metrics parser-accuracy --json`
    - Artifact validates against schema
    - No generated `target/` artifacts committed
  - [ ] 15.2 Parser status
    - `cargo xtask update-status --only parser --check`
    - Parser status shows measured denominator rows and insufficient-data rows honestly
  - [ ] 15.3 Focused tests
    - `cargo test -p xtask parser_accuracy`
    - `cargo test -p xtask update_status::parser --profile agent --locked`
    - `cargo check -p xtask --all-targets --profile agent --locked`
