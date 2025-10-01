---
title: "TODO: Calculate proper name range for call hierarchy"
labels: ["technical-debt", "lsp", "call-hierarchy"]
---

## Problem

The `TODO` at `crates/perl-parser/src/call_hierarchy_provider.rs:121` indicates that the `selection_range` for call hierarchy items is currently a clone of the full range, and the actual name range needs to be calculated. This can lead to less precise or confusing results in LSP clients when navigating call hierarchies, as the highlight might cover more than just the symbol's name.

```rust
// crates/perl-parser/src/call_hierarchy_provider.rs
// L121: let selection_range = range.clone(); // TODO: Calculate name range
```

## Proposed Fix

Implement logic to accurately calculate the `name_range` for call hierarchy items. This would involve:
1. Analyzing the AST node corresponding to the call hierarchy item (e.g., a function definition).
2. Identifying the specific token or AST node that represents the name of the function/method.
3. Extracting its span (start and end position) to set as the `name_range`.

```rust
// crates/perl-parser/src/call_hierarchy_provider.rs

// Before:
// let selection_range = range.clone(); // TODO: Calculate name range

// After (conceptual):
let name_range = get_name_range_from_node(node); // `get_name_range_from_node` would be a new or enhanced AST utility
let selection_range = name_range.clone(); // Use the precise name range for selection
```

## Acceptance Criteria

- [ ] Call hierarchy items correctly report the precise `name_range`.
- [ ] LSP clients display more accurate highlights for call hierarchy entries.
- [ ] Relevant tests are updated or added to validate the correctness of `name_range` calculation.
- [ ] Code adheres to project conventions.
