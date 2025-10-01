---
title: "TODO: Add CodeActionKind::REFACTOR_EXTRACT capability"
labels: ["technical-debt", "lsp", "capabilities", "refactoring"]
---

## Problem

The `TODO` at `crates/perl-parser/src/capabilities.rs:406` indicates that the `CodeActionKind::REFACTOR_EXTRACT` capability is not yet exposed by the LSP server. This is because the corresponding "extract variable/subroutine" refactoring tests are not yet passing. Consequently, IDEs connected to this LSP server cannot offer the "extract method/variable" refactoring feature to users.

```rust
// crates/perl-parser/src/capabilities.rs
// L406: // TODO: Add CodeActionKind::REFACTOR_EXTRACT when extract variable/subroutine tests pass
```

## Proposed Fix

Once the "extract variable/subroutine" refactoring is fully implemented, stable, and its dedicated tests are consistently passing, the `CodeActionKind::REFACTOR_EXTRACT` capability should be enabled. This will allow LSP clients to discover and offer this valuable refactoring feature.

```rust
// crates/perl-parser/src/capabilities.rs

// Before:
// // TODO: Add CodeActionKind::REFACTOR_EXTRACT when extract variable/subroutine tests pass
// // capabilities.code_action_provider = Some(CodeActionProviderCapability::Options(CodeActionOptions {
// //     code_action_kinds: Some(vec![CodeActionKind::REFACTOR_EXTRACT]),
// //     resolve_provider: Some(true),
// //     work_done_progress_options: Default::default(),
// // }));

// After (conceptual, once tests pass):
capabilities.code_action_provider = Some(CodeActionProviderCapability::Options(CodeActionOptions {
    code_action_kinds: Some(vec![CodeActionKind::REFACTOR_EXTRACT]),
    resolve_provider: Some(true),
    work_done_progress_options: Default::default(),
}));
```

## Acceptance Criteria

- [ ] The "extract variable/subroutine" refactoring is fully implemented and passes all tests.
- [ ] The `CodeActionKind::REFACTOR_EXTRACT` is added to the LSP server's capabilities.
- [ ] LSP clients can successfully trigger and apply the "extract method/variable" refactoring.
- [ ] Code adheres to project conventions.
