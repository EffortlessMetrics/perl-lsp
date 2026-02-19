# CLAUDE.md - perl-ts-advanced-parsers

## Crate Overview

- **Version**: 0.1.0
- **Tier**: Experimental / internal (`publish = false`)
- **Purpose**: Composed parser experiments layering heredoc processing, slash disambiguation, error recovery, incremental parsing, streaming, and an experimental LSP server on top of the Pest grammar.

## Commands

```bash
cargo build -p perl-ts-advanced-parsers
cargo test -p perl-ts-advanced-parsers
cargo clippy -p perl-ts-advanced-parsers
cargo doc -p perl-ts-advanced-parsers --no-deps
# Run stress tests (deep nesting)
cargo test -p perl-ts-advanced-parsers --features stress-tests
```

## Architecture

### Dependencies

- `perl-parser-pest` -- Pest grammar, `PureRustPerlParser`, `AstNode`, `Rule`
- `perl-ts-heredoc-parser` -- heredoc scanning, `LexerAdapter` (slash preprocessing/postprocessing)
- `perl-ts-heredoc-analysis` -- heredoc analysis utilities
- `perl-ts-partial-ast` -- partial AST types
- `regex`, `pest`, `serde`, `serde_json` -- external crates

### Key Types / Modules

| Module | Primary Type | Role |
|--------|-------------|------|
| `full_parser` | `FullPerlParser` | Multi-phase parser: heredoc -> slash disambiguation -> Pest -> AST postprocess |
| `enhanced_full_parser` | `EnhancedFullParser` | Adds DATA/END section extraction and POD handling |
| `enhanced_parser` | `EnhancedPerlParser` | Auto-detects heredocs, delegates to `StatefulPerlParser` |
| `disambiguated_parser` | `DisambiguatedParser` | Slash-only disambiguation via `LexerAdapter` |
| `stateful_parser` | `StatefulPerlParser` | Line-by-line heredoc/format collection and AST injection |
| `context_aware_parser` | `ContextAwareHeredocParser`, `ContextAwareFullParser` | Handles heredocs inside `eval` and `s///e` |
| `streaming_parser` | `StreamingParser<R>` | Chunk-based parsing with `ParseEvent` iterator |
| `incremental_parser` | `IncrementalParser` | Maintains `ParseTree` across edits; re-parses affected regions |
| `iterative_parser` | `IterativeBuilder` (trait on `PureRustPerlParser`) | Explicit-stack AST building to prevent stack overflow |
| `error_recovery` | `ErrorRecoveryParser` | Configurable `RecoveryStrategy` list; collects `ErrorNode`s |
| `lsp_server` | `PerlLanguageServer` | Document management, diagnostics, completions, symbol extraction |

### Feature Flags

- `stress-tests` -- enables deep-nesting iterative parser stress tests

## Usage Examples

```rust
use perl_ts_advanced_parsers::full_parser::FullPerlParser;

let mut parser = FullPerlParser::new();
let ast = parser.parse("my $x = <<EOF;\nHello\nEOF\n")?;
let sexp = parser.parse_to_sexp("print 1 / /re/;")?;
```

```rust
use perl_ts_advanced_parsers::streaming_parser::{StreamingParser, StreamConfig};
use std::io::Cursor;

let mut sp = StreamingParser::new(Cursor::new("my $x = 42;\n"), StreamConfig::default());
for event in sp.parse() {
    println!("{:?}", event);
}
```

## Important Notes

- This crate is **not published** (`publish = false`); it is workspace-internal.
- The `lsp_server` module is experimental and separate from the production `perl-lsp` binary.
- `EnhancedFullParser` and `FullPerlParser` both follow a multi-phase pipeline: heredoc scan -> slash disambiguation -> Pest parse -> AST build -> postprocess.
- The iterative builder (`IterativeBuilder` trait) is `pub(crate)` and not part of the public API.
