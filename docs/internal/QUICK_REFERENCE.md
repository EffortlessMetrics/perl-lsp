# tree-sitter-perl Quick Reference

## 🚀 Quick Start

```bash
# Install LSP server
cargo install --git https://github.com/EffortlessMetrics/tree-sitter-perl --bin perl-lsp

# Use parser library
cargo add perl-parser
```

## 📦 Project Structure

```
tree-sitter-perl/
├── crates/
│   ├── perl-lexer/        # v3 tokenizer (native)
│   ├── perl-parser/       # v3 parser + LSP server
│   ├── tree-sitter-perl-rs/  # v2 parser (Pest)
│   └── tree-sitter-perl-c/   # v1 parser (C bindings)
├── docs/                  # Documentation
├── benches/              # Performance benchmarks
└── xtask/                # Build automation
```

## 🛠️ Common Commands

### Build
```bash
# Build everything
cargo build --all

# Build LSP server
cargo build -p perl-parser --bin perl-lsp --release

# Build v2 parser
cargo build --features pure-rust

# Build v3 parser
cargo build -p perl-lexer -p perl-parser
```

### Test
```bash
# Run all tests
cargo test --all

# Test v3 parser
cargo test -p perl-parser

# Run edge case tests
cargo run -p perl-parser --example test_edge_cases
```

### Benchmark
```bash
# Run benchmarks
cargo bench

# Compare all parsers
cargo xtask compare
```

### LSP Server
```bash
# Run LSP server
perl-lsp --stdio

# With logging
perl-lsp --stdio --log

# Test capabilities
cargo run -p perl-parser --example lsp_capabilities
```

## 📊 Parser Comparison

| Feature | v1 (C) | v2 (Pest) | v3 (Native) |
|---------|--------|-----------|-------------|
| **Coverage** | ~95% | ~99.995% | ~100% |
| **Speed** | ~12-68µs | ~200-450µs | **~1-150µs** |
| **Edge Cases** | Limited | 95% | **100%** |
| **Dependencies** | C toolchain | None | None |
| **Recommended** | ❌ | For grammar work | ✅ **Production** |

## 🖥️ LSP Features

- ✅ Syntax diagnostics
- ✅ Document symbols
- ✅ Go to definition
- ✅ Find references
- ✅ Signature help
- ✅ Semantic tokens
- ✅ Incremental parsing
- 🚧 Code completion (planned)

## 📖 Key Documentation

- **[README.md](README.md)** - Full project overview
- **[docs/LSP_DOCUMENTATION.md](docs/LSP_DOCUMENTATION.md)** - LSP guide
- **[CLAUDE.md](CLAUDE.md)** - AI assistant guide
- **[ROADMAP.md](ROADMAP.md)** - Project status and plans

## 🔗 Links

- [Repository](https://github.com/EffortlessMetrics/tree-sitter-perl)
- [Issues](https://github.com/EffortlessMetrics/tree-sitter-perl/issues)
- [Discussions](https://github.com/EffortlessMetrics/tree-sitter-perl/discussions)

## ⚡ Performance Tips

1. **Use v3 parser** for production (4-19x faster than v1)
2. **Enable release mode** for best performance
3. **Cache parsed ASTs** when using LSP
4. **Use incremental updates** for large files

## 🐛 Troubleshooting

### LSP not working?
```bash
# Check if installed
which perl-lsp

# Test manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio
```

### Parser errors?
```bash
# Enable debug output
RUST_LOG=debug cargo run

# Check specific file
cargo run -p perl-parser --example debug -- file.pl
```

## 📈 Version History

- **v0.5.0** (upcoming) - LSP server implementation
- **v0.4.0** - v3 parser complete, 100% edge cases
- **v0.3.0** - v2 Pest parser, 99.995% coverage
- **v0.2.0** - Initial pure Rust attempt
- **v0.1.0** - Original C-based parser