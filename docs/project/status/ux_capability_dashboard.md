# Semantic UX Capability Dashboard

This dashboard maps parser accuracy and semantic proof rails to editor-facing
Perl language intelligence.

It answers one question:

> Does the editor feel Perl-aware?

This dashboard is **descriptive**, not generative: parser and semantic metrics
are consumed from existing artifacts (linked below). Updates here record how
those metrics translate into user-facing capability claims.

## Status vocabulary

| Status | Meaning |
|---|---|
| `legacy` | Provider still uses legacy/local behavior. |
| `semantic-shadow` | Semantic path is measured but not primary. |
| `semantic-live` | Semantic path drives user-visible behavior. |
| `semantic-live-with-fallback` | Semantic path drives behavior when available; legacy path remains fallback. |
| `insufficient_data` | Not enough proof to make a durable claim. |

## Data ownership

This dashboard consumes existing parser accuracy and semantic scorecard
artifacts. It does not recompute metrics and does not own their source values.

Source-of-truth artifacts:

| Input family | Source of truth |
|---|---|
| Parser accuracy | [parser.md](parser.md) and parser-accuracy artifacts |
| Semantic facts / readiness | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) and semantic scorecard artifacts |
| Shadow comparison | semantic shadow-compare receipts |
| UX status | this dashboard, manually maintained from source artifacts |

When a source value changes, update the source artifact first, then refresh
the corresponding row here. Never edit a row in this dashboard to "fix" a
number that disagrees with its source — fix the source.

## TBD policy

`TBD` means the row shape is defined but no durable value has been assigned
yet. A `TBD` row should become one of:

- `legacy`
- `semantic-shadow`
- `semantic-live`
- `semantic-live-with-fallback`
- `insufficient_data`

during a follow-up population PR. `TBD` is not a permanent status — it is a
placeholder that signals "structure is set, value pending."

## Status transition rules

| From | To | Requirement |
|---|---|---|
| `legacy` | `semantic-shadow` | Semantic path exists and has shadow / proof receipts |
| `semantic-shadow` | `semantic-live` | Provider uses semantic path as primary behavior |
| `semantic-live` | `semantic-live-with-fallback` | Provider uses semantic path when indexed data exists and preserves legacy fallback |
| any | `insufficient_data` | Proof source is missing, stale, or too thin |
| any | `legacy` | Semantic path removed or disabled |

A transition is a documentation change in this dashboard *plus* a link to
the receipt that proves the new state. Promotions without a receipt should
remain at the previous level.

## First population pass

The first population PR fills only rows backed by durable artifacts:

- existing scorecard receipts
- existing shadow-compare receipts
- existing parser-accuracy receipts
- merged provider behavior already in production

It does **not**:

- infer values from code inspection alone
- promote rows to `semantic-live` without a runtime receipt
- copy numbers that may go stale; prefer linking to the source artifact
- expand the dashboard's scope into other rails

## Parser accuracy inputs

Compact summary only. Full detail lives in the parser accuracy status
artifact; this table consumes it without mirroring numbers that live elsewhere.

| Input | Current read | Why it matters |
|---|---:|---|
| `fixture_count` / `family_count` | TBD | Denominator quality |
| `line_construct_f1` | TBD | Source-shape understanding |
| `ast_node_kind_f1` | TBD | AST structural accuracy |
| `symbol_decl_f1` | TBD | Declaration extraction |
| `symbol_ref_f1` | TBD | Reference extraction |
| `dynamic_false_precision_count` | TBD | Perl dynamic safety |
| `fast_path_wrong_result_count` | TBD | Incremental / fast-path safety |
| `failure_packet_count` | TBD | Actionable remaining gaps |
| `insufficient_data_count` | TBD | Honesty about unproven rows |

See [parser.md](parser.md) for the canonical parser corpus and coverage view.

## Semantic scorecard inputs

Compact summary only. Full detail lives in the semantic scorecard and
release-readability dashboards.

| Input | Current read | Why it matters |
|---|---:|---|
| `declaration_facts` | TBD | Symbol declarations |
| `occurrence_facts` | TBD | Uses / references |
| `definition_candidates` | TBD | Goto / hover / rename substrate |
| `reference_edges` | TBD | References and safe edits |
| `import_specs` | TBD | Visibility and diagnostics |
| `export_facts` | TBD | Completion / rename safety |
| `package_graph_edges` | TBD | Inheritance / roles / methods |
| `method_candidates_fixture_pass_rate` | TBD | Method completion |
| `rename_plan_pass_rate` | TBD | Safe rename |
| `safe_delete_plan_pass_rate` | TBD | Safe delete |
| `undefined_symbol_false_positive_fixture_rate` | TBD | Diagnostic trust |
| `visible_symbols_fixture_pass_rate` | TBD | Completion and hover visibility |

See [semantic_capability_dashboard.md](semantic_capability_dashboard.md) for the
release-readable view, and `semantic_scorecard.md` / `semantic_scorecard.json`
for the underlying receipts.

## Editor UX capability rows

One row per LSP surface. Each row names its proof source and a concrete next
improvement so the dashboard identifies leverage as well as state.

| UX surface | Status | Proof source | Current user-facing claim | Current limits | Next improvement |
|---|---|---|---|---|---|
| Completion | TBD | TBD | TBD | TBD | Rank visible symbols by provenance |
| Method completion | TBD | TBD | TBD | TBD | Receiver-aware value-shape ranking |
| Hover | TBD | TBD | TBD | TBD | Explain origin / confidence / dynamic boundaries |
| Diagnostics | TBD | TBD | TBD | TBD | Count dynamic-boundary suppressions |
| Goto definition | TBD | TBD | TBD | TBD | Improve candidate confidence explanations |
| Find references | TBD | TBD | TBD | TBD | Deepen callsite / coderef / typeglob coverage |
| Rename | TBD | TBD | TBD | TBD | Expose blocker explanations in LSP responses |
| Safe delete | TBD | TBD | TBD | TBD | Expose blocker explanations in LSP responses |
| Document symbols | TBD | TBD | TBD | TBD | Tie to parser / symbol accuracy rows |
| Workspace symbols | TBD | TBD | TBD | TBD | Tie to workspace semantic index |
| Semantic tokens | TBD | TBD | TBD | TBD | Tie to parser line / AST accuracy |

## Dynamic Perl honesty

| Row | Current read | Policy |
|---|---:|---|
| dynamic boundary detected | TBD | Prefer conservative `unavailable` / `ambiguous` over false exactness |
| ambiguous result | TBD | Surface uncertainty; do not pretend exactness |
| unavailable result | TBD | Acceptable when dynamic Perl prevents safe resolution |
| low-confidence result | TBD | May inform ranking, not unsafe edits |
| false-exact result count | TBD | Should be zero |
| unsafe-edit count | TBD | Should be zero |

The dashboard rewards conservative honesty. It does not imply full static
resolution of dynamic Perl.

## Reliable user-facing claims

- Imported symbols can be explained when exact import facts exist.
- Dynamic strict-bareword diagnostics are suppressed only when semantic
  evidence supports suppression.
- Rename and safe-delete are conservative and may block unsafe edits with
  explanations.
- Method completion is improving, but unknown dynamic receivers must not
  invent exact methods.

## Current limits

- No full Perl type inference.
- No runtime symbolic evaluator.
- No full Moose / Moo metamodel.
- No complete CPAN metadata resolver.
- Dynamic Perl remains conservative.
- Parser and semantic metrics are consumed from existing artifacts, not
  recomputed here.

## Next recommended UX improvement

Receiver-aware method completion ranking using value-shape-lite hints:

- `$self->` — current package methods rank higher
- `Foo->new` assignment — `Foo` methods rank higher
- literal `bless` — package methods rank with medium confidence
- unknown receiver — safe fallback to broad workspace candidates
- dynamic receiver — low confidence; never outranks exact evidence
