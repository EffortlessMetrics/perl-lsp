---
title: "TODO: Implement workspace-wide rename refactoring"
labels: ["technical-debt", "refactoring", "lsp-feature"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.rs:356` indicates that workspace-wide rename functionality is not yet implemented. This is a fundamental LSP feature that significantly enhances developer productivity by allowing a symbol to be renamed consistently across an entire project.

```rust
// crates/perl-parser/src/refactoring.rs
// L356: // TODO: Implement workspace-wide rename
```

## Proposed Fix

Implement the logic for performing a workspace-wide rename. This would involve:
1. Identifying all occurrences of the symbol to be renamed across the entire workspace (e.g., using a workspace index or AST traversal).
2. Generating and applying the necessary text edits to all affected files.
3. Handling potential conflicts or ambiguities (e.g., multiple symbols with the same name in different scopes).

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Implement workspace-wide rename
// handle_rename_request(params);

// After (conceptual):
let changes = find_and_apply_workspace_rename(old_name, new_name)?;
apply_workspace_edits(changes);
```

## Acceptance Criteria

- [ ] Workspace-wide rename operations correctly identify and update all symbol occurrences.
- [ ] The refactoring is applied atomically across all files.
- [ ] Edge cases like shadowed variables or different namespaces are handled correctly.
- [ ] Relevant tests are updated or added to cover workspace-wide rename scenarios.
- [ ] Code adheres to project conventions.
