---
title: "TODO: Implement code movement refactoring"
labels: ["technical-debt", "refactoring", "lsp-feature"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.2rs:407` indicates that code movement refactoring (e.g., moving a function, class, or variable declaration to a different file or module) is not yet implemented. This feature is essential for restructuring codebases and improving modularity.

```rust
// crates/perl-parser/src/refactoring.rs
// L407: // TODO: Implement code movement
```

## Proposed Fix

Implement code movement refactoring. This would involve:
1. Identifying the code element to be moved (e.g., a subroutine, a package, a variable).
2. Determining the target location (e.g., a different file, a new module).
3. Applying changes to both the source and target files:
    - Removing the element from the source file.
    - Adding the element to the target file.
    - Updating all references (imports, calls) to the moved element across the workspace.

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Implement code movement
// handle_code_action(MoveCode);

// After (conceptual):
let move_edits = move_code_element(document, element_id, target_uri)?;
apply_workspace_edits(move_edits);
```

## Acceptance Criteria

- [ ] Users can successfully move code elements (e.g., functions, classes) between files/modules.
- [ ] All references to the moved element are correctly updated.
- [ ] The refactoring maintains code correctness and functionality.
- [ ] Relevant tests are updated or added to cover code movement scenarios.
- [ ] Code adheres to project conventions.
