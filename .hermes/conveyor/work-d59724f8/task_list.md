# Task List: @INC Paths in Module Completion

## Implementation Tasks

- [ ] 1. Modify `CompletionProvider` in `crates/perl-lsp-completion/src/completion.rs`
  - Add `include_paths: Vec<PathBuf>` and `system_inc_paths: Vec<PathBuf>` fields
  - Create new constructor `new_with_index_and_source_with_inc()` accepting include paths
  - Keep existing `new_with_index_and_source()` with empty paths for backward compatibility

- [ ] 2. Implement directory scanning in `crates/perl-lsp-completion/src/completion/workspace.rs`
  - Add `scan_directory_for_modules()` helper function
  - Add `path_to_module_name()` helper for path → module name conversion
  - Update `add_use_module_completions()` to accept and use include paths
  - Implement max depth (8), file matching (.pm only), and symlink cycle detection

- [ ] 3. Wire include paths in `crates/perl-lsp/src/runtime/language/completion.rs`
  - Fix Step 4 to use `effective_include_paths(&perl5lib_paths)` instead of `include_paths_for_doc()`
  - Parse `PERL5LIB` environment variable using `WorkspaceConfig::parse_perl5lib()`
  - Get system @INC paths when `useSystemInc: true`
  - Pass paths to the new `CompletionProvider` constructor

- [ ] 4. Add caching strategy
  - Cache scoped to `CompletionProvider` instance (invalidated by config change)
  - Cache key: include path directory
  - Cache value: sorted list of module names
  - Limit results to 20 per include path

- [ ] 5. Update sort tiering
  - External modules use tier 2 sort text prefix `"2_"`
  - Verify workspace modules (tier 1) sort before external (tier 2)

- [ ] 6. Add tests
  - Test module completion finds modules in include paths
  - Test module completion finds PERL5LIB modules
  - Test module completion finds system @INC modules when enabled
  - Test prefix matching works correctly (e.g., `DB` → `DBI`, `DBD::MySQL`)
  - Test deduplication with workspace symbols
  - Test sort ordering is correct (workspace before external)
  - Test performance: completion returns within 100ms for cached results
