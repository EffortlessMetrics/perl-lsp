# CLAUDE.md — tree-sitter-perl-rs

## Tier

Tier 2+ (depends on `perl-parser-core` Tier 2 + `perl-ast` Tier 1b).

## Purpose

`tree-sitter-perl-rs` is the **Rust-native Perl parser with tree-sitter-style ergonomics
and tree-sitter-compatible output**. It is a facade over the v3 recursive-descent native
parser (`perl-parser-core`). It is NOT bindings to the C tree-sitter grammar.

This crate is part of the **Perl tooling platform** positioning:
- The native Rust parser/lexer/analysis stack is the center of gravity.
- This crate provides the tree-sitter interoperability surface — a first-class product,
  not a compatibility shim.
- Users coming from tree-sitter get a familiar API while benefiting from the full v3
  parser capabilities (error recovery, incremental support, semantic analysis).

## Public API surface

```rust
pub struct Parser { /* wraps perl-parser-core */ }
impl Parser {
    pub fn new() -> Self;
    pub fn parse(&mut self, source: &str) -> Option<Tree>;
}

pub struct Tree { /* owns the v3 AST */ }
impl Tree {
    pub fn root_node(&self) -> Node;
    pub fn source(&self) -> &str;
    pub fn edit(&mut self, edit: &InputEdit);
    pub fn walk(&self) -> TreeCursor<'_>;
}

pub struct Node<'tree> { /* borrows from Tree */ }
impl<'tree> Node<'tree> {
    pub fn kind(&self) -> &'static str;        // v3 internal name (e.g. "Program")
    pub fn grammar_kind(&self) -> String;      // tree-sitter grammar name (e.g. "source_file")
    pub fn to_sexp(&self) -> String;           // delegates to perl_ast::Node::to_sexp()
    pub fn child_count(&self) -> usize;
    pub fn child(&self, i: usize) -> Option<Node<'tree>>;
    pub fn children(&self) -> impl Iterator<Item = Node<'tree>>;
    pub fn walk(&self) -> TreeCursor<'tree>;
    pub fn start_byte(&self) -> usize;
    pub fn end_byte(&self) -> usize;
    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, Utf8Error>;
    pub fn is_leaf(&self) -> bool;
    pub fn inner(&self) -> &'tree perl_ast::Node;  // escape hatch
}

pub struct PerlLanguage { /* node kind descriptor, NOT tree_sitter::Language */ }
impl PerlLanguage {
    pub fn node_kind_count(&self) -> usize;
    pub fn node_kind_names(&self) -> &[&'static str];
    pub fn node_kind_is_named(&self, kind: &str) -> bool;
}

pub fn language() -> PerlLanguage;
pub static LANGUAGE: PerlLanguage;

pub use perl_ast::NodeKind as PerlNodeKind;  // for pattern matching without perl-ast dep
```

## How it differs from `tree-sitter-perl-c`

| | `tree-sitter-perl-rs` | `tree-sitter-perl-c` |
|---|---|---|
| Backing engine | v3 native Rust parser | C tree-sitter grammar |
| Binding type | **NOT bindings** — facade | Conventional C bindings |
| Error recovery | Full v3 tolerance | Grammar-level |
| Use when | Rust-first Perl tooling | tree-sitter C ecosystem compat |

## Workspace inheritance

Version, edition, rust-version, license, authors, repository, and homepage are all
inherited from `[workspace.package]` in the root `Cargo.toml`.

## Commands

```bash
cargo build -p tree-sitter-perl-rs          # Build
cargo test -p tree-sitter-perl-rs           # Run all tests
INSTA_UPDATE=always cargo test -p tree-sitter-perl-rs --test snapshots  # Accept snapshots
cargo clippy -p tree-sitter-perl-rs         # Lint
cargo doc -p tree-sitter-perl-rs --open     # View documentation
```

## Backlog follow-ups

- Field-name accessors (named children by field name, as in `node.child_by_field_name("body")`)
- Predicate / query API (pattern matching over the AST)
- True incremental re-parsing (skipping unchanged regions in `Parser::parse_with_old_tree`)
