# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-parser-core` is the **core parsing engine** providing the fundamental parsing machinery. It is a Tier 2 foundational crate used by `perl-parser` and higher-level analysis crates.

**Purpose**: Core parser engine for perl-parser — provides AST construction, statement/expression parsing, and error recovery.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-parser-core            # Build this crate
cargo test -p perl-parser-core             # Run tests
cargo clippy -p perl-parser-core           # Lint
cargo doc -p perl-parser-core --open       # View documentation
```

## Architecture

### Dependencies

**Internal (Tier 1 leaf crates)**:
- `perl-lexer` - Tokenization
- `perl-token` - Token definitions
- `perl-position-tracking` - UTF-8/UTF-16 positions
- `perl-ast` - AST node definitions
- `perl-quote` - Quote operator handling
- `perl-pragma` - Pragma validation
- `perl-edit` - Edit operations
- `perl-builtins` - Built-in function metadata
- `perl-regex` - Regex operator handling
- `perl-heredoc` - Heredoc processing
- `perl-error` - Error types
- `perl-tokenizer` - Token stream utilities

### Main Modules

| Path | Purpose |
|------|---------|
| `lib.rs` | Core engine interface |
| `engine/` | Parsing logic subdirectory |
| `tokens/` | Token stream handling |

### Key Responsibilities

1. **Recursive Descent Parsing**: Implements the v3 native parser
2. **AST Construction**: Builds typed AST nodes from token streams
3. **Error Recovery**: Provides graceful handling of malformed input
4. **Position Tracking**: Maintains byte and UTF-16 positions

## Usage

This crate is typically used indirectly through `perl-parser`. Direct usage:

```rust
use perl_parser_core::Engine;

let source = "my $x = 1;";
let engine = Engine::new(source);
let ast = engine.parse()?;
```

## Design Patterns

### Error Recovery

The parser uses continuation-based error recovery to provide useful diagnostics even for incomplete or malformed code:

```rust
// Parser continues past errors, collecting diagnostics
let result = engine.parse();
for diagnostic in result.diagnostics() {
    // Handle parser errors
}
```

### Quote Context

Special handling for Perl's complex quoting:

```rust
// Handles q//, qq//, qw//, qr//, etc.
// Delegates to perl-quote for delimiter matching
```

## Important Notes

- Prefer using `perl-parser` for typical use cases
- This crate provides the low-level parsing API
- Changes here affect all higher-level crates
