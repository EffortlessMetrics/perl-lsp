# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lexer` is a **Tier 1 leaf crate** providing context-aware tokenization for Perl source code.

**Purpose**: Mode-based Perl lexer that disambiguates context-sensitive tokens (division vs regex, modulo vs hash sigil, heredocs, quote-like operators) and supports checkpointing for incremental parsing.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lexer            # Build this crate
cargo test -p perl-lexer             # Run tests
cargo clippy -p perl-lexer           # Lint
cargo doc -p perl-lexer --open       # View documentation
cargo bench -p perl-lexer            # Run lexer benchmarks
cargo test -p perl-lexer --features slow_tests  # Include slow stress tests
```

## Architecture

### Dependencies

- `perl-position-tracking` (workspace) - Line/column position tracking
- `unicode-ident` - Unicode XID identifier validation
- `memchr` - Fast byte scanning for delimiters
- `thiserror` - Error type definitions

### Key Types (public API)

| Type | Module | Purpose |
|------|--------|---------|
| `PerlLexer<'a>` | `lib.rs` | Main lexer struct; call `next_token()` to iterate |
| `Token` | `token.rs` | Token with `token_type`, `text`, `start`, `end` |
| `TokenType` | `token.rs` | Enum of all token kinds (operators, keywords, literals, etc.) |
| `StringPart` | `token.rs` | Parts of interpolated strings (Literal, Variable, Expression) |
| `LexerMode` | `mode.rs` | ExpectTerm, ExpectOperator, ExpectDelimiter, InFormatBody, InDataSection |
| `LexerConfig` | `lib.rs` | Configuration: `parse_interpolation`, `track_positions`, `max_lookahead` |
| `LexerCheckpoint` | `checkpoint.rs` | Saved lexer state for backtracking |
| `CheckpointCache` | `checkpoint.rs` | Cache of checkpoints for incremental parsing |
| `Checkpointable` | `checkpoint.rs` | Trait: `checkpoint()`, `restore()`, `can_restore()` |
| `LexerError` | `error.rs` | Error variants (UnterminatedString, UnterminatedRegex, etc.) |

### Modules

| File | Purpose |
|------|---------|
| `lib.rs` (~3K lines) | Main lexer: `PerlLexer`, mode transitions, all token-parsing methods |
| `token.rs` | `Token`, `TokenType`, `StringPart` definitions |
| `mode.rs` | `LexerMode` enum and context-sensitivity documentation |
| `checkpoint.rs` | `LexerCheckpoint`, `CheckpointCache`, `Checkpointable` trait |
| `quote_handler.rs` | Quote-operator helpers (delimiter pairing, modifier specs) |
| `unicode.rs` | Unicode identifier classification (`is_perl_identifier_start/continue`) |
| `error.rs` | `LexerError` enum and `Result` alias |

### Budget Limits

Guards against pathological input; emits `UnknownRest` token on overflow:

- `MAX_REGEX_BYTES`: 64 KB
- `MAX_HEREDOC_BYTES`: 256 KB
- `MAX_DELIM_NEST`: 128 levels
- `MAX_HEREDOC_DEPTH`: 100 levels
- `HEREDOC_TIMEOUT_MS`: 5 seconds

## Usage Examples

### Basic tokenization

```rust
use perl_lexer::{PerlLexer, TokenType};

let mut lexer = PerlLexer::new("my $x = 42;");
while let Some(token) = lexer.next_token() {
    if matches!(token.token_type, TokenType::EOF) { break; }
    println!("{:?}: {}", token.token_type, token.text);
}
```

### Custom configuration

```rust
use perl_lexer::{PerlLexer, LexerConfig};

let config = LexerConfig {
    parse_interpolation: true,
    track_positions: true,
    max_lookahead: 1024,
};
let mut lexer = PerlLexer::with_config("my $x = 1;", config);
```

### Checkpointing

```rust
use perl_lexer::{PerlLexer, Checkpointable};

let mut lexer = PerlLexer::new("my $x = 1;");
let cp = lexer.checkpoint();
let _ = lexer.next_token();
lexer.restore(&cp); // backtrack
```

## Important Notes

- This crate is the foundation for `perl-parser` and the LSP stack; changes are high-impact
- `lib.rs` is intentionally large (~3K lines) to keep hot paths in a single compilation unit
- `quote_handler.rs` is `pub(crate)` only; modifier validation lives in the parser layer
- Test thoroughly when modifying quote, heredoc, or mode-transition logic
- The `simd` feature flag is defined but currently a no-op placeholder
