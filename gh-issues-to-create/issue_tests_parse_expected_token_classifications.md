---
title: "TODO: Parse expected token classifications in tests"
labels: ["technical-debt", "testing", "parser"]
---

## Problem

The `TODO` at `crates/tree-sitter-perl-rs/src/tests.rs:574` indicates that the `expected_tokens` field in the test suite is currently `Vec::new()`, meaning that expected token classifications are not being parsed or asserted. This leaves a gap in test coverage, as the correctness of token classification, a fundamental aspect of the lexer/parser, is not being fully validated.

```rust
// crates/tree-sitter-perl-rs/src/tests.rs
// L574: expected_tokens: Vec::new(), // TODO: Parse expected token classifications
```

## Proposed Fix

Implement the logic to parse and compare expected token classifications within the test framework. This would involve:
1. Defining a clear data structure for `ExpectedTokenClassification` that can represent the expected token type and its associated text/span.
2. Modifying the test input format or adding a new mechanism to specify these expected classifications.
3. Implementing a function to parse this information from the test input.
4. Integrating this parsing and comparison logic into the existing test assertions to validate the actual token stream against the expected classifications.

```rust
// crates/tree-sitter-perl-rs/src/tests.rs

// Before:
// expected_tokens: Vec::new(), // TODO: Parse expected token classifications

// After (conceptual):
expected_tokens: parse_expected_token_classifications(test_input), // `parse_expected_token_classifications` would be a new helper function
```

## Acceptance Criteria

- [ ] The test suite can parse and assert against expected token classifications.
- [ ] Test coverage for the lexer/parser's token classification is improved.
- [ ] All existing tests continue to pass, and new tests for token classification can be added.
- [ ] Code adheres to project conventions.
