# tree-sitter-perl-rs — Roadmap

## Phase 1 (shipped)

- `Parser` / `Tree` / `Node<'tree>` types with tree-sitter-compatible API shape
- `to_sexp()` — tree-sitter-compatible S-expression output
- `kind()`, `child_count()`, `child()`, `children()` — tree traversal
- `start_byte()`, `end_byte()`, `utf8_text()` — source location and extraction
- `is_leaf()`, `inner()`, `tree_source()` — utility and escape hatch
- `PerlNodeKind` re-export for pattern matching without a direct `perl-ast` dependency
- Snapshot tests for representative Perl constructs

## Phase 2 (planned)

### Tree cursor / walk API

A `TreeCursor` type for streaming traversal without per-call Vec allocation.
Mirrors `tree_sitter::TreeCursor`.

### Edit / incremental parsing

`Tree::edit()` to apply `InputEdit` structures and `Parser::parse_with_old_tree()` for
incremental re-parsing of changed source regions.

### Field-name accessors

`Node::child_by_field_name(name: &str) -> Option<Node>` and
`Node::children_by_field_name(name: &str) -> impl Iterator<Item = Node>` to address named
child slots (e.g. `"body"`, `"condition"`, `"name"`).

### `Language` constant

A `LANGUAGE` constant or `language()` function returning a type compatible with
`tree_sitter::Language` (if API stability permits), enabling use with tree-sitter tooling
that expects a language object.

### Predicate / query API

`Query` and `QueryCursor` types for pattern matching over the AST, analogous to the
tree-sitter query API.

### `kind()` name remapping

Map v3 internal node kind names (e.g. `"Program"`, `"Subroutine"`) to canonical tree-sitter
grammar names (e.g. `"source_file"`, `"subroutine_declaration"`). The current `kind()` returns
v3 internal names; `to_sexp()` already uses grammar-canonical names.

## Known limitations

- `end_byte()` may return `source.len() + 1` for the root node on some inputs. Callers should
  clamp to `source.len()` when using it as a slice index.
- `Node::children()` allocates a `Vec<&AstNode>` internally on each call. Avoid calling it
  in tight loops; iterate once and collect if you need random access.
- `RecursionLimit` / `NestingTooDeep` parse errors from the v3 parser produce `None` from
  `Parser::parse()` rather than a partial tree. In practice this only affects pathologically
  deep nesting.
- `Node::kind()` returns v3 internal kind names, not tree-sitter grammar node type strings.
  The root node reports `"Program"` rather than `"source_file"`. Use `to_sexp()` for output
  that uses canonical grammar names.
