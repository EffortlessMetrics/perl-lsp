---
title: "TODO: Implement scoped rename refactoring"
labels: ["technical-debt", "refactoring", "lsp-feature"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.rs:370` indicates that scoped rename functionality is not yet implemented. This feature is crucial for renaming local variables, function parameters, or private methods within their defined scope without affecting identically named symbols elsewhere, improving code maintainability.

```rust
// crates/perl-parser/src/refactoring.rs
// L370: // TODO: Implement scoped rename
```

## Proposed Fix

Implement the logic for performing a scoped rename. This would involve:
1. Identifying all occurrences of the symbol within its defined lexical or semantic scope.
2. Generating and applying the necessary text edits only within that specific scope.
3. Ensuring that symbols outside the scope with the same name are not affected.

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Implement scoped rename
// handle_rename_request(params);

// After (conceptual):
let changes = find_and_apply_scoped_rename(old_name, new_name, scope)?;
apply_document_edits(changes);
```

## Acceptance Criteria

- [ ] Scoped rename operations correctly identify and update all symbol occurrences within the specified scope.
- [ ] Symbols outside the target scope are unaffected.
- [ ] The refactoring is applied atomically within the document.
- [ ] Relevant tests are updated or added to cover scoped rename scenarios.
- [ ] Code adheres to project conventions.
