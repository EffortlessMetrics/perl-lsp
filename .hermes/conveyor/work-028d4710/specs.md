# Spec: `PerlLanguage` Descriptor, `language()` Function, and `LANGUAGE` Constant

## Feature Description
Add a native Rust `PerlLanguage` descriptor struct to `tree-sitter-perl-rs` that exposes node kind metadata via `language()` and `LANGUAGE`, following the tree-sitter ecosystem convention. This completes Phase 2 gap 6/6.

## Background
`tree-sitter-perl-rs` intentionally excludes the `tree-sitter` crate and C toolchain, making `tree_sitter::Language` (an opaque C FFI type) impossible to implement natively. The `PerlLanguage` descriptor provides an informational API for Rust-native tooling without claiming tree-sitter C compatibility.

## Behavior

### `PerlLanguage` struct
A public struct holding a static reference to node kind names:
- Field: `kind_names: &'static [&'static str]`
- Constructed once as the `LANGUAGE` singleton

### `impl PerlLanguage`
Three query methods:
- `node_kind_count() -> usize` — returns `self.kind_names.len()`
- `node_kind_names() -> &[&'static str]` — returns all kind names (alphabetical order, from `ALL_KIND_NAMES`)
- `node_kind_is_named(kind: &str) -> bool` — returns `self.kind_names.contains(&kind)`

### `language()` function
Returns `LANGUAGE` (the singleton `PerlLanguage`). Follows tree-sitter ecosystem convention for discoverability.

### `LANGUAGE` constant
`static LANGUAGE: PerlLanguage = PerlLanguage { kind_names: perl_ast::NodeKind::ALL_KIND_NAMES };`

## Acceptance Criteria

1. **`PerlLanguage` struct is public and constructible** — `tree_sitter_perl_rs::PerlLanguage` is accessible and `LANGUAGE` can be used in downstream code.
2. **`language()` returns the `LANGUAGE` singleton** — calling `language()` returns the same `PerlLanguage` instance as the `LANGUAGE` constant.
3. **`node_kind_count()` returns a non-zero count** — `LANGUAGE.node_kind_count() > 0` (verified by test).
4. **`node_kind_names()` includes "Program"** — `LANGUAGE.node_kind_names().contains(&"Program")` (verified by test).
5. **`node_kind_is_named()` correctly distinguishes** — returns `true` for `"Program"`, `false` for `"__nonexistent_kind__"` (verified by test).
6. **Doc comment distinguishes from `tree_sitter::Language`** — the struct-level doc states this is NOT `tree_sitter::Language`, cannot be used with `tree_sitter::Parser::set_language`, and directs users to `tree-sitter-perl-c` for drop-in compatibility.
7. **Three BDD tests added to `behavior_spec_tests.rs`** — tests for: non-zero kind count, "Program" presence, named kind discrimination.
8. **All tests pass** — `cargo test -p tree-sitter-perl-rs` is green after the change.
9. **Clippy clean** — `cargo clippy -p tree-sitter-perl-rs --tests` passes with no warnings.
10. **Fmt clean** — `cargo xtask fmt` produces no diff.

## Non-Goals
- This does NOT implement `tree_sitter::Language` (impossible without C FFI)
- This does NOT add a dependency on `tree-sitter-perl-c`
- This does NOT modify `perl-ast` or any other crate
- This does NOT implement any other Phase 2 gap (gaps 1-5 are separate work items)
- This does NOT provide predicate/query API

## Dependencies
- `perl-ast` (already a dependency): provides `NodeKind::ALL_KIND_NAMES`
- `perl-parser-core` (already a dependency): no new requirements