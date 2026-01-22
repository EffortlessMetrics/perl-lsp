# perl-tdd-support

Test-driven development support utilities for the Perl parser ecosystem.

## Purpose

This crate provides tools to support TDD workflows when working with Perl code, including test generation, execution runners, and validation utilities for Perl parser and LSP development. It helps maintain high code quality through systematic test-driven development practices.

## Key Features

- **Test Case Generators**: Generate test cases for Perl syntax patterns
- **Test Execution Runners**: Execute tests with comprehensive result capture
- **TDD Workflow Helpers**: Support iterative parser development with TDD practices
- **Parser Validation**: Validate parser behavior against expected outcomes
- **LSP Compatibility**: Optional LSP-compatible features for integration testing

## Usage

```rust
use perl_tdd_support::tdd_basic;

// Use TDD helpers to validate parser behavior
// (specific APIs depend on tdd module implementation)
```

## Features

- `default`: Basic TDD functionality
- `lsp-compat`: Enable LSP types integration for LSP feature testing

## Documentation

For detailed API documentation, see [docs.rs/perl-tdd-support](https://docs.rs/perl-tdd-support).

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
