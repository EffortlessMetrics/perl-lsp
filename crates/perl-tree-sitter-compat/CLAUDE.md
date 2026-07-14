# CLAUDE.md (perl-tree-sitter-compat)

## Role

Tree-sitter-compatible output over the native Perl parser. An **adapter, not
a re-implementation**: it projects the native recursive-descent parser's AST
into tree-sitter's shapes (named nodes with kinds, byte/point ranges,
S-expression rendering, highlight captures) so editors and tooling built for
the tree-sitter ecosystem can consume the native parser's output without a
separate grammar.

## Owns

- `convert` -- `parse_to_tree`, `to_ts_node`, `TreeError`: builds the
  tree-sitter-shaped tree from native parser output.
- `node` -- `TsNode`, `TsPoint`, `pascal_to_snake` (native `NodeKind` names
  are PascalCase; tree-sitter node-kind names are snake_case).
- `highlight` -- `Highlight`, `highlights`, `capture_for`: node-granular
  highlight capture mapping.
- `sexp` -- `to_sexp`, `to_sexp_pretty`: S-expression rendering compatible
  with tree-sitter's format.

## Does not own

- Parsing itself -- depends on `perl-parser-core` for the actual AST; this
  crate only re-shapes it.
- UTF-16 position handling or workspace-wide facts -- uses
  `perl-workspace-core` only for its UTF-8 line index.
- Full tree-sitter fidelity: per the crate's documented scope, only named
  nodes are surfaced (no anonymous token/punctuation nodes), and
  token-precise highlighting plus locals/injection capture queries are
  documented follow-ups, not current behavior.

## Neighbors

- Upstream: `perl-parser-core`, `perl-workspace-core`, `serde`.
- Downstream: none in-workspace yet -- this crate sits at the same
  substrate layer as other `perl-workspace-core` consumers but has no
  current in-tree callers.

## Read first

- `src/lib.rs` -- full architecture doc comment, including the explicit
  "Scope" section describing what's deliberately not implemented yet.
- `docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md` (the
  section covering this adapter) for why it exists instead of a separate
  tree-sitter grammar.

## Focused validation

`cargo test -p perl-tree-sitter-compat` -- see `tests/adapter.rs` for the
end-to-end parse -> tree -> S-expression / highlight flow.

## Review hotspots

`node::pascal_to_snake` -- the PascalCase-to-snake_case node-kind name
mapping is the seam most likely to silently diverge from real tree-sitter
grammars' naming conventions when new `NodeKind` variants are added
upstream in `perl-parser-core`/`perl-ast`.

## Claim boundary

Describes the adapter's current scope as authored, including its explicit
non-goals (anonymous nodes, token-precise highlighting, injection queries).
Does not assert compatibility with any specific tree-sitter-perl grammar
version beyond the named-node/S-expression/highlight-capture surface this
crate implements.
