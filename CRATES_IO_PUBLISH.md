# Crates.io Publication Checklist

## Prerequisites
- [x] All packages have required metadata (name, version, authors, license, description)
- [x] Repository URL is set
- [x] Keywords and categories are defined
- [x] README path is specified
- [ ] Verify all dependencies are published on crates.io
- [ ] Run final test suite
- [ ] Update version numbers consistently

## Publishing Order (dependencies first)
1. **perl-lexer** (0.7.3)
   - No external dependencies beyond standard crates
   - Ready to publish

2. **perl-parser** (0.7.3)
   - Depends on perl-lexer
   - Includes LSP binary
   - Ready after perl-lexer is published

## Pre-publication Steps
```bash
# 1. Run all tests
cargo test --all

# 2. Build in release mode
cargo build --release --all

# 3. Run clippy
cargo clippy --all

# 4. Check documentation
cargo doc --no-deps --all

# 5. Verify package contents
cargo package --list -p perl-lexer
cargo package --list -p perl-parser

# 6. Dry run
cargo publish --dry-run -p perl-lexer
cargo publish --dry-run -p perl-parser
```

## Publication Commands
```bash
# 1. Publish perl-lexer first
cargo publish -p perl-lexer

# 2. Wait for crates.io to index (few minutes)

# 3. Update perl-parser to use crates.io version
# Edit crates/perl-parser/Cargo.toml:
# perl-lexer = "0.7.3"  # Remove path dependency

# 4. Publish perl-parser
cargo publish -p perl-parser
```

## Post-publication
- [ ] Verify packages on crates.io
- [ ] Test installation: `cargo install perl-parser --bin perl-lsp`
- [ ] Update README with installation instructions
- [ ] Create GitHub release
- [ ] Announce on relevant forums/communities