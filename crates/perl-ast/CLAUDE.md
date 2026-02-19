# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-ast` is a **Tier 1 leaf crate** providing AST (Abstract Syntax Tree) node definitions for the Perl parser ecosystem.

**Purpose**: Typed representation of Perl syntax constructs used by the parser, semantic analyzer, and LSP server.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-ast              # Build this crate
cargo test -p perl-ast               # Run tests
cargo clippy -p perl-ast             # Lint
cargo doc -p perl-ast --open         # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` -- Span/position types (`SourceLocation`, `Range`, `Position`)
- `perl-token` -- Token definitions (`Token`, `TokenKind`)

### Source Modules

| File | Purpose |
|------|---------|
| `lib.rs` | Re-exports `Node`, `NodeKind`, `SourceLocation` |
| `ast.rs` | Primary AST: `Node` struct (kind + location), `NodeKind` enum (50+ variants), S-expression output |
| `v2.rs` | Enhanced AST for incremental parsing: `Node` with `NodeId` + `Range`, `NodeIdGenerator`, `MissingKind`, `DiagnosticId` |

### Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `ast::Node` | `ast` | Primary AST node: `kind: NodeKind` + `location: SourceLocation` |
| `ast::NodeKind` | `ast` | Enum with 50+ variants (Program, Subroutine, If, Variable, FunctionCall, etc.) |
| `v2::Node` | `v2` | Enhanced node with `id: NodeId`, `kind: NodeKind`, `range: Range` |
| `v2::NodeKind` | `v2` | Subset of node kinds for incremental parsing |
| `v2::NodeIdGenerator` | `v2` | Sequential unique ID generator for v2 nodes |
| `v2::MissingKind` | `v2` | Enum for specific kinds of missing syntax (Expression, Block, Semicolon, etc.) |
| `v2::DiagnosticId` | `v2` | Type alias (`u32`) for lightweight error references |

### NodeKind Categories (ast module)

**Declarations**: `VariableDeclaration`, `VariableListDeclaration`, `Subroutine`, `Method`, `Package`, `Class`, `Format`
**Control flow**: `If`, `While`, `For`, `Foreach`, `Given`, `When`, `Default`, `StatementModifier`, `LabeledStatement`
**Expressions**: `Binary`, `Unary`, `Ternary`, `Assignment`, `FunctionCall`, `MethodCall`, `IndirectCall`
**Literals**: `Number`, `String`, `Heredoc`, `ArrayLiteral`, `HashLiteral`, `Regex`
**Variables**: `Variable`, `VariableWithAttributes`, `Typeglob`
**Modules**: `Use`, `No`, `PhaseBlock`, `DataSection`
**Error recovery**: `Error`, `MissingExpression`, `MissingStatement`, `MissingIdentifier`, `MissingBlock`, `UnknownRest`
**Other**: `Program`, `Block`, `ExpressionStatement`, `Return`, `LoopControl`, `Eval`, `Do`, `Try`, `Diamond`, `Ellipsis`, `Undef`, `Readline`, `Glob`, `Identifier`, `Prototype`, `Signature`, `MandatoryParameter`, `OptionalParameter`, `SlurpyParameter`, `NamedParameter`

## Usage

```rust
use perl_ast::{Node, NodeKind, SourceLocation};

// Construct a node
let loc = SourceLocation { start: 0, end: 10 };
let node = Node::new(
    NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
    loc,
);

// S-expression output
assert_eq!(node.to_sexp(), "(variable $ x)");

// Pattern match on kind
match &node.kind {
    NodeKind::Variable { sigil, name } => { /* ... */ }
    _ => {}
}
```

## Important Notes

- `ast::Node` is a concrete struct, not a trait -- work with it via pattern matching on `NodeKind`
- `Node::to_sexp()` produces tree-sitter-compatible S-expressions for test comparison
- `NodeKind::kind_name()` returns a static string name; `NodeKind::ALL_KIND_NAMES` lists all names
- Adding new `NodeKind` variants requires updating `to_sexp()`, `to_sexp_inner()`, `kind_name()`, `ALL_KIND_NAMES`, and the `visit_children()` method
- Dependents: `perl-parser-core`, `perl-tokenizer`, `perl-pragma`, `perl-error`
