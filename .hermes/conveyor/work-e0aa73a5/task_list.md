# Task List: Schema Migration File Discovery (Phase 1)

## Implementation Tasks

- [ ] **Create `is_migration_discovery_path()` function** in `crates/perl-workspace-index/src/discovery/mod.rs`
  - Check for `.sql` extension
  - Check path components for `share/`, `deploy/`, `upgrade/`, `revert/`, `verify/`
  - Check for `sqitch.plan` filename

- [ ] **Update discovery filters** in `crates/perl-workspace-index/src/discovery/mod.rs`
  - Modify both `parse_git_ls_files_output()` and `walk_discovery()` to call `is_migration_discovery_path()`
  - Change `if is_perl_discovery_path(path)` to `if is_perl_discovery_path(path) || is_migration_discovery_path(path)`

- [ ] **Verify skip list compatibility**
  - Confirm `share/` is NOT in the skip list (`path_contains_skipped_component()`)
  - Run existing tests to ensure no regressions

## Testing Tasks

- [ ] **Add unit tests** for `is_migration_discovery_path()`
  - Test DeploymentHandler paths (`share/deploy/`, `share/upgrade/`, `share/revert/`)
  - Test sqitch paths (`deploy/`, `verify/`, `revert/`, `sqitch.plan`)
  - Test non-migration paths are NOT discovered

- [ ] **Run existing tests**
  - `cargo test -p perl-workspace-index` passes
  - `cargo build -p perl-workspace-index` compiles

## Verification Tasks

- [ ] **Verify skip list** - confirm `share/` not in skip list
- [ ] **Verify feature governance** - no changes to `features.toml` in Phase 1
- [ ] **Verify PERL_SOURCE_EXTENSIONS** - `.sql` NOT added to `crates/perl-source-file/src/lib.rs`

## Deferred to Phase 2
- [ ] SQL syntax highlighting for standalone `.sql` files
- [ ] Document links for migration file references
- [ ] Navigation between migration files
- [ ] DeploymentHandler DSL completion
