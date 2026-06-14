---
tags: [parser, ast, nodekind, exhaustiveness, if-let, wildcard-arm, visit-children, scanner-blindness, silent-drop, lsp-feature-gap]
repos: [perl-lsp-swarm]
related: ["#1457", "#1362"]
portable: true
article_asset: true
search_terms: [NodeKind, NestedVariableList, exhaustiveness, if let NodeKind, wildcard arm, visit_children, silent drop, semantic tokens, hover, go-to-definition, rename, reference tracking, node_analysis, variable_decl_from_node, #1457, #1362]
---

# New NodeKind variant silently dropped by three non-exhaustive consumers

**Date**: 2026-06
**Hazard class**: scanner-blindness (non-exhaustive-consumer variant)
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 4 generalization)

## What happened

PR #1457 (issue #1362) added `NodeKind::NestedVariableList` to support nested
variable declarations in Perl's `my ($x, ($y, $z)) = ...` form. The builder
correctly extended all **exhaustive** `match NodeKind::` arms — these are
compiler-enforced and would not compile otherwise. However, three non-exhaustive
consumer patterns silently swallowed the new variant and produced no output for
it, each causing a distinct user-facing LSP feature gap:

1. **`perl-semantic-analyzer` — `node_analysis` loop** (`crates/perl-semantic-analyzer/src/analysis/node_analysis.rs`):
   An `if let NodeKind::VariableListDeclaration { .. } = node.kind` guard inside
   a node-walk loop had no `else` branch. `NestedVariableList` nodes were skipped
   entirely — no semantic tokens or hover information was produced for variables
   declared inside nested lists.

2. **`perl-symbol` — `variable_decl_from_node`** (`crates/perl-symbol/src/surface/decl.rs`
   or equivalent in the symbol extraction layer): A function that mapped
   `NodeKind::VariableListDeclaration` to a declaration record had no arm for
   `NestedVariableList`. The function returned `None`, so nested-list variables
   generated no workspace symbols — go-to-definition and rename were silently
   broken for them.

3. **`perl-workspace` — `visit_children`** (`crates/perl-workspace/src/traversal/visit.rs`
   or equivalent): A traversal helper used a `_ => { /* no children */ }` wildcard
   arm for unrecognized variants. `NestedVariableList` matched the wildcard, so
   its child nodes were never visited — reference tracking produced no cross-file
   references for variables in nested declarations.

Deep-review (commit `c5c8f6bf8` on PR #1457) caught all three gaps and added
explicit arms for `NestedVariableList` in each consumer before merge.

## Why

The Rust exhaustiveness checker enforces complete coverage only over `match`
expressions with no wildcard arm. The three consumer patterns above are structurally
invisible to it:

- An `if let NodeKind::X { .. } = ...` with no `else` branch is a pattern match
  that **succeeds or skips** — new variants are silently skipped. The compiler
  never warns.
- A `_ => { /* no children */ }` wildcard arm in a `match` is fully exhaustive —
  adding a new variant does not change the match from the compiler's perspective.
  The new variant falls into the no-op arm.

Both patterns are common in traversal and extraction code where "unrecognized
variants do nothing" was intentional at write-time but becomes a silent gap
every time a new variant is added.

## Fix

Deep-review (commit `c5c8f6bf8`, PR #1457) added three explicit consumer arms:

- `node_analysis.rs`: extended the `if let` guard to also match `NestedVariableList`
  (or restructured to a `match` covering both variants) so semantic tokens and
  hover are emitted for inner variables.
- `variable_decl_from_node` (perl-symbol surface/decl layer): added a
  `NodeKind::NestedVariableList` arm that maps to a declaration record,
  restoring workspace symbols and enabling go-to-definition and rename.
- `visit_children` (perl-workspace traversal): replaced or augmented the wildcard
  arm with an explicit `NodeKind::NestedVariableList` arm that recurses into
  child nodes, restoring reference tracking.

## Spec impact

Added `PARSER-5` to `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md`: when a
change adds a new `NodeKind` variant, the spec must mandate a grep for
`if let NodeKind::` loops without an else branch and `_ =>` wildcard arms in
traversal/extraction code (especially `visit_children`, semantic-token emitters,
symbol extractors). Each silent drop is one missing LSP feature for the new
construct.

Also added a pointer to this hazard in `docs/reference/PARSER_CONTRACTS.md`
§4 (NodeKind Classification).

## Portable lesson

The exhaustiveness checker is a necessary but insufficient guard when adding
enum variants. Non-exhaustive consumer patterns — `if let` with no else, and
wildcard arms that "do nothing" — are blind spots. The equivalent in any
language or system is a default handler that silently absorbs new cases.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 4 (Scanner Literal / Comment Blindness) — generalized to
  "consumer blindness": a consumer that processes a known variant but silently
  drops variants it does not recognize, including newly added ones. The
  structural cause is the same: a filter with a silent no-op path.
- **Generalization**: Every new enum variant requires auditing the non-exhaustive
  consumer surface — not just exhaustive matches — because the compiler only
  guards the latter. Each un-audited `if let` or wildcard arm is a silent drop
  waiting to ship.

## Related PRs

- [#1457](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1457) — the PR where
  `NodeKind::NestedVariableList` was added; deep-review caught and fixed the 3
  silent drops in commit `c5c8f6bf8`
- [#1362](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1362) — the issue
  that motivated the `NestedVariableList` addition
