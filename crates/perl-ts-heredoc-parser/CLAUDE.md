# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-ts-heredoc-parser` is an **internal microcrate** providing the heredoc parsing pipeline and context-aware Perl lexer.

**Purpose**: Multi-phase heredoc processing (detect, collect, integrate), slash token disambiguation (division vs regex vs substitution vs transliteration), dynamic delimiter recovery, and a lexer adapter that rewrites ambiguous tokens for PEG grammar compatibility.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-ts-heredoc-parser          # Build this crate
cargo test -p perl-ts-heredoc-parser           # Run tests
cargo clippy -p perl-ts-heredoc-parser         # Lint
cargo doc -p perl-ts-heredoc-parser --open     # View documentation
```

## Architecture

### Dependencies

- `perl-ts-heredoc-analysis` -- statement tracking (`StatementTracker`, `find_statement_end_line`) and dynamic delimiter recovery
- `perl-parser-pest` -- `AstNode` type used by `LexerAdapter::postprocess`
- `regex` -- pattern matching for dynamic delimiter recovery and lexer rules

### Key Types and Modules

| Item | Module | Purpose |
|------|--------|---------|
| `parse_with_heredocs` | `heredoc_parser` | High-level entry point: three-phase heredoc pipeline |
| `HeredocScanner` | `heredoc_parser` | Phase 1: detects `<<EOF` declarations, marks skip lines, timeout/depth protection |
| `HeredocCollector` | `heredoc_parser` | Phase 2: collects heredoc content with statement-boundary awareness |
| `HeredocIntegrator` | `heredoc_parser` | Phase 3: integrates placeholders into output for parser consumption |
| `HeredocRecovery` | `heredoc_recovery` | Resolves dynamic delimiters via static analysis, pattern matching, context analysis |
| `RecoveryResult` | `heredoc_recovery` | Recovery outcome: delimiter, confidence score, method, alternatives, diagnostics |
| `EnhancedHeredocLexer` | `enhanced_heredoc_lexer` | Tokenizer for backtick, escaped, indented, and whitespace-around-operator heredocs |
| `PerlLexer` | `perl_lexer` | Context-aware lexer with `ExpectTerm`/`ExpectOperator` mode for slash disambiguation |
| `LexerAdapter` | `lexer_adapter` | Preprocesses input: rewrites `/` to `_DIV_`, `s///` to `_SUB_/…`, `tr///` to `_TRANS_/…` |
| `Token` / `TokenType` | `perl_lexer` | Token representation with types: Division, RegexMatch, Substitution, Transliteration, etc. |

### Processing Flow

1. **Heredoc pipeline** (`parse_with_heredocs`):
   - Phase 1: `HeredocScanner` finds `<<TERM` declarations, marks content lines to skip
   - Phase 2: `HeredocCollector` gathers content between declaration and terminator lines
   - Phase 3: `HeredocIntegrator` returns processed input with `__HEREDOC_N__` placeholders
2. **Slash disambiguation** (`LexerAdapter::preprocess`):
   - `PerlLexer` tokenizes input tracking `ExpectTerm`/`ExpectOperator` mode
   - Division slashes become `_DIV_`, substitutions become `_SUB_/…/…/flags`, transliterations become `_TRANS_/…/…/flags`
3. **Dynamic recovery** (`HeredocRecovery::recover_dynamic_heredoc`):
   - Tries cached results, static analysis (variable tracking), pattern matching, context analysis, then heuristic fallback

## Usage

```rust
use perl_ts_heredoc_parser::heredoc_parser::parse_with_heredocs;

let input = "my $x = <<'EOF';\nHello\nEOF\n";
let (processed, declarations) = parse_with_heredocs(input);
// processed contains __HEREDOC_1__ placeholder
// declarations[0].content == Some("Hello")
```

```rust
use perl_ts_heredoc_parser::lexer_adapter::LexerAdapter;

let output = LexerAdapter::preprocess("$x / 2");
// output contains "_DIV_" instead of "/"
```

## Important Notes

- Heredoc depth is capped at 100 (`MAX_HEREDOC_DEPTH`) and scanning times out after 5 seconds to prevent DoS
- The `PerlLexer` handles Perl's full quoting zoo: `q//`, `qq//`, `qw//`, `qx//`, `qr//`, `s///`, `tr///`, `y///`, heredocs, and POD
- `LexerAdapter::postprocess` walks the full `AstNode` tree to restore original tokens after PEG parsing
- All tests are inline (`#[cfg(test)]` modules within source files); there is no separate `tests/` directory
- The crate is `publish = false` (internal workspace use only)
