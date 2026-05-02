# Requirements Document

## Introduction

The Parser Accuracy Observability system extends the current parser scorecard from clean-parse and recovery-health tracking into layered accuracy measurement. The goal is to answer, with evidence, how well perl-lsp parses Perl and how accurate the derived editor-facing facts are.

The current parser scorecard already tracks corpus cleanliness, error density, recovery worklists, node-kind coverage, and parse cost. That is necessary, but not sufficient. A parser can parse a file cleanly while attaching nodes incorrectly, missing symbols, inventing false references, widening spans, or making an incremental fast path return a different result from a full parse. This system adds the denominator, gold-scoring, structural accuracy, semantic accuracy, incremental equivalence, span correctness, and metric-trust layers needed to make parser progress measurable without conflating parser success with editor UX or release readiness.

### Implementation Ordering Constraint

Requirements MUST be implemented in small slices. The canonical order is:

1. **Metric contract and schemas** - define parser accuracy artifact shape and fixture metadata contract.
2. **Fixture denominator inventory** - report what is labeled before scoring accuracy.
3. **Line-level construct scoring** - compare expected line tags with observed tags.
4. **AST structural scoring** - compare node kinds, spans, and parent-child edges on gold fixtures.
5. **Symbol and edge scoring** - score declarations, references, imports, exports, and resolution edges.
6. **False-positive and dynamic-boundary scoring** - make invented precision visible.
7. **Recovery and incremental equivalence scoring** - measure salvage and fast-path correctness.
8. **Cost, scale, cache, determinism, and gold-drift rows** - prevent misleading speed or accuracy wins.
9. **Status rendering and ratchets** - render measured rows and only ratchet floors after stable samples.

Do not implement provider impact, real-project partial labels, release thresholds, or rolling trend windows before the first scorecard artifact exists and validates against schema.

### Ownership

- **Metric command owner:** `xtask` parser metrics tasks.
- **Status owner:** `xtask/src/tasks/update_status/parser.rs`.
- **Gold fixture owner:** parser and semantic fixture crates that already own source snippets.
- **Runtime artifacts:** generated under `target/metrics/` or `target/receipts/`.
- **Committed contracts and baselines:** `.ci/schemas/`, `.ci/metrics/baselines/`, and docs/spec files.

## Glossary

- **Parser_Accuracy_Scorecard:** A machine-readable JSON artifact containing parser denominator, clean parse, line, AST, symbol, span, incremental, cost, scale, cache, determinism, gold-drift, and metric-runtime rows.
- **Fixture_Family:** A named Perl construct family such as `heredoc`, `typeglob_alias`, `generated_accessor`, `dynamic_require`, `quote_like`, or `signatures`.
- **Gold_Fixture:** A fixture with explicit expected tags, nodes, symbols, spans, edges, or dynamic-boundary outcomes.
- **Partial_Label:** A real-project or large fixture annotation that scores only known regions and does not treat unlabeled regions as negative space.
- **Negative_Region:** A labeled region where the scorer expects no symbol, edge, diagnostic, parse error, or dynamic resolution.
- **Line_Tag:** A construct tag assigned to a source line, such as `sub_decl`, `import`, `regex`, `pod`, `dynamic_boundary`, or `recovery_region`.
- **False_Precision:** A confident parser or semantic result for a dynamic or unsupported Perl construct where the correct behavior is conservative fallback, unavailable, warning, or blocker.
- **Recovery_Spillover:** Lines or regions after the first parse error that remain affected by recovery.
- **Incremental_Equivalence:** Equality between full parsing of final source and incremental parsing after applying the same edit sequence.
- **Metric_State:** A row state of `measured` or `insufficient_data`; insufficient data is not a parse result.

## Requirements

### Requirement 1: Denominator Visibility

**User Story:** As a parser maintainer, I want accuracy reports to show the labeled input denominator, so that percentages cannot overstate confidence from tiny or narrow fixtures.

#### Acceptance Criteria

1. WHEN a parser accuracy scorecard is generated, THE scorecard SHALL report fixture count, fixture family count, scored line count, scored symbol count, fully labeled region count, partial labeled region count, unknown region count, negative region count, dynamic boundary case count, unsupported construct case count, real project file count, generated fixture count, and hand-labeled fixture count.
2. THE scorecard SHALL break denominator counts down by fixture family.
3. THE scorecard SHALL distinguish fully labeled gold fixtures from partial-label real-project fixtures.
4. THE scorecard SHALL NOT treat unlabeled regions as false positives unless they are explicitly marked as negative regions.

### Requirement 2: Fixture Family Coverage

**User Story:** As a reviewer, I want parser scorecards to show which Perl construct families are represented, so that rare but important Perl features do not disappear behind micro averages.

#### Acceptance Criteria

1. THE fixture inventory SHALL support at least these families: packages, subroutines, methods, lexicals, globals, imports, exports, qualified references, bare references, same bare sub in multiple packages, typeglob aliases, AUTOLOAD, eval string, dynamic require, generated accessors, roles, inheritance, heredocs, regexes, quote-like operators, POD, format statements, Moose/Moo DSL, signatures/invocants, postderef, do-while/until, and given/when/default.
2. THE scorecard SHALL report macro averages across fixture families separately from micro averages across all scored items.
3. THE scorecard SHALL identify worst-performing fixture families by score and sample count.

### Requirement 3: Clean Parse Metrics

**User Story:** As a parser maintainer, I want clean-parse rates to remain visible but not be treated as the whole accuracy story.

#### Acceptance Criteria

1. THE scorecard SHALL report clean parse file rate, clean parse line rate, parse error density per KLOC, first error bucket, first error line, files with recovery, strict clean rate, and partial clean rate.
2. THE scorecard SHALL report parser failure clusters including quote/transliteration, heredoc/delimiter, declaration/package, recovery-only, encoding/multibyte, regex, operator precedence, and incremental edit application.
3. THE parser status renderer SHALL continue to show failure clusters in the parser status page.

### Requirement 4: Line-Level Construct Accuracy

**User Story:** As an editor feature maintainer, I want line-level construct scores, so that we know whether the parser understood source shape before provider behavior is measured.

#### Acceptance Criteria

1. THE scorecard SHALL compare expected line tags with observed line tags.
2. THE scorecard SHALL compute line exact match rate, line construct precision, line construct recall, line construct F1, line error false positive rate, line error false negative rate, line dynamic boundary correct rate, and unsupported line detection rate.
3. THE scoring model SHALL compute true positives, false positives, and false negatives from set intersection and set difference between expected and observed tags.
4. THE line tag vocabulary SHALL include package declarations, sub declarations, method declarations, variable declarations, imports, exports, function calls, method calls, regexes, quote-like operators, heredoc opener/body/terminator, POD, format declarations, given/when, do-while, until loops, dynamic boundaries, parse errors, recovery regions, and unsupported constructs.

### Requirement 5: AST Structural Accuracy

**User Story:** As a parser maintainer, I want structural AST scores, so that a clean parse cannot hide incorrect tree shape.

#### Acceptance Criteria

1. THE scorecard SHALL report node-kind precision, node-kind recall, node-kind F1, node span exact rate, node span near rate, parent-child edge accuracy, tree depth accuracy, operator precedence accuracy, delimiter pairing accuracy, recovery node count, unexpected error node count, and missing expected node count.
2. THE scorecard SHALL break AST metrics down by declarations, control flow, expressions, operators, regex nodes, quote-like nodes, heredoc nodes, package/class/role nodes, variable declarations, function calls, and method calls.
3. THE scorecard SHALL classify a line tag match with an incorrect parent-child edge as structurally wrong.

### Requirement 6: Symbol and Edge Accuracy

**User Story:** As a semantic feature maintainer, I want symbol and edge scores, so that we know whether parser output produces correct declarations, references, imports, exports, scopes, spans, and relationships.

#### Acceptance Criteria

1. THE scorecard SHALL report precision, recall, and F1 for symbol declarations, symbol references, definition edges, and reference edges.
2. THE scorecard SHALL report import precision/recall, export precision/recall, canonical name accuracy, display name accuracy, symbol kind accuracy, scope accuracy, package accuracy, span exact rate, span near rate, and semantic match rate.
3. THE scorecard SHALL break symbol rows down by package, subroutine, method, lexical variable, global variable, import, export, typeglob alias, generated accessor, role method, inherited method, and dynamic boundary.
4. THE scorecard SHALL use canonical fact shard fields such as anchors, entities, occurrences, edges, byte spans, provenance, confidence, and hashes where available.

### Requirement 7: False Positive and False Precision Tracking

**User Story:** As a reviewer, I want invented symbols and false exact resolutions reported separately, so that missing a fact is not conflated with unsafe overconfidence.

#### Acceptance Criteria

1. THE scorecard SHALL report false symbol count, false declaration count, false reference count, false import count, false export count, false parse error count, false exact resolution count, false dynamic resolution count, symbols emitted in comments, symbols emitted in POD, symbols emitted in strings, and symbols emitted in unknown regions.
2. THE scorecard SHALL report `dynamic_false_precision_count`.
3. THE floor `dynamic_false_precision_count == 0` SHALL be eligible for ratchet enforcement after the first measured artifact is stable.
4. Dynamic, unavailable, ambiguous, or unsupported cases SHALL score as conservative fallback when no exact static answer is justified.

### Requirement 8: Recovery Quality and Salvage

**User Story:** As a user of malformed or incomplete code, I want parser recovery to be contained, so that one syntax error does not poison the rest of the file.

#### Acceptance Criteria

1. THE scorecard SHALL report first error line accuracy, error region precision, error region recall, recovery spillover mean, recovery spillover p95, recovery spillover max, salvaged lines after error, salvaged symbols after error, post-error symbol recall, and post-error line F1.
2. THE scorecard SHALL distinguish local recovery from recovery regions that extend to EOF.
3. THE status page SHALL identify worst recovery fixture families when sample count is sufficient.

### Requirement 9: Incremental Equivalence

**User Story:** As a performance maintainer, I want incremental parsing measured against full parsing, so that fast paths cannot silently return wrong output.

#### Acceptance Criteria

1. THE scorecard SHALL compare full parse of final source with incremental parse after applying the same edit sequence.
2. THE scorecard SHALL report incremental full parse equivalence rate, incremental edit apply equivalence rate, incremental no panic rate, incremental no-progress count, incremental timeout count, incremental full reparse fallback rate, incremental checkpoint hit rate, incremental checkpoint miss rate, incremental reparse byte ratio, incremental reused token ratio, incremental reused node ratio, and incremental changed range accuracy.
3. THE floor `fast_path_wrong_result_count == 0` SHALL be eligible for ratchet enforcement after the first measured artifact is stable.
4. Incremental checks SHALL be run with the actual `incremental` feature enabled.

### Requirement 10: Span and Coordinate Correctness

**User Story:** As an LSP provider maintainer, I want span and range correctness measured directly, so that parsing accuracy carries through to editor positions.

#### Acceptance Criteria

1. THE scorecard SHALL report byte span exact rate, line span exact rate, UTF-16 range exact rate, span near rate, span invalid count, span out-of-bounds count, span inverted count, span non-char-boundary count, CRLF position error count, Unicode position error count, and tab column mismatch count.
2. THE fixture inventory SHALL include UTF-8 multibyte, surrogate-pair-style code points, CRLF, mixed newline styles, tabs, BOM, empty spans, and cross-line spans.

### Requirement 11: Confidence Calibration

**User Story:** As a provider maintainer, I want confidence levels calibrated, so that high-confidence facts are trustworthy and heuristic facts do not drive unsafe actions.

#### Acceptance Criteria

1. THE scorecard SHALL report exact fact precision, high-confidence precision, medium-confidence precision, low-confidence precision, heuristic fact precision, dynamic boundary precision, and confidence calibration error.
2. THE scorecard SHALL distinguish exact, high-confidence, heuristic, and dynamic-boundary provenance.
3. Unsafe provider actions SHALL NOT be powered by low-confidence or dynamic-boundary facts without explicit blocker/warning behavior.

### Requirement 12: Unsupported Construct Honesty

**User Story:** As a product owner, I want unsupported Perl constructs visible, so that accuracy improves honestly without pretending dynamic Perl is static.

#### Acceptance Criteria

1. THE scorecard SHALL report unsupported construct detected count, unsupported construct missed count, unsupported construct family count, unsupported construct false exact count, and unsupported-but-salvaged count.
2. Symbolic calls, dynamic requires, eval string, AUTOLOAD, and unsupported DSL constructs SHALL be eligible for conservative dynamic/unsupported labels.
3. Unsupported constructs SHALL NOT disappear from denominator counts.

### Requirement 13: Provider Impact

**User Story:** As an editor UX maintainer, I want parser accuracy connected to provider outcomes, so that parser improvements are visible in editor behavior without merging scorecard ownership.

#### Acceptance Criteria

1. THE parser accuracy scorecard SHALL expose provider-impact rows for document symbol precision/recall, goto-definition hit rate, references precision/recall, hover symbol origin accuracy, completion visible symbol relevance, completion import visibility accuracy, rename safe edit accuracy, safe-delete blocker accuracy, diagnostic false positive rate, and diagnostic false negative rate.
2. Provider impact rows MAY be `insufficient_data` until gold provider fixtures are wired.
3. Provider impact rows SHALL reference editor-intelligence or UX scorecards rather than duplicating their ownership.

### Requirement 14: Real-Project Partial Labels

**User Story:** As a maintainer, I want real-project behavior measured separately from gold fixtures, so that weird real Perl informs progress without corrupting precision math.

#### Acceptance Criteria

1. THE scorecard SHALL report real project clean parse rate, real project error density, real project recovery spillover p95, real project symbol density, real project parse p95 ms, real project index p95 ms, and real project memory peak MB.
2. Real-project scoring SHALL support partial labels for known package declarations, sub declarations, imports, exports, selected references, and selected dynamic boundaries.
3. Unlabeled real-project regions SHALL NOT count as false positives unless marked as negative regions.

### Requirement 15: Cost, Speed, and Scale Shape

**User Story:** As a performance reviewer, I want phase cost and scale context, so that speed wins are not misleading or only true for tiny files.

#### Acceptance Criteria

1. THE scorecard SHALL report lex, parse, AST projection, recovery, semantic extraction, workspace insert, definition query, reference query, completion query, and incremental edit p50/p95 timings where applicable.
2. THE scorecard SHALL report memory and allocation rows including peak RSS MB, allocated bytes, allocation count, tokens allocated, AST nodes allocated, semantic facts allocated, incremental state bytes, checkpoint bytes, and cache size bytes where available.
3. THE scorecard SHALL report file bytes, line count, token count, AST node count, symbol count, import count, export count, sub count, package count, max nesting depth, max brace depth, max regex length, max heredoc body bytes, quote-like count, heredoc count, and dynamic boundary count.
4. THE scorecard SHALL normalize cost with parse ms per KB, parse ms per 1k tokens, semantic ms per symbol, workspace index ms per file, correct symbols per ms, and correct lines per ms.

### Requirement 16: Cache and Reuse Quality

**User Story:** As an incremental parser maintainer, I want reuse quality measured, so that speed gains are attributable and correct.

#### Acceptance Criteria

1. THE scorecard SHALL report lexer checkpoint reuse rate, parser checkpoint reuse rate, semantic fact cache hit rate, workspace shard reuse rate, unchanged file skip rate, content hash hit rate, fast path attempt count, fast path success count, fast path fallback count, and fast path wrong result count.
2. THE floor `fast_path_wrong_result_count == 0` SHALL be eligible for ratchet enforcement after the first stable measured artifact exists.

### Requirement 17: Determinism and Invariants

**User Story:** As a reviewer, I want parse output determinism measured, so that nondeterministic facts or hashes cannot silently destabilize providers.

#### Acceptance Criteria

1. THE scorecard SHALL report parse hash stability rate, token stream hash stability rate, AST hash stability rate, semantic fact hash stability rate, and diagnostic hash stability rate.
2. THE scorecard SHALL report whitespace invariance rate, comment invariance rate, newline style invariance rate, incremental equivalence rate, and repeated parse determinism rate.
3. Adding comments SHALL NOT change symbol facts except for spans when expected by fixture metadata.
4. CRLF and LF variants SHALL preserve line tags unless fixture metadata explicitly permits a difference.

### Requirement 18: Failure Attribution

**User Story:** As a fix-forward agent, I want failed scorecard rows to identify the likely layer, so that regressions are actionable.

#### Acceptance Criteria

1. THE scorecard SHALL emit failure packets with failure kind, likely layer, fixture ID, line, parser line tag presence, AST node presence, semantic fact presence, and provider result presence.
2. Likely layers SHALL include lexer, parser, recovery, AST projection, semantic fact extraction, workspace index, provider query, gold/schema issue, and CI/metric infrastructure.
3. The status page SHALL summarize top failure packets without requiring raw log inspection.

### Requirement 19: Gold Quality and Drift

**User Story:** As a reviewer, I want gold fixture changes audited, so that accuracy improvements cannot come from weakening expected truth.

#### Acceptance Criteria

1. THE scorecard SHALL report gold schema errors, gold span errors, gold duplicate symbol IDs, gold missing resolves-to targets, gold changed line count, gold changed symbol count, gold removed expectation count, gold added expectation count, and gold dynamic expectation changes.
2. Removing expected symbols, widening expected spans, changing expected dynamic behavior, lowering thresholds, or removing fixture families SHALL require a human-readable explanation in the PR.
3. Gold fixture validation SHALL run before score computation.

### Requirement 20: Metric Runtime and Flakiness

**User Story:** As a CI operator, I want the parser accuracy system itself measured, so that it remains trustworthy and cheap enough for its cadence tier.

#### Acceptance Criteria

1. THE scorecard SHALL report metric runtime ms, metric timeout count, metric flake count, metric artifact size bytes, metric CI runner failure count, metric orphan process count, and metric cache hit rate.
2. PR-fast metrics SHALL be bounded and small.
3. Merge-gate metrics SHALL be sharded when they exceed the fast lane budget.
4. Nightly and release metrics MAY run the full corpus and real-project labels.

### Requirement 21: Deltas, Floors, and Confidence

**User Story:** As a maintainer, I want every metric row to show current, previous, delta, floor, threshold, direction, sample count, and confidence, so that movement is interpreted correctly.

#### Acceptance Criteria

1. Each metric row SHALL support current value, previous value, delta, floor, threshold, direction, sample count, and confidence.
2. Missing or below-threshold sample counts SHALL report `insufficient_data`, not zero.
3. Floors SHALL NOT be raised from a single lucky run.
4. Macro and micro averages SHALL both be available for line, AST, symbol, and provider-impact scores.
