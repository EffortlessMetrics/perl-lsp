# tree-sitter-perl (workspace harness)

Internal Tree-sitter validation/benchmark harness used in this workspace.

## Status

- `publish = false`
- Not intended as the primary public parser crate
- Maintained for compatibility checks, parser comparisons, and tooling experiments

For production parser APIs, use:

- [`perl-parser`](https://crates.io/crates/perl-parser) (current parser stack)
- [`perl-parser-pest`](https://crates.io/crates/perl-parser-pest) (legacy v2 parser)

## Scope

- Runs parser comparison binaries and benchmarks
- Provides development fixtures and harness code around Tree-sitter integration
- Supports feature-gated compatibility paths (including v2 microcrate bridge)

## Typical Commands

```bash
# Run internal parser comparison tooling
cargo run -p tree-sitter-perl --bin ts_test_parsers --features pure-rust

# Run internal benchmarks
cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust
```

## License

Apache-2.0 OR MIT.
