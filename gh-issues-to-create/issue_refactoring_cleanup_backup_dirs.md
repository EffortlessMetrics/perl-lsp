---
title: "TODO: Implement cleanup for refactoring backup directories"
labels: ["technical-debt", "refactoring", "cleanup"]
---

## Problem

The `TODO` at `crates/perl-parser/src/refactoring.rs:317` indicates that backup directories created during refactoring operations are not being cleaned up. This can lead to an accumulation of temporary files and unnecessary disk usage over time.

```rust
// crates/perl-parser/src/refactoring.rs
// L317: // TODO: Cleanup backup directories
```

## Proposed Fix

Implement a cleanup mechanism for backup directories created during refactoring. This could be:
1. A deferred cleanup that runs automatically after a refactoring operation completes successfully.
2. A configuration option to enable/disable backup creation or specify a cleanup policy (e.g., delete after N days, or on application exit).

```rust
// crates/perl-parser/src/refactoring.rs

// Before:
// TODO: Cleanup backup directories
// create_backup_directory(path);

// After (conceptual):
let backup_dir = create_backup_directory(path)?;
// ... perform refactoring ...
// Ensure cleanup happens, even if refactoring fails
defer! { // Assuming a defer macro or similar mechanism
    cleanup_backup_directory(&backup_dir);
}
```

## Acceptance Criteria

- [ ] Backup directories are automatically cleaned up after successful refactoring.
- [ ] A mechanism exists to prevent indefinite accumulation of backup files.
- [ ] User can configure backup behavior if desired.
- [ ] Relevant tests are updated or added.
- [ ] Code adheres to project conventions.
