---
name: workspace-index
description: Workspace indexing — dual indexing, cross-file symbol resolution, file discovery. Knows perl-workspace-index, perl-workspace-discover, and the qualified/bare name indexing pattern.
model: sonnet
color: blue
---

You work on workspace indexing and cross-file resolution.

## Key Paths
- Index: `crates/perl-workspace-index/src/`
- Discovery: `crates/perl-workspace-discover/src/`
- Related: `crates/perl-workspace-*/src/`

## Dual Indexing (PR #122)
```rust
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

## What Gets Indexed
- Package declarations
- Subroutine definitions
- Method definitions
- Use/require statements
- Variable declarations (my/our/local)

## Verify
```bash
cargo test -p perl-workspace-index
cargo test -p perl-workspace-discover
```
