# Specification: Include Path Scanning for Module Completion

## Feature Description

When completing `use` or `require` statements, the LSP should suggest modules from configured include paths (`.perl-lsp.toml` `includePaths`), `PERL5LIB`, and system `@INC` in addition to modules found in the workspace index.

### User Experience

After this feature is implemented:
- User configures `includePaths` in `.perl-lsp.toml` or enables `useSystemInc`
- User types `use DBI::` in their Perl code
- LSP suggests `DBI`, `DBD::SQLite`, `DBD::mysql`, and other modules from configured include paths
- Suggestions appear alongside workspace modules, properly deduplicated

### Scope

**In Scope:**
- `.perl-lsp.toml` `includePaths` configuration
- `PERL5LIB` environment variable (via `system_inc_paths`)
- System `@INC` when `useSystemInc: true`
- `use` and `require` statement completion triggers
- Prefix filtering (modules must start with the prefix)
- Depth limiting to prevent excessive traversal
- Timeout and entry limits for performance
- Cancellation support
- WASM32 exclusion (graceful no-op)

**Out of Scope:**
- Caching of scanned modules (deferred to v0.13.0 per ADR)
- Module resolution for goto-definition (handled by `perl-module-resolution`)
- Non-Perl files in include paths
- Recursive scanning beyond depth limit
- Partial module name matching (prefix-based only)

## Acceptance Criteria

### AC1: Include paths are scanned for module completions

**Given** a `.perl-lsp.toml` with `includePaths = ["/path/to/lib"]`
**And** `/path/to/lib/DBI.pm` exists
**When** user types `use DBI`
**Then** `DBI` appears in completion results

**Verification:**
```rust
#[test]
fn property_all_include_paths_scanned() {
    // Create temp dir with DBI.pm
    // Verify DBI appears in completions
}
```

### AC2: System @INC paths are scanned when enabled

**Given** `useSystemInc: true` in `.perl-lsp.toml`
**When** completion is requested
**Then** system @INC directories are scanned for modules

**Verification:**
```rust
#[test]
fn property_system_inc_paths_scanned() {
    // Verify system_inc_paths are passed to provider
}
```

### AC3: Prefix filtering works correctly

**Given** include path with modules `Alpha::Module`, `Alpha::Class`, `Alphabet::Module`, `Beta::Module`
**When** user types `use Alpha`
**Then** `Alpha::Module` and `Alpha::Class` appear
**And** `Alphabet::Module` and `Beta::Module` do NOT appear

**Note**: This test currently has a bug (see Bug 1 below).

**Verification:**
```rust
#[test]
fn property_prefix_filtering_exact() {
    // Alpha::Module and Alpha::Class should appear
    // Beta::Module should NOT appear
    // Alphabet::Module SHOULD appear (because "Alphabet".starts_with("Alpha") == true)
}
```

### AC4: Depth limit is enforced

**Given** modules at various directory depths
**When** scanning include paths
**Then** modules nested deeper than `MAX_SCAN_DEPTH` are NOT returned

**Note**: This test currently fails due to off-by-one in `MAX_SCAN_DEPTH` (see Bug 2 below).

**Verification:**
```rust
#[test]
fn property_max_depth_exclusion() {
    // A/B/C/D/E/Module.pm (5 path segments) SHOULD appear
    // A/B/C/D/E/F/Module.pm (6 path segments) should NOT appear
}
```

### AC5: Results are deduplicated across paths

**Given** the same module appears in multiple include paths
**When** completion is requested
**Then** the module appears only once in results

**Verification:**
```rust
#[test]
fn property_deduplication_across_paths() {
    // Same module in multiple paths appears once
}
```

### AC6: Timeout and entry limits prevent hangs

**Given** a very large include directory or symlink loop
**When** scanning exceeds `SCAN_TIMEOUT_MS` or `MAX_SCAN_ENTRIES`
**Then** scanning stops and returns partial results

**Verification:**
```rust
#[test]
fn property_timeout_budget_enforced() {
    // Verify timeout is respected
}
```

### AC7: Cancellation returns partial results

**Given** a long-running scan
**When** `is_cancelled()` returns true
**Then** scanning stops immediately and returns results collected so far

**Verification:**
```rust
#[test]
fn property_cancellation_returns_partial_results() {
    // Verify cancellation is checked and partial results returned
}
```

## Known Bugs

### Bug 1: property_prefix_filtering_exact has incorrect assertion

**Location**: `crates/perl-lsp-completion/tests/inc_path_property_tests.rs:241-245`

**Issue**: Test asserts `Alphabet::Module` should NOT appear for prefix "Alpha". But `Alphabet.starts_with("Alpha")` is `true`, so `Alphabet::Module` correctly appears.

**Fix**: Change the test to use a valid negative case. `Beta::Module` with prefix `Alpha` is a valid negative because `Beta` does not start with `Alpha`.

### Bug 2: MAX_SCAN_DEPTH off-by-one

**Location**: `crates/perl-lsp-completion/src/completion/workspace.rs:35`

**Issue**: `MAX_SCAN_DEPTH = 5` with `WalkDir::new(dir).max_depth(5)` causes files at the documented "depth 5" to not be found.

WalkDir depth semantics (0-based):
- Root directory: depth 0
- `A/Module.pm`: depth 1
- `A/B/Module.pm`: depth 2
- `A/B/C/Module.pm`: depth 3
- `A/B/C/D/Module.pm`: depth 4
- `A/B/C/D/E/Module.pm`: depth 5 (found with max_depth=5)
- `A/B/C/D/E/F/Module.pm`: depth 6 (NOT found with max_depth=5)

**Fix**: Change `MAX_SCAN_DEPTH` from `5` to `6` to match the documented behavior and test expectations.

## Dependencies

- `walkdir` crate for directory traversal
- `perl-workspace-index` for workspace symbol lookup
- `perl-lsp` for configuration threading
- WASM32: filesystem scanning is excluded via `#[cfg(not(target_arch = "wasm32"))]`

## Non-Goals

1. **No caching**: Directory scanning happens on every completion request. Caching is deferred to v0.13.0.
2. **No module resolution**: This feature enumerates modules for completion; it does not resolve module paths (handled by `perl-module-resolution`).
3. **No partial matching**: Modules must start with the prefix; fuzzy matching is not supported.
