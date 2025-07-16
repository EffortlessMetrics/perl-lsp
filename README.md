# tree-sitter-perl

Tree-sitter Perl grammar with Rust-native scanner implementation.

## Project Structure

This repository contains both the legacy C implementation and a new Rust implementation:

- `/crates/tree-sitter-perl` - **Rust implementation** (primary development focus)
- `/c` - Legacy C implementation and bindings
- `/xtask` - Build automation and development tools

## Rust Implementation

The Rust implementation provides:
- High-performance Rust-native scanner
- Comprehensive test suite with corpus validation
- Property-based testing and benchmarks
- Modern error handling and Unicode support

### Getting Started with Rust Implementation

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and test
cargo xtask test
cargo xtask corpus
cargo xtask bench
```

### Development Commands

```bash
# Run all tests
cargo xtask test

# Run corpus tests with diagnostics
cargo xtask corpus --diagnose

# Run benchmarks
cargo xtask bench

# Check code quality
cargo xtask check --all

# Format code
cargo xtask fmt
```

## Legacy C Implementation

The legacy C implementation is preserved in the `/c` directory for compatibility with existing bindings and tools.

### Getting Started with Legacy C Implementation

```bash
cd c

# Install dependencies
npm run dev-install

# Generate bindings
npx tree-sitter generate

# Run tests
npx tree-sitter test
```

## Contributing

We welcome contributions to both implementations! The Rust implementation is the primary focus for new development.

### Rust Development

- All Rust code is in `/crates/tree-sitter-perl`
- Use `cargo xtask` for all development tasks
- Follow Rust best practices and error handling patterns
- Add comprehensive tests for new features

### Legacy C Development

- Legacy C code is in `/c` directory
- Follow existing C patterns and conventions
- Ensure compatibility with existing bindings

## Using the Bindings

### Neovim

```lua
local parser_config = require "nvim-treesitter.parsers".get_parser_configs()
parser_config.perl = {
  install_info = {
    url = 'https://github.com/tree-sitter-perl/tree-sitter-perl',
    revision = 'release',
    files = { "c/parser.c", "c/scanner.c" },
  }
}
```

### Emacs

```elisp
(setq treesit-language-source-alist
  '((perl . ("https://github.com/tree-sitter-perl/tree-sitter-perl" "release"))))
(treesit-install-language-grammar 'perl)
```

## License

MIT License - see LICENSE file for details.
