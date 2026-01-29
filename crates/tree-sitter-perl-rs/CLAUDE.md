# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`tree-sitter-perl-rs` is a **Tier 7 Rust-native Tree-sitter crate** providing Tree-sitter Perl grammar with Rust scanner.

**Purpose**: Tree-sitter Perl grammar with Rust scanner — experimental pure-Rust Tree-sitter implementation.

**Version**: See Cargo.toml

## Commands

```bash
cargo build -p tree-sitter-perl-rs       # Build this crate
cargo test -p tree-sitter-perl-rs        # Run tests
cargo clippy -p tree-sitter-perl-rs      # Lint
cargo doc -p tree-sitter-perl-rs --open  # View documentation
```

## Architecture

### Key Difference from C Version

This crate uses a **Rust scanner** instead of C:

| Aspect | tree-sitter-perl-c | tree-sitter-perl-rs |
|--------|-------------------|---------------------|
| Scanner | C (scanner.c) | Rust |
| Build | Requires C compiler | Pure Rust |
| FFI | C bindings | Native Rust |
| Portability | Platform-dependent | Cross-platform |

### Scanner Implementation

The Rust scanner handles:

- Heredoc parsing and terminator matching
- Quote-like operators with balanced delimiters
- Regex delimiters and modifiers
- Context-sensitive tokenization

## Usage

```rust
use tree_sitter_perl_rs::{language, Parser};

let mut parser = Parser::new();
parser.set_language(language())?;

let source = "my $x = 42;";
let tree = parser.parse(source, None)?;
```

### Comparison with Native Parser

```rust
// Tree-sitter parse
let ts_tree = ts_parser.parse(source, None)?;

// Native v3 parse
let native_ast = perl_parser::parse(source)?;

// Compare structure
assert_tree_compatible(&ts_tree, &native_ast);
```

## Important Notes

- Experimental — may not cover all edge cases
- Pure Rust build — no C toolchain required
- Used for Tree-sitter compatibility validation
- Scanner implementation mirrors perl-lexer behavior
