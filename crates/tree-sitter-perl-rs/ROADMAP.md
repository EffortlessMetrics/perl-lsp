# tree-sitter-perl-rs — Roadmap

## Phase 1 (shipped)

- `Parser` / `Tree` / `Node<'tree>` types with tree-sitter-compatible API shape
- `to_sexp()` — tree-sitter-compatible S-expression output
- `kind()`, `native_kind()`, `grammar_kind()`, `child_count()`, `child()`, `children()` — tree traversal
- `start_byte()`, `end_byte()`, `start_position()`, `end_position()`, `utf8_text()` — source location and extraction
- `is_leaf()`, `inner()`, `tree_source()` — utility and escape hatch
- `TreeCursor` — zero-allocation streaming traversal (`walk()`, `goto_first_child()`, `goto_next_sibling()`, `goto_parent()`)
- `Tree::edit()` / `Parser::parse_with_old_tree()` / `InputEdit` — compatibility edit journal
  with an unchanged-source fast path; changed source is currently fully reparsed
- `Parser::parse_detailed()` / `Tree::diagnostics()` / `Tree::has_error()` / `Node::is_error()` /
  `Node::has_error()` — recovery and catastrophic-failure observability
- `PerlLanguage` descriptor, `language()` function, and `LANGUAGE` constant for Rust-native tooling
- `PerlNodeKind` re-export for pattern matching without a direct `perl-ast` dependency
- Canonical named-field access through `child_by_field_name()`,
  `children_by_field_name()`, and `field_name_for_child()`
- Snapshot tests for representative Perl constructs

## Phase 2

### Structural query subset (shipped behind `queries`)

`Query` and `QueryCursor` support node kinds, wildcards, nested children, named fields,
captures, multiple top-level patterns, and byte-range restriction. Unsupported syntax
returns a typed `QueryError`.

### Query predicates (planned)

Add predicates as required by real repository `.scm` fixtures, with explicit conformance
coverage for every supported predicate.

## Known limitations

- `end_byte()` is clamped to the tree source length for safe slice use.
- `Node::children()` allocates a `Vec<&AstNode>` internally on each call. Avoid calling it
  in tight loops; iterate once and collect if you need random access.
- `RecursionLimit` / `NestingTooDeep` parse errors from the v3 parser produce `None` from
  `Parser::parse()` and a typed failure from `Parser::parse_detailed()` rather than a partial
  tree. In practice this only affects pathologically deep nesting.
- `Node::kind()` returns grammar-canonical tree-sitter node type strings such as
  `"source_file"`. Use `Node::native_kind()` when callers need v3 internal kind names
  such as `"Program"`.
