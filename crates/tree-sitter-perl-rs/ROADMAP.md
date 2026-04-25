# tree-sitter-perl-rs — Roadmap

## Phase 1 (shipped)

- `Parser` / `Tree` / `Node<'tree>` types with tree-sitter-compatible API shape
- `to_sexp()` — tree-sitter-compatible S-expression output
- `kind()`, `child_count()`, `child()`, `children()` — tree traversal
- `start_byte()`, `end_byte()`, `utf8_text()` — source location and extraction
- `is_leaf()`, `inner()`, `tree_source()` — utility and escape hatch
- `PerlNodeKind` re-export for pattern matching without a direct `perl-ast` dependency
- Snapshot tests for representative Perl constructs
- `TreeCursor` / `walk()` — streaming traversal without per-call Vec allocation,
  mirroring `tree_sitter::TreeCursor`
- `Tree::edit()` and `Parser::parse_with_old_tree()` — incremental re-parsing of
  changed source regions
- `PerlLanguage` — language descriptor with `node_kind_count()`, `node_kind_names()`,
  and `node_kind_is_named()`, compatible with tree-sitter language metadata conventions
- `grammar_kind()` — returns canonical tree-sitter grammar names (e.g. `"source_file"`,
  `"sub"`) rather than v3 internal names; complements `kind()`

## Phase 2 (planned)

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

## Known limitations

- `Node::children()` allocates a `Vec<&AstNode>` internally on each call. Avoid calling it
  in tight loops; iterate once and collect if you need random access. Use `Tree::walk()` /
  `TreeCursor` for allocation-free traversal.
- `RecursionLimit` / `NestingTooDeep` parse errors from the v3 parser produce `None` from
  `Parser::parse()` rather than a partial tree. In practice this only affects pathologically
  deep nesting.
- `Node::kind()` returns v3 internal kind names, not tree-sitter grammar node type strings.
  The root node reports `"Program"` rather than `"source_file"`. Use `grammar_kind()` to get
  the canonical tree-sitter name, or `to_sexp()` for output that uses grammar names throughout.
