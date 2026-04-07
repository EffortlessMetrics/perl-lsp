# tree-sitter-perl-rs

Rust-native Perl parser with tree-sitter-style ergonomics and tree-sitter-compatible output.

## What it is

A facade over the v3 native recursive-descent Perl parser (`perl-parser-core`) that provides an API surface matching the conventions of the `tree-sitter` crate. Users familiar with tree-sitter can work with Perl ASTs immediately, while the underlying engine is the full-featured native v3 stack—not the C tree-sitter grammar.

## Key differences from the C tree-sitter grammar

| Aspect | tree-sitter-perl-rs | tree-sitter-perl-c |
|--------|---------------------|-------------------|
| **Backing engine** | v3 native Rust parser | C tree-sitter grammar |
| **Binding type** | Facade (NOT bindings) | Conventional C bindings |
| **Error recovery** | Full v3 tolerance | Grammar-level only |
| **Use when** | Rust-first Perl tooling | tree-sitter C ecosystem compatibility |

## API surface

```rust
use tree_sitter_perl_rs::Parser;

let mut parser = Parser::new();
if let Some(tree) = parser.parse("my $x = 42;") {
    let root = tree.root_node();
    println!("{}", root.to_sexp());
    // Output: (source_file (my_declaration (variable $ x) (number 42)))
}
```

Core types:
- `Parser::new() -> Self` — Create a parser instance
- `Parser::parse(&mut self, source: &str) -> Option<Tree>` — Parse Perl source
- `Tree::root_node() -> Node<'_>` — Get the root of the syntax tree
- `Node::kind() -> &'static str` — Node type name (e.g., "Program", "Subroutine")
- `Node::to_sexp() -> String` — Tree-sitter-compatible S-expression
- `Node::child_count()`, `Node::child()`, `Node::children()` — Tree traversal
- `Node::start_byte()`, `Node::end_byte()`, `Node::utf8_text()` — Source location
- `Node::inner() -> &perl_ast::Node` — Escape hatch to the v3 AST

## Error tolerance

The v3 parser is highly error-tolerant. `Parser::parse()` returns `Option<Tree>`:
- `Some(tree)` — Almost always, even for malformed input (partial tree produced)
- `None` — Only on extreme edge cases where no AST can be built at all

## Workspace role

Part of the **Perl tooling platform**, providing the tree-sitter interoperability surface for users migrating from the C grammar or building Perl-adjacent tools.

## License

MIT OR Apache-2.0
