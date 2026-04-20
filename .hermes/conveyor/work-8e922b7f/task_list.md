# Task List — work-8e922b7f

## Phase 1: Dependency Metadata Parsing (New Crate)
- [ ] Create `crates/perl-dependency-metadata/Cargo.toml` with `serde_yaml`, `serde_json`, `regex` dependencies
- [ ] Implement `cpanfile` parser (regex-based, handles `requires 'Module', 'version';`)
- [ ] Implement `META.json` parser using `serde_json`
- [ ] Implement `META.yml` parser using `serde_yaml` with graceful fallback
- [ ] Implement vendor path detection via directory existence (`vendor/lib/perl5`, `local/lib/perl5`)
- [ ] Export `DependencyInfo { declared, vendor_path }` struct

## Phase 2: Auto-includePaths Integration
- [ ] Extend `perl-lsp-config/src/lib.rs` to scan for cpanfile/META.json/META.yml in workspace root
- [ ] Call `perl-dependency-metadata` to get vendor paths
- [ ] Augment `effective_include_paths()` with detected vendor path
- [ ] Add `vendor_path: Option<PathBuf>` field to `WorkspaceConfig`
- [ ] Add `serde_yaml` to `perl-lsp-config/Cargo.toml` dependencies

## Phase 3: Missing Dependency Diagnostics Enhancement
- [ ] Modify `perl-lsp-diagnostics/src/lints/missing_module.rs` to accept `DependencyInfo`
- [ ] Implement two-stage resolver: check cpanfile first, suppress PL701 if declared
- [ ] Explicitly scope to `use` statements only (no `require` handling)
- [ ] Wire `perl-dependency-metadata` into diagnostic pipeline

## Phase 4a: Cpanfile Editing Utility
- [ ] Build cpanfile editor in `perl-dependency-metadata`: read, parse, insert in alphabetical order
- [ ] Preserve existing formatting and comments
- [ ] Handle edge case: cpanfile doesn't exist → create it

## Phase 4b: Code Action to Add Missing Dependency
- [ ] Register `CodeActionKind::QuickFix` handler for PL701 in `perl-lsp-code-actions`
- [ ] Implement quick-fix that calls Phase 4a utility
- [ ] Apply updated cpanfile via workspace edit

## Testing
- [ ] Add unit tests for cpanfile parser in `perl-dependency-metadata`
- [ ] Add unit tests for META.json and META.yml parsers
- [ ] Add integration test in `ux_scenario` fixtures (cpanfile + missing module)
- [ ] Add test for alphabetical insertion in cpanfile editing

## Documentation
- [ ] Update `docs/reference/CONFIG.md` with auto-detection behavior
- [ ] Document `perl.include_paths` disable mechanism
