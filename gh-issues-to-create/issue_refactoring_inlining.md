---
title: "TODO: Implement inlining refactoring"
labels: ["technical-debt", "refactoring", "lsp-feature"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.rs:499` indicates that the "inline" refactoring (e.g., inlining a variable, a constant, or a simple function call) is not yet implemented. This refactoring can improve code readability by removing unnecessary abstractions or intermediate variables.

```rust
// crates/perl-parser/src/refactoring.rs
// L499: // TODO: Implement inlining
```

## Proposed Fix

Implement the "inline" refactoring. This would involve:
1. Identifying the variable, constant, or function call to be inlined.
2. Analyzing its usage to ensure it can be safely inlined without changing program semantics.
3. Replacing all occurrences of the variable/call with its actual value or the body of the function.
4. Removing the original definition if it's no longer used after inlining.

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Implement inlining
// handle_code_action(InlineVariable);

// After (conceptual):
let inline_edits = inline_code_element(document, element_id)?;
apply_workspace_edits(inline_edits);
```

## Acceptance Criteria

- [ ] Users can successfully inline variables, constants, and simple function calls.
- [ ] The refactoring preserves the original program's behavior.
- [ ] The original definition is removed if it becomes unused.
- [ ] Relevant tests are updated or added to cover inlining scenarios.
- [ ] Code adheres to project conventions.
