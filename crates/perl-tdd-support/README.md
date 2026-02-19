# perl-tdd-support

Test-driven development helpers, test generators, and safe assertion utilities for the Perl LSP workspace.

## Overview

This crate provides TDD workflow support for Perl parser and LSP development:

- **`must` / `must_some` / `must_err`** -- Safe `unwrap` replacements that comply with the workspace "no unwrap/expect" policy
- **Test generators** -- Generate Perl test cases (Test::More, Test2::V0, Test::Simple, Test::Class) from parsed ASTs
- **Test runner** -- Discover and execute Perl `.t` files with TAP output parsing
- **TDD workflow manager** -- Red-green-refactor cycle state machine with coverage tracking
- **Refactoring analyzer** -- Detect high complexity, long methods, excessive parameters, and naming issues
- **Ignored test governance** -- Baseline tracking, quality gates, and trend reporting for ignored tests

## Features

| Feature | Description |
|---------|-------------|
| `default` | Core TDD helpers and test generation |
| `lsp-compat` | LSP type integration (`lsp-types`, `url`) for code actions and diagnostics |

## Usage

```rust
use perl_tdd_support::{must, must_some, must_err};

// Safe unwrap replacements for tests
let value = must(some_result);
let item = must_some(some_option);
let err = must_err(expected_err_result);
```

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

## License

Licensed under either of MIT or Apache-2.0 at your option.
