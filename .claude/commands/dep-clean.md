---
description: Remove unused dependencies identified by cargo machete
argument-hint: "[crate] e.g. 'perl-parser' or empty for all"
---

# Dependency Clean

Remove unused dependencies from **$ARGUMENTS** (default: all crates).

## Steps

1. **Identify candidates**:
   ```bash
   cargo machete
   ```

2. **For each unused dependency**:
   a. Remove from `Cargo.toml`
   b. Verify build: `cargo build -p <crate>`
   c. Verify tests: `cargo test -p <crate>`
   d. If removal breaks build: skip it (machete false positive)

3. **Commit each removal separately**:
   ```
   chore(<crate>): remove unused dependency <dep>
   ```

## Safety Rules

- One dep removal per commit (easy to revert)
- Always verify build AND tests pass after removal
- Check if the dep is used via feature flags
- Check if the dep is used in `#[cfg(test)]` blocks
- Check if the dep is used only in examples or benchmarks
