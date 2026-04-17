# Specifications: Public Query Helpers for tree-sitter-perl-c

## Feature Description

Add a public `queries` submodule to the `tree-sitter-perl-c` crate that exposes
tree-sitter query file contents via two types of functions:

1. **Raw string accessors** (`*_query_str()`) — return `&'static str` for callers who want
   to construct `tree_sitter::Query` themselves
2. **Convenience constructors** (`load_*_query()`) — return `Result<tree_sitter::Query, QueryError>`
   and wrap the `Query::new(&language(), QUERY_STR)?` pattern

The module exposes all four upstream query files from `tree-sitter-perl/queries/`:
`injections.scm`, `highlights.scm`, `folds.scm`, and `matchup.scm`.

## Acceptance Criteria

### AC1: Public API Surface
- [ ] `crates/tree-sitter-perl-c/src/queries.rs` exists and defines eight public functions:
  - `pub fn injections_query_str() -> &'static str`
  - `pub fn highlights_query_str() -> &'static str`
  - `pub fn folds_query_str() -> &'static str`
  - `pub fn matchup_query_str() -> &'static str`
  - `pub fn load_injections_query() -> Result<Query, QueryError>`
  - `pub fn load_highlights_query() -> Result<Query, QueryError>`
  - `pub fn load_folds_query() -> Result<Query, QueryError>`
  - `pub fn load_matchup_query() -> Result<Query, QueryError>`
- [ ] `pub use tree_sitter::QueryError;` is re-exported from the `queries` module
- [ ] `pub mod queries;` is added to `src/lib.rs` (making `tree_sitter_perl_c::queries::*` accessible)

### AC2: Compilation and Path Correctness
- [ ] All four `include_str!("../../../tree-sitter-perl/queries/<file>.scm")` paths compile without error
- [ ] `cargo build -p tree-sitter-perl-c` succeeds
- [ ] `cargo doc -p tree-sitter-perl-c --no-deps` succeeds (no warnings on new public API)

### AC3: Elimination of Duplication
- [ ] `src/lib.rs` no longer contains a private `const INJECTIONS_QUERY` with `include_str!` (inside `#[cfg(test)]` or otherwise)
- [ ] `tests/bdd_workflows.rs` uses `tree_sitter_perl_c::queries::injections_query_str()` instead of its own `include_str!` call at lines 126 and 168
- [ ] No `include_str!` referencing `tree-sitter-perl/queries/` remains in `tests/bdd_workflows.rs`

### AC4: Test Coverage
- [ ] `crates/tree-sitter-perl-c/tests/queries.rs` exists as an integration test file
- [ ] Tests verify all four `*_query_str()` functions return non-empty strings
- [ ] Tests verify all four `load_*_query()` functions return `Ok(query)` (successful `Query` construction)
- [ ] `cargo test -p tree-sitter-perl-c` passes (all existing tests + new tests)

### AC5: ROADMAP.md Updated
- [ ] `ROADMAP.md` lines 27-28 (the "does not expose tree-sitter query helpers" bullet) are removed or rephrased to reflect the new capability
- [ ] The "Known limitations vs. upstream grammar" section no longer lists the query helpers gap

### AC6: No Breaking Changes
- [ ] All existing public API (`language()`, `try_create_parser()`, `create_parser()`, `parse_perl_code()`, `parse_perl_file()`, `get_scanner_config()`) remains unchanged
- [ ] `cargo clippy -p tree-sitter-perl-c --tests` passes with no new warnings

## Non-Goals

- This change does **not** modify the contents of any `.scm` query file in `tree-sitter-perl/queries/`
- This change does **not** add query-based features (syntax highlighting, code folding, tag matching) to the crate or to `perl-lsp`
- This change does **not** add `tree-sitter` as a new dependency (uses existing `tree-sitter = "0.26.6"`)
- This change does **not** affect other crates in the workspace (e.g., `perl-parser`)
- This change does **not** validate query correctness beyond successful `Query::new()` construction

## Dependencies

| Dependency | Version | Role |
|------------|---------|------|
| `tree-sitter` | `0.26.6` | `Query`, `QueryError`, `Language`, `QueryCursor` types (already present) |
| `cc` | (build) | Compiles vendored C sources (already present, no change) |

No new dependencies are introduced. All required types (`Query`, `QueryError`) are available from the existing `tree-sitter` dependency.

## Implementation Notes

### Path Resolution
`include_str!("../../../tree-sitter-perl/queries/<file>.scm")` resolves relative to the crate root (`crates/tree-sitter-perl-c/`). From `src/queries.rs`, three levels up reaches the workspace root where `tree-sitter-perl/` lives. This is the same path already verified at `src/lib.rs:163`.

### Test Organization
Integration tests live in `tests/queries.rs` (separate binary), following the existing pattern established by `tests/bdd_workflows.rs`. Unit tests within `src/queries.rs` (using `#[cfg(test)]`) are also acceptable.

### Error Handling
`load_*_query()` functions return `Result<Query, QueryError>` to propagate query syntax errors to callers. The underlying `Query::new()` call uses the `?` operator, making the error case explicit and forcing callers to handle it.

### ROADMAP Update
The `ROADMAP.md` update (AC5) is a required part of this work item, not optional cleanup. The "known limitation" bullet must be removed or rephrased for the work to be considered complete.
