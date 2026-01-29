# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lexer` is a **Tier 1 leaf crate** providing high-performance, context-aware tokenization for Perl source code.

**Purpose**: High-performance Perl lexer with context-aware tokenization — handles Perl's complex lexical grammar including sigils, quotes, and heredocs.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lexer            # Build this crate
cargo test -p perl-lexer             # Run tests
cargo clippy -p perl-lexer           # Lint
cargo doc -p perl-lexer --open       # View documentation
```

## Architecture

### Dependencies

**External only** (plus `perl-position-tracking`):
- `unicode-ident` - Unicode identifier validation
- `memchr` - Fast byte scanning
- `thiserror` - Error definitions

### Main Modules

| File | Size | Purpose |
|------|------|---------|
| `lib.rs` | 118KB | Main lexer implementation |
| `checkpoint.rs` | - | Lexer state checkpoints for backtracking |
| `mode.rs` | - | Lexer mode management (string, regex, etc.) |
| `quote_handler.rs` | - | Quote-aware tokenization |
| `unicode.rs` | - | Unicode handling |
| `token.rs` | - | Token definitions |
| `error.rs` | - | Lexer error types |

### Lexer Modes

The lexer operates in different modes to handle Perl's context-sensitive grammar:

| Mode | Purpose |
|------|---------|
| Normal | Standard Perl code |
| String | Inside string literals |
| Regex | Inside regex patterns |
| Quote | Inside q/qq/qw operators |
| Heredoc | Inside heredoc content |

## Key Features

### Context-Aware Tokenization

```rust
use perl_lexer::Lexer;

let source = r#"my $x = "hello $name";"#;
let mut lexer = Lexer::new(source);

// Lexer handles interpolation context automatically
for token in lexer {
    // Token includes context information
}
```

### Checkpointing

```rust
// Save lexer state for potential backtracking
let checkpoint = lexer.checkpoint();

// Try parsing something
if !successful {
    lexer.restore(checkpoint);
}
```

### Unicode Support

Full Unicode identifier support following Perl's rules:
- Unicode letters for identifier starts
- Unicode combining marks and digits for continuations

## Performance Considerations

- Uses `memchr` for fast delimiter scanning
- Minimal allocations during tokenization
- Checkpoint-based backtracking (no full rescan)

## Important Notes

- This is the foundation for all parsing — changes here are high-impact
- The `lib.rs` is intentionally large (118KB) to keep hot paths together
- Test thoroughly when modifying quote or heredoc handling
