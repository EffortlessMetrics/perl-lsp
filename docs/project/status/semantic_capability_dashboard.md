# 0.13.2 Semantic Capability Dashboard

> Human-owned release summary. Keep numeric claims sourced from the generated
> semantic scorecard, shadow-compare receipts, or checked-in fixture tests.

This dashboard is the release-readable view of the semantic proof rail. The
canonical detailed artifacts remain [semantic_scorecard.md](semantic_scorecard.md),
[semantic_scorecard.json](semantic_scorecard.json),
[semantic_shadow_compare.md](semantic_shadow_compare.md), and
[semantic_shadow_compare.json](semantic_shadow_compare.json).

## Release Posture

The 0.13.2 semantic substrate is fixture-backed and available across the core
fact rows. The editor can rely on shared semantic facts for declarations,
definitions, imports, exports, occurrences, package graph edges, references,
inheritance, and role composition in the current deterministic fixtures.

The current proof is still intentionally conservative. Dynamic Perl boundaries
are represented instead of guessed, semantic method completion only cuts over
when semantic candidates cover the legacy method set, real-workspace proof is
small, and dedicated semantic query p95 rows are not yet part of the scorecard.

## Dashboard

| Row | 0.13.2 status | Release meaning | Evidence |
| --- | --- | --- | --- |
| `fact_rows_available` | `9/9` fact rows available; `0` unavailable rows | The semantic substrate is present for the current deterministic fixture family. | [semantic_scorecard.md](semantic_scorecard.md#fact-coverage) |
| `completion_import_pass_rate` | `100%` | Import/export visibility fixtures pass, including empty-import suppression and export-tag expansion. | [semantic_scorecard.md](semantic_scorecard.md#readiness-rows) |
| `method_candidates_pass_rate` | `100%` | Method candidate queries are available and passing; receiver-shape ranking is a separate next step. | [semantic_scorecard.md](semantic_scorecard.md#readiness-rows) |
| `rename_plan_pass_rate` | `100%`; unsafe edit count `0` | Rename planning is fixture-backed and currently produces no unsafe edits in the scorecard. | [semantic_scorecard.md](semantic_scorecard.md#readiness-rows) |
| `safe_delete_plan_pass_rate` | `100%` | Safe-delete blocker planning is fixture-backed for the current cases. | [semantic_scorecard.md](semantic_scorecard.md#readiness-rows) |
| `undefined_false_positive_rate` | `0%` | Undefined-symbol diagnostics have no measured false positives in the current fixture receipts. | [semantic_scorecard.md](semantic_scorecard.md#readiness-rows) |
| `dynamic_boundary_fixture_count` | `4` dynamic or dynamic-boundary fixture families; scorecard confidence breakdown reports `2` dynamic-boundary facts | Dynamic require, AUTOLOAD, eval-string, and typeglob alias cases are measured as conservative semantic boundaries rather than exact claims. | [semantic_scorecard.md](semantic_scorecard.md#fixture-ids) |
| `real_workspace_baseline_count` | `1` small CPAN-style baseline family, `4` Perl files, `2` baseline tests | Real-workspace proof has started, but it is not yet broad ecosystem coverage. | [semantic_real_workspace_baseline.rs](../../../crates/perl-workspace/tests/semantic_real_workspace_baseline.rs) |
| `method_completion_shadow_or_cutover_status` | Guarded cutover: semantic method completions are used only when semantic candidates cover the legacy method set; release-readiness shadow compare has `0` regressions and `0` unavailable receipts | Method completion can show semantic own/inherited/generated details without dropping legacy candidates; value-shape receiver ranking remains future work. | [workspace.rs](../../../crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs) and [semantic_shadow_compare.md](semantic_shadow_compare.md#release-readiness-verdict-counts) |
| `semantic_query_latency_status` | Limited: no dedicated semantic-query p95 scorecard rows yet | Existing real-project latency suites cover end-to-end LSP p50/p95/p99, but semantic query p95 rows and invalidation receipts remain a follow-up proof item. | [BENCHMARKING.md](../../how-to/BENCHMARKING.md#real-project-latency-suite) |

## Reliable User-Facing Claims

- The scorecard has no unavailable semantic rows for the current deterministic
  fixture family.
- Import completion, visible symbols, method candidates, rename planning,
  safe-delete planning, shadow-regression readiness, and undefined-symbol
  false-positive checks pass the current fixture gates.
- Dynamic Perl constructs in the current fixtures are treated conservatively
  instead of being promoted to exact semantic claims.
- Semantic method completion can surface own, inherited, and generated method
  context when the guarded cutover accepts the semantic candidate set.

## Current Limits

- Receiver-shape-driven method ranking is not yet the completion ranking proof.
- Dynamic-boundary diagnostics are only measured for the current fixture family;
  broader forms still need coverage.
- Real-workspace semantic proof currently covers one small CPAN-style family,
  not the planned Mojolicious, DBIx::Class, test-heavy, or template-heavy set.
- Semantic latency is not yet reported as `symbol_at_p95`,
  `definitions_p95`, `references_p95`, `visible_symbols_at_p95`,
  `method_candidates_p95`, `completion_semantic_p95`, or
  `single_file_fact_rebuild_p95`.
