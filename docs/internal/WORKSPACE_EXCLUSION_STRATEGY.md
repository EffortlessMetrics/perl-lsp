# Workspace Exclusion Strategy

## Overview

This document explains the simplified workspace exclusion strategy implemented for the Perl LSP project. The strategy reduces complexity and improves maintainability while keeping excluded crates available for specialized use cases.

## Excluded Crates

The following directories are excluded from the default workspace build:

| Directory | Reason for Exclusion | Status |
|-----------|---------------------|--------|
| `tree-sitter-perl` | Legacy tree-sitter grammar (not used by workspace) | Maintained separately |
| `tree-sitter-perl-c` | Requires `libclang-dev` system package for `bindgen` | Legacy, requires C dependencies |
| `crates/tree-sitter-perl-rs` | Legacy Rust wrapper for tree-sitter (not used) | Not required by workspace |
| `fuzz` | Requires `cargo-fuzz` installation and specialized build | Fuzzing infrastructure |
| `archive` | Archived legacy components | Unmaintained |

## Rationale

### Problem (Issue #151)

The previous workspace configuration had several complexity issues:

1. **Confusing Dependencies**: `tree-sitter-perl` and `tree-sitter-perl-c` were defined in `[workspace.dependencies]` but also excluded from the workspace
2. **Maintenance Overhead**: Unclear why certain crates were excluded and how to build them independently
3. **Contributor Confusion**: New contributors unclear about build requirements and optional dependencies
4. **Integration Issues**: Difficulty managing dependencies between excluded and included crates

### Solution

The simplified strategy:

1. **Removed Unused References**: Deleted `tree-sitter-perl` and `tree-sitter-perl-c` from `[workspace.dependencies]` since no workspace members use them
2. **Improved Documentation**: Added clear, structured comments explaining each exclusion
3. **Verified Independence**: Confirmed no workspace members depend on excluded crates
4. **Validation Tools**: Created scripts to verify exclusion strategy remains clean

## Implementation Details

### Changes Made

1. **Cargo.toml Exclusions Section**:
   - Added structured documentation block with clear explanations
   - Listed each excluded directory with reason for exclusion
   - Removed inline comments in favor of grouped documentation

2. **Workspace Dependencies**:
   - Removed `tree-sitter-perl = { path = "crates/tree-sitter-perl-rs", features = ["pure-rust"] }`
   - Removed `tree-sitter-perl-c = { path = "crates/tree-sitter-perl-c" }`
   - Added explanatory comment noting the exclusion

3. **Validation**:
   - Created `scripts/validate-workspace-exclusions.sh` to verify exclusion strategy
   - Created integration test `crates/perl-lsp/tests/workspace_exclusion_test.rs`
   - Ensures no accidental dependencies sneak in

### Benefits

- **Reduced Complexity**: 2 fewer workspace dependency declarations
- **Clear Documentation**: Grouped, structured explanation of exclusions
- **Better Maintainability**: Validation scripts prevent regression
- **Contributor Experience**: Clear understanding of what's excluded and why
- **Build Simplicity**: Default build doesn't require `libclang-dev` or `cargo-fuzz`

## Validation

### Automated Checks

Run the validation script to verify exclusion strategy:

```bash
./scripts/validate-workspace-exclusions.sh
```

This checks:
- ✅ Excluded directories exist
- ✅ Exclusion strategy is documented
- ✅ `workspace.dependencies` doesn't reference excluded crates
- ✅ All exclusions are listed in `exclude` section
- ✅ Excluded crates not in workspace members
- ✅ No workspace members depend on excluded crates

### Manual Verification

```bash
# Verify workspace builds without excluded crates
cargo check --workspace

# Count workspace members (should be 44, excluding the 5 excluded directories)
cargo metadata --format-version=1 | jq '.workspace_members | length'

# Verify no tree-sitter-perl references in workspace
cargo metadata --format-version=1 | jq '.workspace_members[]' | grep tree-sitter-perl
# (should return nothing)
```

## Building Excluded Crates

If you need to work with excluded crates for specialized tasks:

### tree-sitter-perl-c

Requires `libclang-dev`:

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libclang-dev

# Build
cargo check --manifest-path=crates/tree-sitter-perl-c/Cargo.toml
```

### tree-sitter-perl-rs

```bash
# Requires workspace members built first
cargo build --workspace

# Then build excluded crate
cargo check --manifest-path=crates/tree-sitter-perl-rs/Cargo.toml
```

### fuzz

Requires `cargo-fuzz`:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run fuzzing
cargo fuzz --manifest-path=fuzz/Cargo.toml
```

## Maintenance

### Adding New Exclusions

If you need to exclude a new directory:

1. Add it to the `exclude` array in `Cargo.toml`
2. Update the documentation block with the reason for exclusion
3. Run `./scripts/validate-workspace-exclusions.sh` to verify
4. Update this document with the new exclusion

### Removing Exclusions

If a crate should be reintegrated:

1. Remove from `exclude` array in `Cargo.toml`
2. Add to `members` array if needed
3. Update documentation
4. Verify workspace still builds: `cargo check --workspace`
5. Update this document

## References

- Issue #151: Complex Workspace Exclusion Strategy May Cause Maintenance Issues
- Cargo Book: [Workspace Configuration](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- Validation Script: `scripts/validate-workspace-exclusions.sh`
- Integration Test: `crates/perl-lsp/tests/workspace_exclusion_test.rs`

## Status

✅ **Implemented**: Simplified exclusion strategy (2026-01-28)
✅ **Validated**: All exclusion checks passing
✅ **Documented**: Clear documentation and validation tools in place
