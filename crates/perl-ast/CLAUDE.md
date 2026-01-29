# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-ast` is a **Tier 1 leaf crate** providing AST (Abstract Syntax Tree) node definitions for the Perl parser.

**Purpose**: AST definitions for Perl parser — typed representation of Perl syntax constructs.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-ast              # Build this crate
cargo test -p perl-ast               # Run tests
cargo clippy -p perl-ast             # Lint
cargo doc -p perl-ast --open         # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` - Span/position types
- `perl-token` - Token definitions

### Key Types

| Type | Purpose |
|------|---------|
| `Node` | Base AST node trait |
| `Statement` | Statement-level constructs |
| `Expression` | Expression-level constructs |
| `Block` | Code blocks |
| `Subroutine` | Subroutine definitions |
| `Package` | Package declarations |

### Node Categories

**Statements**:
- Variable declarations (`my`, `our`, `local`, `state`)
- Control flow (`if`, `unless`, `while`, `for`, `foreach`)
- Subroutine definitions
- Package/module declarations
- Use/require statements

**Expressions**:
- Literals (numbers, strings, regex)
- Variables (scalars, arrays, hashes)
- Operators (binary, unary, ternary)
- Function calls
- Method calls
- Anonymous subs/closures

## Usage

```rust
use perl_ast::{Node, Statement, Expression};

fn visit_node(node: &dyn Node) {
    // Common node operations
    let span = node.span();
    let children = node.children();
}
```

## Important Notes

- AST nodes should be immutable after construction
- Each node type includes position information for LSP features
- Adding new node types requires updating visitors in dependent crates
