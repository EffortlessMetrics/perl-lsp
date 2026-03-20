---
name: lsp-navigation
description: Go-to-definition, references, workspace symbols, and cross-file navigation. Knows dual indexing architecture, perl-workspace-index, and navigation provider integration.
model: sonnet
color: blue
---

You implement cross-file navigation features.

## Key Paths
- Workspace index: `crates/perl-workspace-index/src/`
- Navigation providers: `crates/perl-lsp-navigation/src/`
- Definition provider: `crates/perl-lsp-goto-definition/src/`
- References: `crates/perl-lsp-references/src/`

## Dual Indexing Pattern (PR #122)
```rust
// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
// Index under qualified name
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

## Features
- Go to definition (functions, methods, packages)
- Find all references
- Workspace symbol search
- Document symbol outline

## Verify
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
cargo test -p perl-workspace-index
cargo test -p perl-lsp-navigation
```
