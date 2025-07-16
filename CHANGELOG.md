# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Rust Conversion

### 🚀 Major Changes
- **Rust Conversion**: Converting from C/JS to pure Rust implementation
- **Build System**: Replacing C build with pure Rust `cargo build`
- **Dependencies**: Eliminating C toolchain dependencies

### 🔧 Development
- [ ] Port `src/scanner.c` to `src/scanner.rs`
- [ ] Port `src/tsp_unicode.h` to Rust Unicode helpers
- [ ] Remove C build logic from `bindings/rust/build.rs`
- [ ] Update `Cargo.toml` for pure Rust build
- [ ] Add comprehensive Rust unit tests
- [ ] Update documentation for Rust usage

### 📋 Planned
- [ ] Scanner logic porting (Phase 2.1)
- [ ] Unicode helpers porting (Phase 2.2)
- [ ] Build system migration (Phase 2.3)
- [ ] Rust bindings update (Phase 3.1)
- [ ] Grammar integration testing (Phase 3.2)
- [ ] Corpus test validation (Phase 4.1)
- [ ] Rust unit test suite (Phase 4.2)
- [ ] Integration testing (Phase 4.3)
- [ ] Documentation updates (Phase 5.1)
- [ ] Code quality improvements (Phase 5.2)
- [ ] CI/CD setup (Phase 6.1)
- [ ] Release preparation (Phase 6.2)

---

## [1.0.0] - 2024-01-XX

### 🎉 Initial Release
- Initial tree-sitter Perl parser implementation
- C-based scanner with JavaScript grammar
- Support for Neovim and Emacs tree-sitter integration
- Comprehensive corpus test suite

### ✨ Features
- Full Perl syntax parsing support
- Heredoc and quote handling
- Unicode identifier support
- Complex interpolation logic
- POD documentation parsing

### 🔧 Technical
- C scanner implementation (`src/scanner.c`)
- JavaScript grammar definition (`grammar.js`)
- Multi-language bindings (Node.js, Rust, Python, Go, Swift)
- Tree-sitter corpus test coverage

---

## Migration Notes

### For Downstream Consumers
- **Neovim**: No changes required - parser will continue to work
- **Emacs**: No changes required - parser will continue to work
- **Rust Consumers**: API will remain compatible during transition
- **Build Systems**: Will switch from C build to pure Rust

### Breaking Changes
- None planned - maintaining full API compatibility
- Build process will change from C compilation to `cargo build`
- Internal implementation will be Rust-native

---

*For detailed progress tracking, see [ROADMAP.md](./ROADMAP.md)* 