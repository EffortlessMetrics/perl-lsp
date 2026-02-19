# perl-corpus

Test corpus management, property-based generators, and edge case fixtures for Perl parsers.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

`perl-corpus` provides curated Perl code samples, randomized code generators (via `proptest`), and a CLI tool for corpus linting, indexing, and statistics. It is used across the workspace for parser testing, LSP feature validation, and DAP variable inspection.

## Key Features

- **Corpus file parsing**: Load and query section-based `.txt` corpus files with metadata (tags, IDs, flags)
- **Edge case fixtures**: 100 static edge cases and 32 complex data structure samples, filterable by tag
- **Property-based generators**: 21 generator categories (regex, heredoc, glob, tie, I/O, expressions, OOP, and more)
- **Randomized codegen**: Deterministic Perl code generation with seed control and coverage options
- **Specialized fixtures**: Continue/redo, format statements, glob expressions, tie interface test cases
- **CLI tool**: `perl-corpus lint`, `perl-corpus index`, `perl-corpus stats`, `perl-corpus gen`

## Usage

```rust
use perl_corpus::{parse_dir, find_by_tag, EdgeCaseGenerator, generate_perl_code_with_seed};

// Load and query corpus sections
let sections = parse_dir(std::path::Path::new("test_corpus")).unwrap();
let regex_tests = find_by_tag(&sections, "regex");

// Generate deterministic Perl code
let code = generate_perl_code_with_seed(10, 42);

// Query edge case fixtures
let heredocs = EdgeCaseGenerator::by_tag("heredoc");
```

## License

MIT OR Apache-2.0
