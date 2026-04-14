# Task List — work-d716ca36: @INC Paths Not Used in Module Completion

## Implementation Tasks

### Task 1: Add fields to `CompletionProvider` struct
- [ ] **File**: `crates/perl-lsp-completion/src/completion.rs`
- [ ] Add `include_paths: Vec<PathBuf>` field to `CompletionProvider`
- [ ] Add `system_inc_paths: Vec<PathBuf>` field to `CompletionProvider`
- [ ] Update `new_with_index_and_source()` to accept these parameters
- [ ] Update `new_with_index()` to pass empty vecs
- [ ] **Verification**: `cargo build --package perl-lsp-completion` succeeds

### Task 2: Wire config through LSP handler
- [ ] **File**: `crates/perl-lsp/src/runtime/language/completion.rs`
- [ ] Get `include_paths` and `use_system_inc` from `self.workspace_config.lock()`
- [ ] If `use_system_inc`, get system paths via `config.get_system_inc()` (requires re-locking)
- [ ] Pass paths to `CompletionProvider::new_with_index_and_source()`
- [ ] **Verification**: LSP starts without errors

### Task 3: Extend `add_use_module_completions()` signature
- [ ] **File**: `crates/perl-lsp-completion/src/completion/workspace.rs`
- [ ] Add `include_paths: &[PathBuf]` parameter
- [ ] Add `system_inc_paths: &[PathBuf]` parameter
- [ ] Add `is_cancelled: &dyn Fn() -> bool` parameter
- [ ] **Verification**: `cargo build` succeeds

### Task 4: Implement `scan_directory_for_modules()` helper
- [ ] **File**: `crates/perl-lsp-completion/src/completion/workspace.rs`
- [ ] Use `walkdir::WalkDir` with `max_depth(3)` and `follow_links(false)`
- [ ] Implement 30ms timeout using `std::time::Instant`
- [ ] Implement max 10,000 entries cap
- [ ] Check `is_cancelled()` at each directory entry
- [ ] Convert `File/Path/To/Module.pm` → `File::Path::To::Module`
- [ ] Filter by prefix match
- [ ] Use `"(include)"` and `"(system)"` detail text
- [ ] **Verification**: Unit test with temp directory

### Task 5: Update call site in `get_completions()`
- [ ] **File**: `crates/perl-lsp-completion/src/completion.rs`
- [ ] Pass `&self.include_paths` and `&self.system_inc_paths` to `add_use_module_completions()`
- [ ] Pass cancellation callback
- [ ] **Verification**: `cargo build` succeeds

### Task 6: WASM32 graceful degradation
- [ ] **File**: `crates/perl-lsp-completion/src/completion/workspace.rs`
- [ ] Wrap filesystem scanning in `#[cfg(not(target_arch = "wasm32"))]`
- [ ] On wasm32, skip scanning (same as before)
- [ ] **Verification**: Builds on wasm32-unknown-unknown target

### Task 7: Add tests
- [ ] **File**: `crates/perl-lsp-completion/tests/completion_behavior_tests.rs`
- [ ] `test_use_module_completions_includes_external_modules` (AC1)
- [ ] `test_use_module_completions_system_inc` (AC2)
- [ ] `test_use_module_completions_dedup_workspace_and_external` (AC3)
- [ ] `test_use_module_completions_prefix_filtering` (AC4)
- [ ] `test_use_module_completions_nested_external` (AC5)
- [ ] `test_use_module_completions_empty_paths` (AC6)
- [ ] `test_use_module_completions_cancellation` (AC7)
- [ ] `test_use_module_completions_permission_errors` (AC9)
- [ ] `cargo test --package perl-lsp-completion` passes

### Task 8: Run full gate
- [ ] `cargo xtask fmt`
- [ ] `just pr-fast` or `cargo test`
- [ ] `nix develop -c just ci-gate` (canonical gate)

## Follow-up Tasks (Out of Scope for This PR)

- [ ] File issue for `use lib` dynamic extraction from document text
- [ ] Consider caching infrastructure for module lists (v0.13.0+)
- [ ] Consider shared `scan_modules_in_paths()` utility

## Files Modified

| File | Changes |
|------|---------|
| `crates/perl-lsp-completion/src/completion.rs` | Add fields to `CompletionProvider`, wire config |
| `crates/perl-lsp-completion/src/completion/workspace.rs` | Extend signature, implement scanner |
| `crates/perl-lsp/src/runtime/language/completion.rs` | Pass config to provider |
| `crates/perl-lsp-completion/tests/completion_behavior_tests.rs` | Add 7+ tests |
