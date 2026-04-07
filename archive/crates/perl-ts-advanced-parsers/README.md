# perl-ts-advanced-parsers

Composed parser experiments for Perl, providing advanced parsing capabilities
built on top of the `perl-parser-pest` grammar and the `perl-ts-heredoc-*`
heredoc processing pipeline.

## Features

- **Full parser** with heredoc support and slash disambiguation (`FullPerlParser`, `EnhancedFullParser`)
- **Disambiguated parser** for context-sensitive `/` (regex vs division) handling (`DisambiguatedParser`)
- **Stateful parser** for heredoc and format declaration collection (`StatefulPerlParser`)
- **Enhanced parser** that auto-detects heredocs and delegates to the stateful parser (`EnhancedPerlParser`)
- **Context-aware parser** for heredocs inside `eval` strings and `s///e` replacements (`ContextAwareHeredocParser`)
- **Streaming parser** for large files, processing input line-by-line and emitting `ParseEvent`s (`StreamingParser`)
- **Incremental parser** for efficient re-parsing after document edits (`IncrementalParser`)
- **Iterative AST builder** using an explicit stack to avoid stack overflow on deep nesting (`IterativeBuilder`)
- **Error recovery parser** with configurable recovery strategies (`ErrorRecoveryParser`)
- **Experimental LSP server** with diagnostics, completions, and symbol extraction (`PerlLanguageServer`)

## Part of the `tree-sitter-perl-rs` Workspace

This crate is internal (`publish = false`) and is not published to crates.io.
It depends on `perl-parser-pest`, `perl-ts-heredoc-parser`, `perl-ts-heredoc-analysis`,
and `perl-ts-partial-ast`.

## License

Licensed under either of Apache License, Version 2.0 or MIT License, at your option.
