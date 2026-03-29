# tree-sitter-perl

Pure-Rust Perl parser and comparison harness for the workspace.

This crate is not published to crates.io. Use it for parser work, regression
checks, and benchmark comparisons against the native parser stack.

## Where it fits

`tree-sitter-perl` is the validation and parser-compatibility crate. The
`pure-rust` path emits tree-sitter-compatible ASTs, while the comparison tools
let us measure behavior against the rest of the workspace.

## Key entry points

- `PureRustPerlParser`, `PerlParser`, `AstNode`
- `EnhancedPerlParser`, `FullPerlParser`, `EnhancedFullPerlParser`
- `ComparisonHarness`
- `language()`, `parse()`, `parse_with_tree()`

## Example

```rust
use tree_sitter_perl::parse;

let tree = parse("my $x = 1;")?;
assert!(tree.root_node().child_count() > 0);
```

## Commands

```bash
cargo build -p tree-sitter-perl
cargo test -p tree-sitter-perl
cargo run -p tree-sitter-perl --bin ts_test_parsers --features pure-rust
cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust
```
