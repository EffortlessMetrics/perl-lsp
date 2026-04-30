# tree-sitter-perl-rs — Roadmap

## Phase 1 (shipped)

- `Parser` / `Tree` / `Node<'tree>` types with tree-sitter-compatible API shape
- `to_sexp()` — tree-sitter-compatible S-expression output
- `kind()`, `grammar_kind()`, `child_count()`, `child()`, `children()` — tree traversal
- `start_byte()`, `end_byte()`, `start_position()`, `end_position()`, `utf8_text()` — source location and extraction
- `is_leaf()`, `inner()`, `tree_source()` — utility and escape hatch
- `TreeCursor` — zero-allocation streaming traversal (`walk()`, `goto_first_child()`, `goto_next_sibling()`, `goto_parent()`)
- `Tree::edit()` / `Parser::parse_with_old_tree()` / `InputEdit` — incremental re-parsing
- `PerlLanguage` descriptor, `language()` function, and `LANGUAGE` constant for Rust-native tooling
- `PerlNodeKind` re-export for pattern matching without a direct `perl-ast` dependency
- Snapshot tests for representative Perl constructs

## Phase 2 (planned)

### Field-name accessors

`Node::child_by_field_name(name: &str) -> Option<Node>` and
`Node::children_by_field_name(name: &str) -> impl Iterator<Item = Node>` to address named
child slots (e.g. `"body"`, `"condition"`, `"name"`).

### Predicate / query API

`Query` and `QueryCursor` types for pattern matching over the AST, analogous to the
tree-sitter query API.

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
