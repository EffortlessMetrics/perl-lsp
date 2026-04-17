# Specification: Schema Migration File Discovery (Phase 1)

## Feature/Behavior Description

Enable the perl-lsp workspace index to discover database migration files used by Perl schema management tools:
- **DBIx::Class::DeploymentHandler**: `share/deploy/`, `share/upgrade/`, `share/revert/` directories containing `.sql` files
- **sqitch**: `deploy/`, `verify/`, `revert/` directories containing `.sql` files, and `sqitch.plan` files

The workspace index is responsible for finding files in the workspace. Once migration files are discovered, they appear in the IDE file tree and can be opened, but do NOT receive Perl-specific LSP features (semantic highlighting, completion, etc.) in Phase 1.

## Acceptance Criteria

### AC1: DeploymentHandler Directory Discovery
When `is_perl_discovery_path()` or `is_migration_discovery_path()` evaluates a path, it returns `true` for:
- `**/share/deploy/**/*.sql`
- `**/share/upgrade/**/*.sql`
- `**/share/revert/**/*.sql`

**Verification**: Unit test confirms these paths pass the migration discovery filter.

### AC2: Sqitch Directory and File Discovery
When `is_migration_discovery_path()` evaluates a path, it returns `true` for:
- `**/deploy/**/*.sql`
- `**/verify/**/*.sql`
- `**/revert/**/*.sql`
- `**/sqitch.plan`

**Verification**: Unit test confirms these paths pass the migration discovery filter.

### AC3: Non-Migration SQL Files NOT Discovered
When `is_migration_discovery_path()` evaluates a path, it returns `false` for:
- `**/sql/**/*.sql` (generic SQL directory)
- `**/scripts/**/*.sql` (database scripts)
- `**/fixtures/**/*.sql` (test fixtures)
- `**/docs/**/*.sql` (documentation)

**Verification**: Unit test confirms these paths do NOT pass the migration discovery filter.

### AC4: Skip List Compatibility
Migration directories are NOT skipped by `path_contains_skipped_component()`. Specifically, `share/` is not in the skip list.

**Verification**: Unit test confirms `is_migration_discovery_path()` works correctly with the existing skip list.

### AC5: Separate Discovery Path
`.sql` is NOT added to `PERL_SOURCE_EXTENSIONS` in `crates/perl-source-file/src/lib.rs`.

**Verification**: `PERL_SOURCE_EXTENSIONS` contains only Perl-related extensions.

### AC6: Feature Governance
No new feature entries are added to `features.toml` for Phase 1 file discovery.

**Verification**: `features.toml` unchanged (migration discovery is not an LSP feature yet).

## Non-Goals

### Not in Phase 1
1. **SQL syntax highlighting** for standalone `.sql` files — requires a separate tokenization pipeline
2. **Document links** for migration file references — requires new detection pattern
3. **Navigation** between migration files — requires commands and UI
4. **DeploymentHandler DSL completion** — Phase 3

### Why These Are Deferred
- SQL highlighting requires Perl AST pipeline modifications that don't apply to non-Perl files
- Document links require new inline path detection patterns
- Both are more complex than file discovery and belong in Phase 2 with proper design

## Technical Approach

### New Function: `is_migration_discovery_path()`

Location: `crates/perl-workspace-index/src/discovery/mod.rs`

```rust
/// Returns true if the path is a database migration file
/// used by DBIx::Class::DeploymentHandler, sqitch, or similar tools.
///
/// This is separate from `is_perl_discovery_path()` to keep
/// the Perl source concept clean.
pub fn is_migration_discovery_path(path: &Path) -> bool {
    // Check for .sql extension
    let is_sql = path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"));

    if !is_sql {
        // Check for sqitch.plan
        return path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("sqitch.plan"));
    }

    // Check path components for migration directory patterns
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy_lowercase();
        s == "share" || s == "deploy" || s == "upgrade"
            || s == "revert" || s == "verify"
    })
}
```

### Modified Discovery Logic

Location: `crates/perl-workspace-index/src/discovery/mod.rs`

Change the discovery filter from:
```rust
if is_perl_discovery_path(path) { ... }
```

To:
```rust
if is_perl_discovery_path(path) || is_migration_discovery_path(path) { ... }
```

This applies to both `parse_git_ls_files_output()` (line ~99) and `walk_discovery()` (line ~126).

### Files to Modify

| File | Change |
|------|--------|
| `crates/perl-workspace-index/src/discovery/mod.rs` | Add `is_migration_discovery_path()`, update discovery filters |

### Files NOT Modified in Phase 1

| File | Reason |
|------|--------|
| `crates/perl-source-file/src/lib.rs` | `.sql` NOT added to `PERL_SOURCE_EXTENSIONS` |
| `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs` | SQL highlighting deferred to Phase 2 |
| `crates/perl-lsp-document-links/src/lib.rs` | Document links deferred to Phase 2 |
| `features.toml` | No new features for file discovery alone |

## Test Cases

### Unit Tests for `is_migration_discovery_path()`

```rust
#[test]
fn test_deployment_handler_paths() {
    let cases = [
        "share/deploy/1.001/001-auto.sql",
        "share/deploy/1.001/001.sql",
        "share/upgrade/1.001-1.002/001-auto.sql",
        "share/revert/1.002-1.001/001.sql",
    ];
    for path in cases {
        assert!(
            is_migration_discovery_path(Path::new(path)),
            "should discover: {}",
            path
        );
    }
}

#[test]
fn test_sqitch_paths() {
    let cases = [
        "deploy/20230101_initial.sql",
        "verify/20230101_initial.sql",
        "revert/20230101_initial.sql",
        "sqitch.plan",
        "db/core/sqitch.plan",
    ];
    for path in cases {
        assert!(
            is_migration_discovery_path(Path::new(path)),
            "should discover: {}",
            path
        );
    }
}

#[test]
fn test_non_migration_paths() {
    let cases = [
        "sql/migrations/001.sql",
        "scripts/cleanup.sql",
        "fixtures/test_data.sql",
        "docs/schema.sql",
    ];
    for path in cases {
        assert!(
            !is_migration_discovery_path(Path::new(path)),
            "should NOT discover: {}",
            path
        );
    }
}
```

## Verification Commands

```bash
cargo build -p perl-workspace-index
cargo test -p perl-workspace-index
```

## Edge Cases

1. **Case sensitivity**: Path component comparison uses `to_string_lossy_lowercase()` for case-insensitive matching
2. **Nested directories**: `db/core/sqitch.plan` works because it checks all path components
3. **Very deep paths**: Works because it checks all components, not just top-level
4. **Non-standard extensions**: `.mysql`, `.pgsql` NOT discovered in Phase 1

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `share/` conflicts with skip list | Medium | Would prevent all DeploymentHandler discovery | Verify `share/` not in skip list before shipping |
| Fragile path matching | High | May miss unusual migration directory structures | Accept for Phase 1; Phase 2 can improve detection |
| sqitch.plan in unusual locations | Medium | May miss nested sqitch projects | Accept; this is an MVP limitation |

## Future Phases

### Phase 2: Navigation and Document Links
- Document links for migration file references in Perl code
- Commands to jump between deploy/upgrade/revert for same version
- `sqitch.plan` parsing for project-aware navigation

### Phase 3: DSL Completion
- Completion for `schema_version()`, `database_version()`, etc.
- Completion for DeploymentHandler configuration options
