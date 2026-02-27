# perl-ts-heredoc-parser

Heredoc parsing pipeline for Perl with slash disambiguation and recovery.

## Overview

Provides a multi-phase heredoc parsing infrastructure for Perl source code:
three-phase heredoc processing (detection, collection, integration), a
context-aware Perl lexer that disambiguates slash tokens (division vs regex vs
substitution vs transliteration), dynamic heredoc delimiter recovery with
heuristics, an enhanced heredoc lexer supporting backtick/escaped/indented
forms, and a lexer adapter that rewrites ambiguous tokens for PEG grammar
compatibility.

## Public API

- `parse_with_heredocs` -- high-level three-phase heredoc pipeline: detect, collect, integrate
- `HeredocScanner` -- Phase 1: scans input for `<<EOF` declarations and marks content boundaries
- `HeredocCollector` -- Phase 2: collects heredoc content using statement-aware line tracking
- `HeredocIntegrator` -- Phase 3: integrates placeholders into processed output
- `HeredocRecovery` -- recovers dynamic heredoc delimiters via static analysis, pattern matching, and context
- `EnhancedHeredocLexer` -- tokenizer handling backtick, escaped, and indented heredoc variants
- `PerlLexer` -- context-aware lexer with slash disambiguation (`ExpectTerm` vs `ExpectOperator`)
- `LexerAdapter` -- preprocesses slash tokens into `_DIV_`/`_SUB_`/`_TRANS_` markers for PEG parsing

## Workspace Role

Internal microcrate in the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp)
workspace. Depends on `perl-ts-heredoc-analysis` for statement tracking and
`perl-parser-pest` for AST types used in postprocessing.

## License

MIT OR Apache-2.0
