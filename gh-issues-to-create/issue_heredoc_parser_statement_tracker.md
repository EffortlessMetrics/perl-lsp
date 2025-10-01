---
title: "TODO: Fix statement_tracker to handle heredocs inside blocks properly"
labels: ["technical-debt", "heredocs", "parser"]
---

## Problem

The `statement_tracker` in `crates/tree-sitter-perl-rs/src/heredoc_parser.rs` at line 473 does not correctly handle heredocs when they are embedded within code blocks. This can lead to incorrect parsing or highlighting of code containing such constructs.

```rust
// crates/tree-sitter-perl-rs/src/heredoc_parser.rs
// L473: // TODO: Fix statement_tracker to handle heredocs inside blocks properly
```

## Proposed Fix

Refactor the `statement_tracker` logic to correctly identify and track heredocs within various block constructs (e.g., `if`, `for`, `sub`). This might involve:
1. Modifying the `statement_tracker` to be context-aware of block boundaries.
2. Ensuring that heredoc delimiters are correctly matched even when nested.

```rust
// crates/tree-sitter-perl-rs/src/heredoc_parser.rs

// Before:
// TODO: Fix statement_tracker to handle heredocs inside blocks properly
// current_statement_tracker.handle_heredoc(heredoc_info);

// After (conceptual):
if in_block_context {
    statement_tracker.handle_heredoc_in_block(heredoc_info, &block_context);
} else {
    statement_tracker.handle_heredoc(heredoc_info);
}
```

## Acceptance Criteria

- [ ] Heredocs inside code blocks are parsed correctly.
- [ ] The `statement_tracker` accurately reflects the state of heredocs within blocks.
- [ ] Relevant tests are updated or added to cover these edge cases.
- [ ] Code adheres to project conventions.
