# Specification: ParseResult and parse_perl_summary for tree-sitter-perl-c

## Feature Description

Add a `ParseResult` struct and `parse_perl_summary()` convenience function to the
`tree-sitter-perl-c` crate. This provides a pre-computed summary of a parse result
without requiring callers to import the `tree-sitter` crate directly.

### ParseResult Struct

```rust
pub struct ParseResult {
    pub has_errors: bool,
    pub root_kind: String,
    pub grammar_node_kind_count: usize,  // renamed from `node_kind_count`
    pub sexp: String,
    pub tree: tree_sitter::Tree,
}
```

- `has_errors` — `true` if the parse tree contains an error node
- `root_kind` — the `kind()` string of the root node (e.g., `"source_file"`)
- `grammar_node_kind_count` — the total number of distinct node kinds registered
  in the grammar (from `language().node_kind_count()`); a grammar-level constant,
  not a per-tree node count
- `sexp` — the S-expression representation of the root node
- `tree` — the raw `tree_sitter::Tree` for advanced operations

### parse_perl_summary Function

```rust
pub fn parse_perl_summary(code: &str) -> Result<ParseResult, Box<dyn std::error::Error>>
```

Calls `parse_perl_code(code)?` internally and populates a `ParseResult` from the
resulting tree.

## Acceptance Criteria

1. **`parse_perl_summary("my $x = 42;")`** returns a `ParseResult` where:
   - `has_errors` is `false`
   - `root_kind` is `"source_file"`
   - `grammar_node_kind_count` is a positive integer (grammar constant > 0)
   - `sexp` starts with `"(source_file"`
   - `tree.root_node().kind()` equals `"source_file"`

2. **`parse_perl_summary("my $x = ;")`** (invalid Perl) returns a `ParseResult`
   where `has_errors` is `true`.

3. **`parse_perl_summary` is backward-compatible** — `parse_perl_code()` and
   `parse_perl_file()` signatures are unchanged. Existing callers compile without
   modification.

4. **`parse_perl_summary` includes a doc-test** demonstrating basic usage.

5. **`cargo test -p tree-sitter-perl-c`** passes all existing and new tests.

6. **`cargo doc -p tree-sitter-perl-c --no-deps`** generates documentation without
   warnings.

7. **`cargo xtask fmt && cargo clippy -p tree-sitter-perl-c --tests`** passes with
   no warnings.

## Non-Goals

- This does NOT introduce a new parsing engine or alternative parser.
- This does NOT modify `parse_perl_code()` or `parse_perl_file()` signatures.
- This does NOT add new crate dependencies.
- This does NOT add tree-sitter query helpers (those belong in the `tree-sitter` crate).
- This does NOT change the crate's role as a compatibility baseline and benchmarking reference.

## Dependencies

- No new dependencies. Uses only `tree-sitter = "0.26.6"` (existing).
- No changes to `Cargo.toml` are required.

## File Changes

| File | Change |
|------|--------|
| `crates/tree-sitter-perl-c/src/lib.rs` | Add `ParseResult` struct, `parse_perl_summary()` function, 3 unit tests |

## Test Plan

Three unit tests added to the existing `#[cfg(test)] mod tests` block in `src/lib.rs`:

1. `test_parse_perl_summary_valid_code` — parses `"my $x = 42;"`, asserts `!has_errors`,
   `root_kind == "source_file"`, `grammar_node_kind_count > 0`, `sexp.starts_with("(source_file")`

2. `test_parse_perl_summary_invalid_code` — parses `"my $x = ;"`, asserts `has_errors`

3. `test_parse_perl_summary_tree_escape_hatch` — parses `"sub foo { 42 }"`, verifies
   `tree.root_node().kind() == "source_file"`

## Verification Commands

```bash
cargo test -p tree-sitter-perl-c
cargo doc -p tree-sitter-perl-c --no-deps
cargo xtask fmt && cargo clippy -p tree-sitter-perl-c --tests
```
