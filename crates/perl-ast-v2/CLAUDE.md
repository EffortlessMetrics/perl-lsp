# CLAUDE.md (perl-ast-v2)

## Role

Second-generation AST with full position-range tracking, designed for
incremental parsing and richer error reporting than the primary `perl-ast`
node type. Re-exported by `perl-ast` as its `v2` module.

## Owns

- `Node` -- enhanced AST node: `id: NodeId`, `kind: NodeKind`, `range: Range`
  (from `perl-position-tracking`), plus `to_sexp()`.
- `NodeKind` -- node-kind enum for incremental parsing (a distinct set from
  `perl-ast::ast::NodeKind`, not a re-export of it).
- `NodeId` (`usize` alias) and `NodeIdGenerator` -- sequential unique ID
  allocation for incremental re-parses.
- `MissingKind` -- specific missing-syntax categories for error recovery
  (`Expression`, `Statement`, `Identifier`, `Block`, `ClosingDelimiter(char)`,
  `Semicolon`, `Condition`, `Argument`, `Operator`).
- `DiagnosticId` (`u32` alias) -- lightweight index into a diagnostics array
  stored separately from the tree, decoupling structure from message text.

## Does not own

- The primary/stable AST (`perl-ast::ast::Node` / `NodeKind`) -- that's a
  separate, older node representation in a different crate; this crate does
  not extend or wrap it.
- Parsing itself -- this crate defines node shapes only; `perl-parser-core`
  and `perl-lexer` build trees of these nodes.

## Neighbors

- Upstream: `perl-position-tracking` (only dependency, for `Range`).
- Downstream: `perl-ast` (re-exports this crate as its `v2` module),
  `perl-lexer`, `perl-parser-core`.

## Read first

- `src/lib.rs` -- the entire crate; one file holding `Node`, `NodeKind`,
  `MissingKind`, `NodeIdGenerator`.
- `crates/perl-ast/src/lib.rs` doc comment describing `v2` as "experimental
  second-generation AST re-exported from `perl-ast-v2`" -- read this to
  understand the stability expectation relative to the primary AST.

## Focused validation

`cargo test -p perl-ast-v2`. `tests/node_kinds.rs` covers node-kind
construction/coverage; `tests/sexp_branch_coverage.rs` covers `to_sexp()`
output across `NodeKind` variants.

## Review hotspots

Adding a `NodeKind` variant here is independent of adding one to
`perl-ast::ast::NodeKind` -- the two enums are not kept in lockstep
automatically; a change intended for "the AST" needs to be checked against
both call sites if the feature should apply to both node generations.

## Claim boundary

Describes the node/type shapes as authored. Does not assert that
incremental-parsing consumers (beyond the current dependents) exist yet --
`v2` is explicitly documented upstream as experimental.
