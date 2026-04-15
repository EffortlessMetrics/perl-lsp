# Acceptance Criteria — Wave 1 PILOT (perl-module-* → perl-module)

Issue #4420 | Branch: `impl/wave1-perl-module-4420`

All criteria must be satisfied before PR merge.

---

## Structural Changes

- [ ] Crate `crates/perl-module/` exists with Cargo.toml, src/lib.rs, src/api.rs
- [ ] 13 old crate directories deleted: `perl-module-name`, `perl-module-path`, `perl-module-token-core`, `perl-module-boundary`, `perl-module-token`, `perl-module-import`, `perl-module-token-parser`, `perl-module-reference`, `perl-module-import-match`, `perl-module-rename`, `perl-module-resolution`, `perl-module-resolution-path`, `perl-module-resolution-uri`
- [ ] All source files from 13 crates copied into `crates/perl-module/src/` (11 modules + resolution subfolder)
- [ ] All test files from 13 crates copied into `crates/perl-module/tests/`

## Workspace Integration

- [ ] `crates/perl-module/` listed in `[workspace] members`
- [ ] `perl-module` listed in `[workspace.dependencies]` with correct path and version
- [ ] 13 old perl-module-* entries removed from `[workspace] members`
- [ ] 13 old perl-module-* entries removed from `[workspace.dependencies]`
- [ ] Workspace member count is exactly 123 (135 - 13 + 1)

## Publishing

- [ ] `perl-module` listed in `[workspace.metadata.publish].allow` (exactly once)
- [ ] All 13 old perl-module-* entries removed from publish allowlist
- [ ] `cargo xtask publish-closure` shows `perl-module` in output; zero `perl-module-*` crates listed

## External Consumer Updates

- [ ] `perl-lsp` Cargo.toml: replaced 5 perl-module-* deps with single `perl-module` dep
- [ ] `perl-lsp` source (4 files): all imports rewritten to use `perl_module::` prefix
- [ ] `perl-lsp-completion` Cargo.toml: replaced `perl-module-import` with `perl-module`
- [ ] `perl-lsp-completion` source: imports updated
- [ ] `perl-lsp-document-links` Cargo.toml: replaced 2 deps with single `perl-module`
- [ ] `perl-lsp-document-links` source: imports updated
- [ ] `perl-lsp-workspace-symbols` Cargo.toml: replaced `perl-module-path` with `perl-module`
- [ ] `perl-lsp-workspace-symbols` source: imports updated
- [ ] `perl-dap` Cargo.toml: replaced `perl-module-path` with `perl-module`
- [ ] `perl-dap` source: imports updated
- [ ] `perl-refactoring` Cargo.toml: replaced `perl-module-path` with `perl-module`
- [ ] `perl-refactoring` source: imports updated
- [ ] `perl-text-line` test file: imports updated from `perl_module_token::` and `perl_module_token_parser::` to `perl_module::token::` and `perl_module::token_parser::`

## Compilation & Testing

- [ ] `cargo build -p perl-module --lib` succeeds
- [ ] `cargo test -p perl-module --lib` passes (62 tests: name, path, token-core, boundary, token, import, token-parser, reference, import-match, rename, resolution)
- [ ] `cargo build -p perl-lsp-rs --release` succeeds
- [ ] `cargo test -p perl-lsp` passes
- [ ] `just pr-fast` passes (formatting + clippy)

## Code Quality

- [ ] All internal modules default to `pub(crate)` visibility (enforced via no `pub ` on module items outside api.rs)
- [ ] `src/api.rs` re-exports all public items (facade pattern enforced)
- [ ] No dangling `pub` visibility on items meant to be internal
- [ ] All 62 test files updated to use new import paths (no `use perl_module_*::` references)

## Version Alignment

- [ ] `crates/perl-module/Cargo.toml` version set to `0.14.0` (matching major bump for public API change)
- [ ] All 6 consumer crates still build (no version conflicts)

## Final Verification

- [ ] No untracked perl-module-* directories in `crates/`
- [ ] All `use perl_module_*::` references in codebase have been migrated (grep confirms zero matches outside hidden .git/)
- [ ] Ledger `.spec/microcrate-collapse/ledger.md` can be manually updated to mark Wave 1 as "COMPLETE"

---

## Verification Commands (for manual spot-checks)

```bash
# Confirm structure
ls -d crates/perl-module/src/* | wc -l  # should be 11 (modules) + api.rs

# Confirm old crates gone
find crates -maxdepth 1 -name "perl-module-*" -type d | wc -l  # should be 0

# Confirm workspace member count
cargo metadata --no-deps | jq '.workspace_members | length'  # should be 123

# Confirm publish allowlist
grep "perl-module" Cargo.toml | grep -v "crates/perl-module" | wc -l  # should be 1 (just the "allow" entry)

# Confirm tests
cargo test -p perl-module --lib 2>&1 | grep "test result"

# Confirm full build
cargo build -p perl-lsp-rs --release 2>&1 | grep -c "error"  # should be 0
```
