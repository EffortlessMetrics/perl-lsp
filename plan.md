1. **Update `WorkspaceConfig`**:
   - `crates/perl-lsp-config/src/lib.rs`: Edit `WorkspaceConfig` to add `perl_cmd: String` (default `"perl"`), and update `update_from_value` to parse `perlCmd`.
   - Update `WorkspaceConfig` to store `system_inc_cache: Option<EffectiveIncMetadata>`, where `EffectiveIncMetadata` contains `inc: Vec<PathBuf>`, `archname: String`, and `version: String`. (Wait, maybe just `archname` and `version` fields directly?). Yes, `interpreter_metadata: Option<InterpreterMetadata>`.
   - Implement `fetch_interpreter_metadata(&self)` which calls `Command::new(&self.perl_cmd).args(["-MConfig", "-e", "print join(\"\\n\", $Config{archname}, $Config{version}, @INC)"]).output()`.

2. **Add `EffectiveInc`**:
   - `crates/perl-module-resolution/src/effective_inc.rs`: Create `EffectiveInc` struct and logic to aggregate and correctly order include paths, honoring `use lib`, `no lib`, relative and absolute configured paths, and interpreter startup `@INC` paths (if enabled). Handle arch/version expansions.

3. **Update module resolution consumers**:
   - `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs`: Update `resolve_module_path`, `resolve_module_path_with_uri`, and `resolve_module_to_path_with_doc` to use the `EffectiveInc` struct directly. Use `EffectiveInc::build(...)` to construct the unified path list, then just loop over it using `validate_workspace_path` logic (for relative paths) and direct existence check for absolute paths (which can be inside the `EffectiveInc::build` or inside a refactored `resolve_module_path`). Wait, we already refactored `resolve_module_path`! So `EffectiveInc::build` can just return `include_paths: Vec<String>`! And we can just pass that into `resolve_module_path`!

4. **Verify tests pass**:
   - Run `cargo test -p perl-module-resolution` and `cargo test -p perl-module-resolution-path` and `cargo test -p perl-module-resolution-uri`.
   - Run workspace tests: `cargo test`.

5. **Pre-commit**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
