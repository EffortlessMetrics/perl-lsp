# Snapshot Testing Guide

This project uses [`insta`](https://docs.rs/insta) for snapshot testing. Snapshots
capture baseline output for AST structure, error messages, and LSP capabilities so
that unintentional changes surface as explicit diffs during code review.

## What is Snapshot Testing?

A snapshot test captures the exact output of a function call and stores it in a
`.snap` file checked into the repository. On subsequent runs the output is compared
to the stored snapshot. If they differ, the test fails and the developer must
explicitly approve the change.

This means:

- **Silent regressions are impossible.** Any change to AST format, error messages,
  or LSP capabilities produces a failing test with a clear diff.
- **Intentional changes are visible.** When the parser improves or capabilities
  expand, reviewers see the exact change in the PR diff.
- **Baselines are always current.** The `.snap` files represent the ground truth
  of what the system produces today.

## Snapshot Files

### Parser AST Snapshots — `crates/perl-parser/tests/`

| Test file | Snapshots directory | Coverage |
|-----------|-------------------|----------|
| `tests/ast_snap.rs` | `tests/snapshots/ast_snap__*.snap` | AST sexp, error recovery AST, error messages, semantic token legend |

These tests cover:
- **Clean Perl AST** (15 inputs): Variable declarations, sub definitions, packages,
  control flow, regex, arrays, hashes, method calls, string interpolation, closures.
- **Error recovery AST** (10 inputs): Missing semicolons, unclosed blocks, missing
  right-hand sides, unclosed parens, truncated data structures.
- **Error messages** (4 inputs): The formatted error string for each malformed input.
- **Semantic token legend** (3 snapshots): Token type ordering, modifier ordering,
  and the full index-to-name mapping table.

### LSP Capability Snapshots — `crates/perl-lsp-rs/tests/`

| Test file | Snapshots directory | Coverage |
|-----------|-------------------|----------|
| `tests/lsp_features_snapshot_test.rs` | `tests/snapshots/lsp_features_snapshot_test__*.snap` | Advertised feature catalog vs server caps |
| `tests/lsp_cap_snap.rs` | `tests/snapshots/lsp_cap_snap__*.snap` | Full server capabilities, code action kinds, completion triggers, semantic token legend |
| `tests/lsp_workspace_symbol_snap.rs` | `tests/snapshots/lsp_workspace_symbol_snap__*.snap` | Workspace symbol query results, native class/method symbol shape, workspace symbol capability shape |

The `lsp_cap_snap.rs` tests cover:
- **Minimal client capabilities**: Capabilities advertised when the client declares no optional features.
- **Full client capabilities**: Capabilities advertised to a fully-capable client.
- **Code action kinds**: The set of code action categories (quickfix, refactor, etc.).
- **Completion trigger characters**: Characters that trigger autocompletion.
- **Semantic token legend**: The `tokenTypes` and `tokenModifiers` arrays. These arrays
  are index-encoded — any reordering is a **breaking change** for connected clients.
- **Server info name**: The server identity string.

## Running Snapshot Tests

```bash
# Parser AST snapshots
cargo test -p perl-parser --test ast_snap

# LSP capability snapshots
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_cap_snap -- --test-threads=2

# Workspace symbol snapshots
cargo test -p perl-lsp-rs --test lsp_workspace_symbol_snap

# All snapshot tests (combined)
cargo test -p perl-parser --test ast_snap
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_cap_snap -- --test-threads=2
cargo test -p perl-lsp-rs --test lsp_workspace_symbol_snap
```

## Updating Snapshots After Intentional Changes

When parser output, error messages, or LSP capabilities change intentionally:

```bash
# Accept all pending new/changed snapshots automatically
INSTA_UPDATE=unseen cargo test -p perl-parser --test ast_snap
INSTA_UPDATE=unseen RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_cap_snap -- --test-threads=2
INSTA_UPDATE=unseen cargo test -p perl-lsp-rs --test lsp_workspace_symbol_snap

# Or use cargo-insta for interactive review (recommended for large changes)
cargo install cargo-insta
cargo insta review
```

`cargo insta review` launches an interactive UI that shows each pending snapshot
diff and lets you accept or reject individual changes. This is the recommended
workflow when multiple snapshots change at once.

## PR Workflow

1. Make your change.
2. Run `cargo test -p perl-parser --test ast_snap` (and the LSP test).
3. If snapshot tests fail, inspect the diff — is this change expected?
4. If expected: run `INSTA_UPDATE=unseen cargo test ...` or `cargo insta review`.
5. Commit both the code change and the updated `.snap` files together.
6. The PR diff will show the exact before/after for reviewers.

## When Snapshots Catch a Bug

If a snapshot test fails after a code change and you did not intend the output to
change, the snapshot is doing its job. Do not update the snapshot — fix the code.

Common causes of unexpected snapshot failures:
- Parser refactor changed s-expression structure.
- Error message wording was changed without updating callers.
- A new LSP capability was added but existing capability ordering shifted.
- The semantic token legend indices were reordered (breaking change).

## Semantic Token Legend — Special Care

The `tokenTypes` and `tokenModifiers` arrays in the LSP semantic token legend are
**index-encoded**. Clients receive integer indices in the `semanticTokens/full`
response and decode them using the legend received at initialization. This means:

- **Appending** new token types or modifiers is safe.
- **Reordering** any existing entry is a breaking change that silently miscolors tokens in all clients.
- **Removing** an entry is a breaking change.

The snapshots `ast_snap__semantic_token_legend_index_mapping.snap`
and `lsp_cap_snap__semantic_tokens_legend_from_capabilities.snap`
guard against accidental reordering.

## Adding New Snapshot Tests

Follow these conventions:

1. Add the test in the appropriate `tests/` file (`ast_snap.rs` for parser,
   `lsp_cap_snap.rs` for LSP capabilities).
2. Use `insta::assert_snapshot!` for plain text and `insta::assert_yaml_snapshot!`
   for structured data.
3. Run with `INSTA_UPDATE=unseen` to generate the initial snapshot.
4. Commit the new `.snap` file alongside the test.

Example (parser AST):

```rust
use insta::assert_snapshot;
use perl_parser::Parser;

#[test]
fn ast_my_construct() {
    let mut parser = Parser::new("my $x = 42;");
    let output = parser.parse_with_recovery();
    assert_snapshot!(output.ast.to_sexp());
}
```

Example (LSP capability):

```rust
use insta::assert_yaml_snapshot;
use serde_json::json;

mod support;
use support::lsp_harness::LspHarness;

#[test]
fn snapshot_my_new_capability() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(json!({})))?;
    let caps = &init_result["capabilities"];
    assert_yaml_snapshot!("my_new_capability", caps.get("myNewProvider"));
    Ok(())
}
```
