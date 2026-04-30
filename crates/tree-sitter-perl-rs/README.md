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
| `Parser::parse_with_old_tree(&mut self, source: &str, old_tree: &Tree) -> Option<Tree>` | Incremental re-parse; reuses unchanged regions of `old_tree` |
| `Tree::walk() -> TreeCursor<'_>` | Returns a cursor for zero-allocation streaming traversal |
| `Tree::edit(&mut self, edit: &InputEdit)` | Records a source edit; pass the updated tree to `parse_with_old_tree` |
| `Tree::root_node() -> Node<'_>` | Get the root of the syntax tree |
| `Tree::source() -> &str` | Source text this tree was built from |
| `Node::kind() -> &'static str` | Node type name (v3 internal, e.g. `"Program"`) |
| `Node::grammar_kind() -> String` | Grammar-canonical name (e.g. `"source_file"`) matching tree-sitter output |
| `Node::to_sexp() -> String` | Tree-sitter-compatible S-expression for this subtree |
| `Node::child_count() -> usize` | Number of direct children |
| `Node::child(i: usize) -> Option<Node>` | `i`-th direct child |
| `Node::children() -> impl Iterator<Item = Node>` | Iterator over direct children |
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
| `PerlNodeKind` | Re-export of `perl_ast::NodeKind` for pattern matching |

## Error tolerance

The v3 parser is highly error-tolerant. `Parser::parse()` returns `Option<Tree>`:
- `Some(tree)` — Almost always, even for malformed or incomplete input (partial tree produced).
- `None` — Only on extreme edge cases where no AST can be built at all.

This means you can pipe any Perl source through this parser and rely on getting a tree back.

## Known limitations (Phase 1)

- `Node::children()` allocates a `Vec` internally on each call. Prefer iterating once over calling repeatedly.
- `RecursionLimit` / `NestingTooDeep` parse errors produce `None` rather than a partial tree.
- `Node::kind()` returns v3 internal names (e.g. `"Program"`) rather than tree-sitter grammar names (e.g. `"source_file"`). Use `Node::grammar_kind()` for the canonical name or `Node::to_sexp()` for full canonical output.

## Backlog roadmap

The following APIs are not yet implemented and remain on the backlog:

- Field-name accessors (named children by field name, as in `node.child_by_field_name("body")`)
- Predicate / query API (pattern matching over the AST)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
