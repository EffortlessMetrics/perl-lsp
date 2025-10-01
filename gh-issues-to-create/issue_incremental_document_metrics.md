---
title: "TODO: Enable metrics assertions once multi-edit reuse is fully implemented"
labels: ["technical-debt", "incremental-parsing", "metrics"]
---

## Problem

The `TODO` at `crates/perl-parser/src/incremental_document.rs:714` indicates that certain metrics assertions are currently disabled. These assertions are crucial for validating the performance and correctness of the incremental parsing mechanism, specifically related to multi-edit reuse. With them disabled, there's a risk that issues in multi-edit reuse could go undetected.

```rust
// crates/perl-parser/src/incremental_document.rs
// L714: // TODO: enable metrics assertions once multi-edit reuse is fully implemented
```

## Proposed Fix

Once the multi-edit reuse functionality within the incremental parser is fully implemented, stable, and thoroughly tested, the disabled metrics assertions should be re-enabled. This will ensure continuous validation of the incremental parser's efficiency and accuracy.

```rust
// crates/perl-parser/src/incremental_document.rs

// Before:
// // TODO: enable metrics assertions once multi-edit reuse is fully implemented
// // assert_metrics_enabled();

// After (conceptual, once multi-edit reuse is done and stable):
assert_metrics_enabled(); // Re-enable the assertions
```

## Acceptance Criteria

- [ ] Multi-edit reuse functionality is fully implemented and passes all dedicated tests.
- [ ] The metrics assertions at `L714` are re-enabled.
- [ ] All tests, including those with re-enabled assertions, pass successfully.
- [ ] Code adheres to project conventions.
