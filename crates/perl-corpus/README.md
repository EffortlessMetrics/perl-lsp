# perl-corpus

Reusable generators for Perl test corpora: proptest strategies, fixtures, and edge cases.

## Usage

```rust
use perl_corpus::{
    complex_data_structure_cases, generate_perl_code, generate_perl_code_with_options,
    get_all_test_files, CodegenOptions, EdgeCaseGenerator, StatementKind,
};

// Generate random valid Perl code
let code = generate_perl_code();

// Customize code generation coverage
let mut options = CodegenOptions::default();
options.statements = 50;
options.ensure_coverage = true;
options.kinds = vec![StatementKind::Expressions, StatementKind::Regex];
let code = generate_perl_code_with_options(options);

// Generate edge cases for testing
let edge_cases = EdgeCaseGenerator::all_cases();

// Discover local corpus files for integration testing
let files = get_all_test_files();

// Retrieve complex data structure samples for DAP variable rendering
let cases = complex_data_structure_cases();
```

## Features

- Property-based testing strategies via proptest
- Edge case fixtures with tags and IDs
- Random code generation helpers
- Local corpus file discovery (test_corpus + fuzz fixtures)
- Generators for heredoc, quote-like, regex, expressions, whitespace, loop control, format, glob, tie, I/O, declarations

## License

MIT
