# perl-ts-heredoc-analysis

Standalone heredoc analysis tools for Perl parsing. This crate provides
detection and analysis of problematic Perl patterns, particularly around
heredocs, including anti-pattern detection, dynamic delimiter recovery,
encoding-aware lexing, context-sensitive operator parsing, and statement
boundary tracking.

## Modules

- **anti_pattern_detector** -- Detects problematic heredoc patterns (format heredocs, BEGIN-time heredocs, dynamic delimiters, source filters, regex code blocks, eval strings, tied handles) and produces diagnostics with severity, explanations, and suggested fixes.
- **dynamic_delimiter_recovery** -- Resolves heredoc delimiters that are computed at runtime using heuristics such as variable scanning, concatenation resolution, function call evaluation, and environment variable lookups.
- **encoding_aware_lexer** -- Tracks encoding pragmas (`use encoding`, `use utf8`, `use locale`) and ensures correct heredoc delimiter matching across different character encodings.
- **context_sensitive** -- Context-sensitive lexer for Perl operators (`s///`, `tr///`, `m//`) that require special parsing to distinguish from identifiers.
- **statement_tracker** -- Statement boundary tracker with block depth tracking and heredoc context management for proper heredoc content collection.
- **runtime_heredoc_handler** -- Runtime heredoc evaluation with variable interpolation, nested heredoc handling, and eval context tracking.
- **string_utils** -- String utility functions for stripping delimiters and unquoting.

## Part of the perl-lsp Workspace

This crate is part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace and is not published to crates.io.

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
