# Publishing Guide

This guide explains how to publish the perl-lexer and perl-parser crates to crates.io.

## Prerequisites

1. Create accounts on [crates.io](https://crates.io)
2. Run `cargo login` and enter your API token
3. Ensure all tests pass: `cargo test --all`
4. Update version numbers in Cargo.toml files

## Publishing Order

Due to dependencies, crates must be published in this order:

### 1. Publish perl-lexer

```bash
cd crates/perl-lexer
cargo publish --dry-run  # Verify everything looks good
cargo publish
```

Wait a few minutes for crates.io to index the package.

### 2. Update perl-parser dependency

Edit `crates/perl-parser/Cargo.toml`:
```toml
[dependencies]
perl-lexer = "0.4.0"  # Remove the 'path' specification
```

### 3. Publish perl-parser

```bash
cd crates/perl-parser
cargo publish --dry-run  # Verify everything looks good
cargo publish
```

## Post-Publishing

1. Create a GitHub release with tag `v0.4.0`
2. Upload pre-built binaries for the CLI tool
3. Update the main README with installation instructions
4. Announce on:
   - Rust subreddit
   - Perl community forums
   - Twitter/X with #rustlang #perl hashtags

## Version Checklist

Before publishing, ensure:

- [ ] All version numbers are updated (0.4.0)
- [ ] CHANGELOG.md is up to date
- [ ] README files have correct version in examples
- [ ] All tests pass
- [ ] Documentation builds without warnings
- [ ] Examples compile and run correctly

## Binary Releases

Build release binaries for perl-parse:

```bash
# Linux
cargo build --release -p perl-parser --features cli --bin perl-parse
strip target/release/perl-parse
tar czf perl-parse-v0.4.0-x86_64-unknown-linux-gnu.tar.gz -C target/release perl-parse

# macOS (if available)
cargo build --release -p perl-parser --features cli --bin perl-parse
strip target/release/perl-parse
tar czf perl-parse-v0.4.0-x86_64-apple-darwin.tar.gz -C target/release perl-parse

# Windows (if available)
cargo build --release -p perl-parser --features cli --bin perl-parse
# Create perl-parse-v0.4.0-x86_64-pc-windows-msvc.zip
```

Upload these to the GitHub release.