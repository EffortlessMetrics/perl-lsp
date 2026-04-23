# perl-ts-partial-ast

Partial parse and anti-pattern AST for Perl. This crate extends the standard
Perl AST with nodes that represent unparseable or problematic constructs
(particularly heredoc edge cases) while still maintaining a valid tree structure.

## Features

- **Extended AST nodes** (`ExtendedAstNode`): wraps normal AST nodes with
  variants for warnings, partial parses, unparseable regions, and
  runtime-dependent constructs.
- **Phase-aware parsing** (`PhaseAwareParser`): tracks Perl compilation phases
  (BEGIN, CHECK, INIT, END, eval, use) to flag heredocs with compile-time
  side effects.
- **Understanding parser** (`UnderstandingParser`): combines the Pest-based
  parser with anti-pattern detection and error recovery, producing parse
  coverage metrics.
- **Tree-sitter adapter** (`TreeSitterAdapter`): converts the extended AST into
  tree-sitter-compatible node structures with separate diagnostics.
- **Edge case handler** (`EdgeCaseHandler`): unified interface that orchestrates
  anti-pattern detection, phase analysis, dynamic delimiter recovery, and
  generates actionable recommendations.

## Role in workspace

This is a tree-sitter microcrate (`perl-ts-*` family) that depends on
`perl-parser-pest` and `perl-ts-heredoc-analysis`. It is consumed by
`perl-ts-advanced-parsers` and other downstream crates that need graceful
handling of Perl code that cannot be fully statically parsed.

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
