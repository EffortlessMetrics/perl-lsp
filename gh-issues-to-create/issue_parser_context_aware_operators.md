---
title: "TODO: Implement proper context-aware parsing for operators"
labels: ["technical-debt", "parser", "operators"]
---

## Problem

The `TODO` at `crates/perl-parser/src/lib.rs:403` suggests that certain operators are not being parsed with proper context awareness. This can lead to ambiguities, incorrect AST representation, or semantic misinterpretations, especially in a language like Perl where operator behavior can be highly context-dependent.

```rust
// crates/perl-parser/src/lib.rs
// L403: // TODO: Implement proper context-aware parsing for these operators
```

## Proposed Fix

Enhance the parser's logic to incorporate context-aware parsing for the specified operators. This might involve:
1. Adding more sophisticated lookahead or backtracking mechanisms to resolve operator ambiguities based on surrounding tokens or AST structure.
2. Integrating with a context-tracking mechanism within the parser to maintain information about the current parsing environment (e.g., whether inside a list context, scalar context, etc.).
3. Potentially refactoring the operator parsing logic to use a more robust approach (e.g., a Pratt parser if not already in use, or a more explicit state machine).

```rust
// crates/perl-parser/src/lib.rs

// Before:
// TODO: Implement proper context-aware parsing for these operators
// parse_operator(token);

// After (conceptual):
parse_operator_with_context(token, &current_parsing_context); // `current_parsing_context` would provide necessary contextual information
```

## Acceptance Criteria

- [ ] All specified operators are parsed correctly, resolving ambiguities based on context.
- [ ] The AST accurately reflects the intended meaning of operator expressions.
- [ ] No regressions are introduced for existing operator parsing.
- [ ] Relevant tests are updated or added to cover context-aware operator parsing.
- [ ] Code adheres to project conventions.
