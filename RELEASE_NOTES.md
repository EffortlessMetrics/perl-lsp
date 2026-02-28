# Perl LSP v0.10.0 — Public Alpha

## Release Date

February 28, 2026

## Overview

Perl LSP v0.10.0 continues the **Public Alpha** series with build reliability fixes, security hardening, document highlight improvements for modern Perl syntax, and release infrastructure hardening. All 80+ workspace crates are now version-consistent and crates.io publish-ready, with 1521 tests passing and a clean security audit.

## 🚀 Key Highlights

- **Fast & Native**: Recursive descent parser written in pure Rust (1–150μs typical).
- **Substantially Complete**: 100% LSP 3.18 coverage (53/53 user-visible, 97/97 protocol).
- **High Performance**: Sub-millisecond incremental updates with 70–99% node reuse.
- **1521 Tests Passing**: Comprehensive test suite with adaptive threading.
- **Security Audited**: Zero known vulnerabilities (`cargo audit` clean).
- **Semver Compatible**: No breaking API changes from v0.9.1.

## 🎯 What's New in v0.10.0

### Document Highlight Improvements
Enhanced highlighting for modern Perl syntax constructs:
- **try/catch parameters**: Proper highlighting of catch block variables.
- **Method and subroutine signatures**: Full support for signature parameter highlighting.
- **String interpolation**: Correct highlighting of interpolated variables within strings.

### Security Hardening
Three security fixes applied during this release cycle:
- **[HIGH]** Fixed path traversal vulnerability in debug adapter launch (#640).
- **[HIGH]** Fixed argument injection in TestRunner (#633).
- **[MEDIUM]** Fixed safe evaluation bypass for iterator/IO operations (#647).
- Pinned `minimatch` to 10.2.3 in the VS Code extension lockfile (#861).

### Performance Optimizations
- **Symbol extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope analyzer**: Optimized unused parameter detection (#638).

### Build & Compilation Fixes
- Resolved 4 compilation errors in the release candidate build (#881).
- All workspace crates compile cleanly on stable Rust.

### Version Consistency
- Updated 77+ files across all workspace crates, documentation, VS Code extension, and feature catalogs to v0.10.0.
- Resolved version drift between workspace root and satellite files still referencing v0.9.1 (#884).
- `features.toml` updated with 100% LSP coverage maintained.

### CI & Release Infrastructure
- **Concurrency groups**: Prevent duplicate release workflow runs (#890).
- **Asset name fixes**: Corrected release workflow artifact URLs (#890).
- **crates.io readiness**: All crate metadata verified, publish-ignore lists normalized, crate badges added (#871).
- **VS Code extension**: Packaging fixes for runtime dependencies and npm lockfile (#863, #866, #869).
- **Scoop/Chocolatey/Homebrew**: Packaging configuration fixes for all package managers.

### Additional Improvements
- Semver-aware benchmark sorting for correct version comparison (#885).
- Context-aware status menu states for improved UX (#646).
- `inlineValues` lifecycle coverage (#729).
- Module infrastructure crates and Content-Length framing hardening (#857).
- Feature governance extracted into 9 microcrates (#848).

## ⚠️ Breaking Changes

None. v0.10.0 is a drop-in upgrade from v0.9.1.

## 🔄 Migration from v0.9.1

No migration steps required. Simply update your installation:

```bash
cargo install perl-lsp --force
```

Editor extensions will detect the new version automatically.

## 🐛 Known Issues

- DAP support remains Phase 1 (bridge to Perl::LanguageServer). Native DAP implementation is planned for a future release.
- Wire protocol and APIs are subject to change during the alpha phase (pre-v0.15.0).

## 🛠️ Installation

### From crates.io

```bash
cargo install perl-lsp
cargo install perl-dap   # optional: debug adapter
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
```

### Quick Install Script (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

## 📊 Platform Support

| Platform | Architecture | Status | Binary |
|----------|-------------|--------|--------|
| Linux (GNU) | x86_64 | ✅ Tier 1 | Pre-built |
| Linux (musl) | x86_64 | ✅ Tier 1 | Pre-built |
| Linux (GNU) | aarch64 | ✅ Tier 1 | Pre-built |
| macOS | x86_64 | ✅ Tier 1 | Pre-built |
| macOS | aarch64 | ✅ Tier 1 | Pre-built |
| Windows | x86_64 | ✅ Tier 1 | Pre-built |

## 📋 Roadmap

### Next Release
- Enhanced DAP native implementation (Phase 2).
- Moo/Moose semantic depth (field recognition).
- Performance optimizations and refactoring refinements.

### v0.15.0 — Stability Contract Milestone
- Formal API stability and contract-locked wire protocol.
- Full protocol compliance audit.
- Package manager distribution.

## 🎯 Support & Community

- **GitHub Issues**: [Report bugs and request features](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [Community discussions and Q&A](https://github.com/EffortlessMetrics/perl-lsp/discussions)
- **Full Changelog**: [CHANGELOG.md](CHANGELOG.md)

## 📜 License

Dual licensed under [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).

---

**Try Perl LSP v0.10.0 today and help shape the future of Perl development!**

🚀 [Get Started Now](docs/GETTING_STARTED.md) | 📖 [Documentation](docs/INDEX.md) | 💬 [Community](https://github.com/EffortlessMetrics/perl-lsp/discussions)
