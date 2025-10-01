---
title: "TODO: Implement method extraction refactoring"
labels: ["technical-debt", "refactoring", "lsp-feature"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.rs:390` indicates that the "extract method" refactoring is not yet implemented. This is a valuable refactoring tool for improving code organization, reducing duplication, and enhancing readability by allowing users to easily turn a block of code into a new subroutine or method.

```rust
// crates/perl-parser/src/refactoring.rs
// L390: // TODO: Implement method extraction
```

## Proposed Fix

Implement the "extract method" refactoring. This would involve:
1. Identifying the selected code block (e.g., from a user's selection in the editor).
2. Analyzing the selected code to determine its inputs (parameters) and outputs (return values).
3. Creating a new method/subroutine with the extracted code, including appropriate parameters and a return type.
4. Replacing the original code block with a call to the newly created method.
5. Handling potential side effects and variable scope.

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Implement method extraction
// handle_code_action(ExtractMethod);

// After (conceptual):
let extracted_method_edits = extract_method_from_selection(document, range)?;
apply_workspace_edits(extracted_method_edits);
```

## Acceptance Criteria

- [ ] Users can successfully extract a selected code block into a new method.
- [ ] The new method has correct parameters and return values.
- [ ] The original code block is replaced with a call to the new method.
- [ ] The refactoring handles various code structures (e.g., loops, conditionals).
- [ ] Relevant tests are updated or added to cover method extraction scenarios.
- [ ] Code adheres to project conventions.
