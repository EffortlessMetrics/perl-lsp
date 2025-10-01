---
title: "TODO: Track containing package/class for workspace symbols"
labels: ["technical-debt", "lsp", "workspace-symbols"]
---

## Problem

The `container` field for `WorkspaceSymbol` objects in `crates/perl-parser/src/workspace_symbols.rs` at line 132 is currently `None`. This means the LSP server isn't providing information about the containing package or class for symbols, which reduces the usefulness and accuracy of workspace symbol searches in IDEs.

```rust
// crates/perl-parser/src/workspace_symbols.rs
// L132: container: None, // TODO: Track containing package/class
```

## Proposed Fix

Implement logic to determine and populate the `container` field for `WorkspaceSymbol` objects. This would involve:
1. Analyzing the AST to identify the parent scope (package, class, subroutine) of each symbol.
2. Populating the `container` field with the appropriate name.

```rust
// crates/perl-parser/src/workspace_symbols.rs

// Before:
// container: None, // TODO: Track containing package/class

// After (conceptual):
container: Some(get_containing_scope_name(symbol_node)),
```

## Acceptance Criteria

- [ ] `WorkspaceSymbol` objects correctly report their containing package or class.
- [ ] Workspace symbol searches in LSP clients provide more accurate and complete results.
- [ ] Relevant tests are updated or added to validate this functionality.
- [ ] Code adheres to project conventions.
