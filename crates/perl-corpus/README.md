# perl-corpus

Reusable generators for Perl test corpora: proptest strategies, builders, and edge cases.

## Usage

```rust
use perl_corpus::{generate_perl_code, EdgeCaseGenerator};

// Generate random valid Perl code
let code = generate_perl_code();

// Generate edge cases for testing
let edge_cases = EdgeCaseGenerator::all_cases();
```

## Features

- Property-based testing strategies via proptest
- Edge case generation for parser testing
- Perl code builders with configurable complexity

## License

MIT