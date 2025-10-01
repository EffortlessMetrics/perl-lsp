---
title: "TODO: Add CodeActionKind::SOURCE_ORGANIZE_IMPORTS capability"
labels: ["technical-debt", "lsp", "capabilities", "import-optimization"]
---

## Problem

The `TODO` at `crates/perl-parser/src/capabilities.rs:407` indicates that the `CodeActionKind::SOURCE_ORGANIZE_IMPORTS` capability is not yet exposed by the LSP server. This is contingent on import optimization being fully tested. As a result, LSP clients cannot offer the "organize imports" feature, which helps maintain clean and consistent import statements.

```rust
// crates/perl-parser/src/capabilities.rs
// L407: // TODO: Add CodeActionKind::SOURCE_ORGANIZE_IMPORTS when import optimization is tested
```

## Proposed Fix

Once the import optimization functionality is fully implemented, stable, and its dedicated tests are consistently passing, the `CodeActionKind::SOURCE_ORGANIZE_IMPORTS` capability should be enabled. This will allow LSP clients to discover and offer this useful code organization feature.

```rust
// crates/perl-parser/src/capabilities.rs

// Before:
// // TODO: Add CodeActionKind::SOURCE_ORGANIZE_IMPORTS when import optimization is tested
// // capabilities.code_action_provider = Some(CodeActionProviderCapability::Options(CodeActionOptions {
// //     code_action_kinds: Some(vec![CodeActionKind::SOURCE_ORGANIZE_IMPORTS]),
// //     resolve_provider: Some(true),
// //     work_done_progress_options: Default::default(),
// // }));

// After (conceptual, once tests pass):
capabilities.code_action_provider = Some(CodeActionProviderCapability::Options(CodeActionOptions {
    code_action_kinds: Some(vec![CodeActionKind::SOURCE_ORGANIZE_IMPORTS]),
    resolve_provider: Some(true),
    work_done_progress_options: Default::default(),
}));
```

## Acceptance Criteria

- [ ] Import optimization functionality is fully implemented and passes all tests.
- [ ] The `CodeActionKind::SOURCE_ORGANIZE_IMPORTS` is added to the LSP server's capabilities.
- [ ] LSP clients can successfully trigger and apply the "organize imports" code action.
- [ ] Code adheres to project conventions.
