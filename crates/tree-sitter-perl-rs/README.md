# tree-sitter-perl-rs

[![Crates.io](https://img.shields.io/crates/v/tree-sitter-perl-rs.svg)](https://crates.io/crates/tree-sitter-perl-rs)
[![Documentation](https://docs.rs/tree-sitter-perl-rs/badge.svg)](https://docs.rs/tree-sitter-perl-rs)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/EffortlessMetrics/perl-lsp)

Rust-native Perl parser with tree-sitter-style ergonomics and tree-sitter-compatible output.

## What it is

A facade over the v3 native recursive-descent Perl parser (`perl-parser-core`) that provides an
API surface matching the conventions of the `tree-sitter` crate. Users familiar with tree-sitter
can work with Perl ASTs immediately, while the underlying engine is the full-featured native v3
stack — not the C tree-sitter grammar.

This is NOT a set of C bindings. For the conventional C/FFI binding to the Perl tree-sitter
grammar, see [`tree-sitter-perl-c`](https://crates.io/crates/tree-sitter-perl-c).

## Quick start

```rust
use tree_sitter_perl_rs::Parser;

let mut parser = Parser::new();
if let Some(tree) = parser.parse("my $x = 42;") {
    let root = tree.root_node();
    println!("{}", root.to_sexp());
    // Output: (source_file (my_declaration (variable $ x)(number 42)))
}
```

## Key differences from `tree-sitter-perl-c`

| Aspect | `tree-sitter-perl-rs` | `tree-sitter-perl-c` |
|--------|-----------------------|----------------------|
| **Backing engine** | v3 native Rust parser | C tree-sitter grammar |
| **Binding type** | Facade (NOT bindings) | Conventional C/FFI bindings |
| **Error recovery** | Full v3 tolerance — partial tree on malformed input | Grammar-level only |
| **Output** | tree-sitter-compatible S-expressions | tree-sitter-compatible S-expressions |
| **Use when** | Rust-first Perl tooling, LSP/DAP integration | tree-sitter C ecosystem compatibility |

## API overview

| Type / Method | Description |
|---|---|
| `Parser::new()` | Create a parser instance |
| `Parser::parse(&mut self, source: &str) -> Option<Tree>` | Parse Perl source; `None` only on complete failure |
| `Tree::root_node() -> Node<'_>` | Get the root of the syntax tree |
| `Tree::source() -> &str` | Source text this tree was built from |
| `Parser::parse_with_old_tree(&mut self, source: &str, old_tree: &Tree) -> Option<Tree>` | Reparse with one validated pending edit using bounded token replay and fresh AST reconstruction; invalid, missing, or multiple edits use a recorded full-parse fallback |
| `Parser::parse_detailed(&mut self, source: &str) -> ParseOutcome` | Parse result with recovered-tree status, diagnostics, and typed catastrophic failure |
| `Tree::walk() -> TreeCursor<'_>` | Returns a cursor for zero-allocation streaming traversal |
| `Tree::edit(&mut self, edit: &InputEdit)` | Records a source edit; pass the updated tree to `parse_with_old_tree` |
| `Tree::diagnostics() -> &[ParseDiagnostic]` | Diagnostics collected while building the tree |
| `Tree::has_error() -> bool` | `true` for diagnostics or an error node anywhere in the tree |
| `Tree::reparse_mode() -> Option<ReparseMode>` | Reports unchanged reuse, token replay, or the typed full-parse fallback reason |
| `Tree::incremental_metrics() -> Option<&IncrementalMetrics>` | Reports tokens reused/re-lexed and bytes reprocessed; absent for initial and unchanged parses |
| `Tree::reprocessed_ranges() -> Vec<Range<usize>>` | Lexer work ranges from the most recent replay or fallback, not structural tree-difference ranges |
| `Node::kind() -> String` | Grammar-canonical node type name (e.g. `"source_file"`) matching tree-sitter output |
| `Node::native_kind() -> &'static str` | Native v3 internal node name (e.g. `"Program"`) |
| `Node::grammar_kind() -> String` | Compatibility alias of `kind()` |
| `Node::is_error() -> bool` / `Node::has_error() -> bool` | Detect an error node or an error descendant |
| `Node::to_sexp() -> String` | Tree-sitter-compatible S-expression for this subtree |
| `Node::child_count() -> usize` | Number of direct children |
| `Node::child(i: usize) -> Option<Node>` | `i`-th direct child |
| `Node::children() -> impl Iterator<Item = Node>` | Iterator over direct children |
| `Node::child_by_field_name(name: &str) -> Option<Node>` | First direct child for a canonical named field |
| `Node::children_by_field_name(name: &str) -> impl Iterator<Item = Node>` | All direct children for a repeated named field |
| `Node::field_name_for_child(i: usize) -> Option<&'static str>` | Canonical field name for a positional child |
| `Node::start_byte() -> usize` | Start byte offset in source (inclusive) |
| `Node::end_byte() -> usize` | End byte offset in source (exclusive) |
| `Node::start_position() -> Point` | `(row, column)` of the first byte |
| `Node::end_position() -> Point` | `(row, column)` past the last byte |
| `Node::utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, Utf8Error>` | Source slice for this node |
| `Node::tree_source() -> &str` | Source string the enclosing tree was built from |
| `Node::is_leaf() -> bool` | `true` if the node has no children |
| `Node::inner() -> &perl_ast::Node` | Escape hatch to the v3 AST |
| `TreeCursor` | Zero-allocation cursor; `node()`, `goto_first_child()`, `goto_next_sibling()`, `goto_parent()` |
| `InputEdit` | Source-edit descriptor (re-export of `perl_parser_core::edit::Edit`) |
| `PerlLanguage` / `language()` / `LANGUAGE` | Language descriptor for Rust-native tooling (not `tree_sitter::Language`) |
| `FieldId` | Stable named-field identifier shared by the AST and facade |
| `PerlNodeKind` | Re-export of `perl_ast::NodeKind` for pattern matching |
| `ParseOutcome` / `ParseFailure` / `ParseDiagnostic` | Detailed recovery and catastrophic-failure reporting |
| `ReparseMode` / `FallbackReason` / `IncrementalMetrics` | Explicit bounded-replay operation classification and measurements |
| `Query` / `QueryCursor` | Structural AST matching when the `queries` feature is enabled |

### Structural queries

Enable the optional `queries` feature for Phase 2a query support:

```toml
tree-sitter-perl-rs = { version = "...", features = ["queries"] }
```

The supported subset includes node kinds, wildcards, nested children, named fields,
captures, multiple top-level patterns, and byte-range restriction. Query predicates and
other unsupported tree-sitter query syntax return a typed `QueryError`; they are not
silently ignored.

### Incremental proof

The repository-native proof command measures the one-edit token-replay contract against
fresh parsing and writes a machine-readable receipt:

```text
cargo xtask tree-sitter-incremental-proof --profile pr
cargo xtask tree-sitter-incremental-proof --profile nightly
```

The receipt records exact checkout/toolchain/input identity, fresh and replay p50/p95
latency, bytes and tokens processed, fallback classification, and facade equivalence.
The command compares S-expressions, node kinds, fields, spans, points, source text,
diagnostics, and error status. The lower-tier `perl-parser-core` suite is the proof of
complete replayed token-stream equivalence. These measurements do not claim AST subtree
reuse or guarantee a replay speedup for every document size or edit class.

## Error tolerance

The v3 parser is highly error-tolerant. `Parser::parse()` returns `Option<Tree>`:
- `Some(tree)` — Almost always, even for malformed or incomplete input (partial tree produced).
- `None` — Only on extreme edge cases where no AST can be built at all.

This means you can pipe any Perl source through this parser and rely on getting a tree back.
Use `Parser::parse_detailed()` when the distinction between a clean tree, a recovered tree,
and a catastrophic failure matters. A recovered tree has `Some(tree)` plus diagnostics;
catastrophic recursion or nesting failures have `tree == None` and a typed `ParseFailure`.

## Known limitations (Phase 1)

- `Node::children()` allocates a `Vec` internally on each call. Prefer iterating once over calling repeatedly.
- `Parser::parse_with_old_tree()` reuses cached tokens only for one validated pending edit.
  The AST is rebuilt from the resulting token stream; this facade does not claim AST subtree
  reuse. Multiple, invalid, missing, oversized, context-sensitive, or unsafe edits fall back
  to a complete parse and expose the reason through `ReparseMode`.
- `Tree::reprocessed_ranges()` reports the lexer replay window. It is not a structural
  `changed_ranges()` API and must not be interpreted as a tree diff.
- `RecursionLimit` / `NestingTooDeep` parse errors produce `None` from `parse()` and a typed
  failure from `parse_detailed()` rather than a partial tree.
- `Node::kind()` now returns grammar-canonical names (e.g. `"source_file"`) for tree-sitter compatibility. Use `Node::native_kind()` when you need the v3 internal PascalCase name.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
