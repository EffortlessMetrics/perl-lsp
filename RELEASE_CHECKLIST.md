# Release Checklist for v0.4.0

## Pre-Release Verification

### ✅ Code Quality
- [x] All tests pass: `cargo test --all`
- [x] Examples compile: `cargo build --examples`
- [x] No critical warnings (only unused variables)
- [x] Version numbers updated (0.4.0)

### ✅ Documentation
- [x] README.md updated with current status
- [x] CHANGELOG.md updated with v0.4.0 changes
- [x] RELEASE_ANNOUNCEMENT.md created
- [x] RELEASE_NOTES_v0.4.0.md created
- [x] API documentation builds without errors

### ✅ Features Completed
- [x] CLI tool (perl-parse) with:
  - [x] S-expression output
  - [x] JSON output format
  - [x] Statistics display
  - [x] Help and version flags
- [x] Integration examples:
  - [x] basic_usage.rs
  - [x] ast_visitor.rs
  - [x] error_handling.rs

### ✅ Binaries
- [x] Release binary built and stripped
- [x] Linux tarball created: perl-parse-v0.4.0-x86_64-unknown-linux-gnu.tar.gz (386K)

### ✅ Publishing Preparation
- [x] Cargo.toml files updated with:
  - [x] Version 0.4.0
  - [x] Proper dependencies
  - [x] Description and metadata
  - [x] License information
- [x] README files for both crates
- [x] PUBLISHING.md guide created

## Publishing Steps

### 1. Final Git Commit
```bash
git add -A
git commit -m "Release v0.4.0: 100% edge case coverage complete"
git tag v0.4.0
git push origin master --tags
```

### 2. Publish to crates.io
```bash
# First perl-lexer
cd crates/perl-lexer
cargo publish --dry-run
cargo publish

# Wait 5 minutes, then perl-parser
cd ../perl-parser
# Update Cargo.toml to remove path dependency
cargo publish --dry-run
cargo publish
```

### 3. GitHub Release
1. Go to https://github.com/EffortlessSteven/tree-sitter-perl/releases/new
2. Tag: v0.4.0
3. Title: "v0.4.0 - 100% Complete Perl 5 Parser 🎉"
4. Copy content from RELEASE_NOTES_v0.4.0.md
5. Upload perl-parse-v0.4.0-x86_64-unknown-linux-gnu.tar.gz

### 4. Announcements
- [ ] Reddit r/rust
- [ ] Reddit r/perl
- [ ] Twitter/X with #rustlang #perl
- [ ] Perl community forums

## Post-Release
- [ ] Update main README with crates.io badges
- [ ] Monitor for issues/feedback
- [ ] Plan next milestone from TODO.md