# CLAUDE.md -- perl-ts-heredoc-analysis

## Crate Overview

- **Version**: 0.1.0
- **Tier**: Leaf crate (no internal workspace dependencies; depends only on external crates)
- **Purpose**: Standalone heredoc analysis tools for Perl parsing. Provides anti-pattern detection, dynamic delimiter recovery, encoding-aware lexing, context-sensitive operator parsing, statement boundary tracking, and runtime heredoc evaluation.

## Commands

```bash
# Build
cargo build -p perl-ts-heredoc-analysis

# Test (all tests are inline in source modules)
cargo test -p perl-ts-heredoc-analysis

# Clippy
cargo clippy -p perl-ts-heredoc-analysis

# Generate docs
cargo doc -p perl-ts-heredoc-analysis --no-deps
```

## Architecture

### Dependencies

- **regex** -- Pattern matching for pragma detection, delimiter patterns, and anti-pattern scanning
- **encoding_rs** -- Character encoding conversion (UTF-8, Windows-1252, etc.)
- **serde** (with derive) -- Serialization support
- **thiserror** -- Error type derivation for `RuntimeError`
- **perl-tdd-support** (dev) -- Test helpers (`must`, `must_some`)

### Key Types and Modules

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `anti_pattern_detector` | `AntiPatternDetector`, `AntiPattern`, `Diagnostic`, `Severity` | Detects 7 categories of problematic heredoc patterns and produces diagnostics |
| `dynamic_delimiter_recovery` | `DynamicDelimiterRecovery`, `RecoveryMode`, `DelimiterAnalysis`, `ParseContext` | Resolves runtime-computed heredoc delimiters via heuristics |
| `encoding_aware_lexer` | `EncodingAwareLexer`, `EncodingContext`, `DelimiterSafety` | Tracks encoding pragmas and normalizes delimiters across encodings |
| `context_sensitive` | `ContextSensitiveLexer`, `ContextToken` | Parses `s///`, `tr///`, `m//` operators |
| `statement_tracker` | `StatementTracker`, `BlockBoundary`, `HeredocContext`, `BlockType` | Tracks statement boundaries, block depth, and heredoc declarations |
| `runtime_heredoc_handler` | `RuntimeHeredocHandler`, `RuntimeHeredocContext`, `RuntimeError` | Runtime heredoc evaluation with interpolation and nested context |
| `string_utils` | `strip_enclosing`, `unquote_if_quoted`, `strip_any`, `is_enclosed` | String delimiter utilities |

### Anti-Pattern Categories

The `AntiPatternDetector` detects: `FormatHeredoc`, `BeginTimeHeredoc`, `DynamicHeredocDelimiter`, `SourceFilterHeredoc`, `RegexCodeBlockHeredoc`, `EvalStringHeredoc`, `TiedHandleHeredoc`.

### Recovery Modes

`DynamicDelimiterRecovery` supports four modes: `Conservative` (mark unparseable), `BestGuess` (heuristic resolution), `Interactive` (prompt user), `Sandbox` (opt-in execution).

## Usage

```rust
use perl_ts_heredoc_analysis::anti_pattern_detector::AntiPatternDetector;

let detector = AntiPatternDetector::new();
let diagnostics = detector.detect_all(perl_source_code);
let report = detector.format_report(&diagnostics);
```

```rust
use perl_ts_heredoc_analysis::statement_tracker::find_statement_end_line;

let end_line = find_statement_end_line(source, heredoc_line);
```

## Important Notes

- This crate has `publish = false` and is internal to the workspace.
- All regex patterns use `LazyLock<Regex>` for thread-safe lazy initialization.
- Tests are inline in each module (no separate `tests/` directory).
- The `statement_tracker` module has block depth tracking for Issue #182/#220.
- The `encoding_aware_lexer` supports mid-file encoding changes via pragma detection.
