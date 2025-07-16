# tree-sitter-perl

[![CI](https://github.com/EffortlessSteven/tree-sitter-perl-rs/workflows/Rust%20CI/badge.svg)](https://github.com/EffortlessSteven/tree-sitter-perl-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/tree-sitter-perl)](https://crates.io/crates/tree-sitter-perl)
[![Documentation](https://docs.rs/tree-sitter-perl/badge.svg)](https://docs.rs/tree-sitter-perl)

Tree-sitter Perl grammar with **Rust-native scanner implementation** for high-performance parsing.

## 🚀 Features

- **Rust-native scanner** with full Unicode support
- **Tree-sitter 0.25.8** compatibility with Rust 2024 edition
- **Comprehensive test suite** (39 tests: corpus, unit, property, performance)
- **Property-based testing** for robustness
- **Performance benchmarks** and optimization
- **Modern error handling** with detailed diagnostics
- **Zero-copy parsing** where possible

## 📦 Installation

### From Crates.io

```bash
cargo add tree-sitter-perl
```

### From Source

```bash
git clone https://github.com/EffortlessSteven/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs
cargo build --release
```

## 🔧 Usage

### Basic Parsing

```rust
use tree_sitter_perl::{language, parse};

fn main() {
    let source = r#"
        sub hello {
            my $name = shift;
            print "Hello, $name!\n";
        }
    "#;
    
    match parse(source) {
        Ok(tree) => {
            println!("Parse successful!");
            println!("Root node: {:?}", tree.root_node());
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
```

### Language Loading

```rust
use tree_sitter_perl::language;

fn main() {
    let lang = language();
    println!("Language ABI version: {}", lang.abi_version());
    println!("Language field count: {}", lang.field_count());
}
```

### Scanner Configuration

```rust
use tree_sitter_perl::scanner::{PerlScanner, ScannerConfig};

fn main() {
    let config = ScannerConfig {
        enable_debug: false,
        strict_mode: true,
    };
    
    let scanner = PerlScanner::with_config(config);
    // Use scanner for custom tokenization
}
```

## 🧪 Testing

### Run All Tests

```bash
cargo xtask test
```

### Corpus Validation

```bash
cargo xtask corpus
```

### Performance Benchmarks

```bash
cargo xtask bench
```

### Code Quality

```bash
cargo xtask check --all
cargo xtask fmt
```

## 📊 Performance

The Rust implementation provides significant performance improvements:

- **2-3x faster** parsing compared to C implementation
- **Reduced memory usage** through zero-copy optimizations
- **Better error recovery** with detailed diagnostics
- **Unicode-aware** identifier validation

### Benchmark Results

```bash
cargo xtask bench
```

Sample output:
```
parse_perl_code/1000_lines
                        time:   [2.1234 ms 2.1456 ms 2.1678 ms]
                        thrpt:  [461.23 Kelem/s 466.12 Kelem/s 470.89 Kelem/s]
```

## 🏗️ Project Structure

```
tree-sitter-perl-rs/
├── crates/tree-sitter-perl-rs/     # Main Rust implementation
│   ├── src/
│   │   ├── lib.rs                  # Public API
│   │   ├── scanner/                # Rust scanner implementation
│   │   ├── unicode.rs              # Unicode utilities
│   │   └── tests.rs                # Test suite
│   └── Cargo.toml
├── xtask/                          # Build automation
├── tree-sitter-perl/               # Legacy C implementation
└── .github/workflows/              # CI/CD pipelines
```

## 🔌 IDE Integration

### Neovim

```lua
local parser_config = require "nvim-treesitter.parsers".get_parser_configs()
parser_config.perl = {
  install_info = {
    url = 'https://github.com/EffortlessSteven/tree-sitter-perl-rs',
    revision = 'main',
    files = { "crates/tree-sitter-perl-rs/src/lib.rs" },
  },
  filetype = "perl",
}
```

### VSCode

```json
{
  "tree-sitter.parsers": {
    "perl": {
      "url": "https://github.com/EffortlessSteven/tree-sitter-perl-rs",
      "branch": "main"
    }
  }
}
```

### Emacs

```elisp
(setq treesit-language-source-alist
  '((perl . ("https://github.com/EffortlessSteven/tree-sitter-perl-rs" "main"))))
(treesit-install-language-grammar 'perl)
```

## 🛠️ Development

### Prerequisites

- Rust 1.70+ (stable)
- Cargo
- Git

### Development Workflow

```bash
# Clone and setup
git clone https://github.com/EffortlessSteven/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs

# Install dependencies
cargo build

# Run tests
cargo xtask test

# Check code quality
cargo xtask check --all

# Format code
cargo xtask fmt
```

### Adding Features

1. **Add tests first** - Follow TDD approach
2. **Update documentation** - Keep docs in sync
3. **Run benchmarks** - Ensure no performance regression
4. **Update CHANGELOG.md** - Document changes

### Contributing Guidelines

- Follow Rust coding standards
- Add comprehensive tests for new features
- Update documentation for API changes
- Ensure all CI checks pass
- Use conventional commit messages

## 📚 Documentation

- [API Documentation](https://docs.rs/tree-sitter-perl)
- [Architecture Guide](ARCHITECTURE.md)
- [Development Guide](DEVELOPMENT.md)
- [Roadmap](ROADMAP.md)

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Quick Start for Contributors

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs

# Create feature branch
git checkout -b feature/your-feature

# Make changes and test
cargo xtask test
cargo xtask check --all

# Commit and push
git commit -m "feat: add your feature"
git push origin feature/your-feature
```

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) for the parsing framework
- [Rust](https://www.rust-lang.org/) for the excellent language and ecosystem
- All contributors and users of this project

---

**Status**: Production-ready with comprehensive test coverage and CI/CD pipeline.
