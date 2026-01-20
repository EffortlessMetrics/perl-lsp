# perl-corpus

Reusable generators for Perl test corpora: proptest strategies, fixtures, and edge cases.

## Usage

```rust
use perl_corpus::{
    complex_data_structure_cases, generate_perl_code, get_all_test_files, EdgeCaseGenerator,
};

// Generate random valid Perl code
let code = generate_perl_code();

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
- Generators for heredoc, quote-like, whitespace, loop control, format, glob, tie

## License

MIT
