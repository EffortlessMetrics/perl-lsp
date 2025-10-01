---
title: "TODO: Set proper name span in parser"
labels: ["technical-debt", "parser", "ast"]
---

## Problem

The `TODO` at `crates/perl-parser/src/parser.rs:2008` indicates that the `name_span` for some parsed elements is not being set correctly, defaulting to `None`. This omission can negatively impact various LSP features such as "rename," "find references," or "go to definition," as the exact location of a symbol's name is crucial for precise navigation and refactoring.

```rust
// crates/perl-parser/src/parser.rs
// L2008: name_span: None, // TODO: Set proper name span
```

## Proposed Fix

Modify the parser to correctly identify and capture the span (start and end position) of names for all relevant AST nodes. This would involve:
1. Enhancing the parsing logic to specifically extract the range corresponding to the identifier or name of a declaration or reference.
2. Populating the `name_span` field of the AST node with this accurate range.

```rust
// crates/perl-parser/src/parser.rs

// Before:
// name_span: None, // TODO: Set proper name span

// After (conceptual):
name_span: Some(get_name_span_from_node(node)), // `get_name_span_from_node` would be a new or enhanced parser helper
```

## Acceptance Criteria

- [ ] All relevant AST nodes have their `name_span` field correctly populated.
- [ ] LSP features relying on `name_span` (e.g., rename, find references) function with improved precision.
- [ ] Relevant tests are updated or added to validate the correctness of `name_span` extraction.
- [ ] Code adheres to project conventions.
