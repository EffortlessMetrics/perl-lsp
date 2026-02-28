# Perl LSP v0.10.0 — Public Alpha

## Release Date

February 28, 2026

## Overview

Perl LSP v0.10.0 is the largest release in the project's history, spanning **60+ merged PRs** (#845–#911). This release focuses on build reliability, security hardening, crates.io publishing readiness, comprehensive documentation, and code quality across the entire 80+ crate workspace. All crates are version-consistent, publish-ready, and backed by 1521 passing tests with a clean security audit.

## 🚀 Key Highlights

- **Fast & Native**: Recursive descent parser written in pure Rust (1–150μs typical).
- **Substantially Complete**: 100% LSP 3.18 coverage (53/53 user-visible, 97/97 protocol).
- **High Performance**: Sub-millisecond incremental updates with 70–99% node reuse.
- **1521 Tests Passing**: Comprehensive test suite with adaptive threading.
- **Security Audited**: Zero known vulnerabilities (`cargo audit` clean) plus 3 proactive security fixes.
- **crates.io Ready**: All 80+ crates verified for public publishing.
- **Semver Compatible**: No breaking API changes from v0.9.1.

## 🎯 What's New in v0.10.0

### Document Highlight for Modern Perl
Enhanced highlighting for modern Perl syntax constructs (#882, #896):
- **try/catch parameters**: Proper highlighting of catch block variables.
- **Method and subroutine signatures**: Full support for signature parameter highlighting.
- **String interpolation**: Correct highlighting of interpolated variables within strings.

### Security Hardening (4 fixes)
- **[HIGH]** Fixed path traversal vulnerability in debug adapter launch (#640).
- **[HIGH]** Fixed argument injection in TestRunner (#633).
- **[MEDIUM]** Fixed safe evaluation bypass for iterator/IO operations (#647).
- **GitHub Actions**: SHA-pinned all workflow action references (#911).
- **Installer scripts**: Hardened for security and reliability (#910).
- **VS Code extension**: Pinned `minimatch` to 10.2.3 to remediate CVEs (#861).

### Performance Optimizations (3 improvements)
- **Symbol extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope analyzer**: Optimized unused parameter detection and fixed double reporting (#638).

### Build & Compilation Fixes
- Resolved 4 compilation errors in the release candidate build (#881).
- Resolved clippy warnings across all targets (#901).
- Resolved cargo doc warnings across workspace (#894).
- All workspace crates compile cleanly on stable Rust.

### Code Quality Campaign
- Unused dependencies removed via cargo-machete sweep (#895).
- Debt ledger updated after cleanup campaign (#898).
- Stale tracked files removed, `.gitignore` hardened (#889).
- Unresolved PR review comments addressed (#892).
- LSP error logging improved in providers (#905).

### Documentation Overhaul
- Public API documentation for `perl-parser` (#904) and leaf crates (#903).
- `CONTRIBUTING.md` polished for public release (#909).
- `README.md` and `ROADMAP.md` dates and status updated (#888).
- `FrameworkKind` and `FrameworkFlags` documentation warnings fixed (#887).
- Cargo doc warnings resolved across workspace (#894).
- Copilot instructions added for AI-assisted development (#886).

### crates.io Publishing Readiness
- All crate metadata verified, publish-ignore lists normalized (#871).
- Publish allowlist expanded with verified leaf crates (#897).
- Dry-run packaging unblocked for all workspace crates (#865).
- Dev-dependency cycle workaround with `--no-verify` (#867).
- Crate badges added to all published crates (#871).

### VS Code Extension Polish
- Marketplace readiness with metadata and packaging fixes (#906).
- Runtime node dependencies included in VSIX packaging (#866).
- npm lockfile added for CI smoke tests (#869).
- Release packaging compatibility restored (#863).

### Test Suite Improvements
- Tie-interface corpus test fixtures added (#900).
- Previously-ignored benchmark test enabled with real assertions (#908).
- InlineValues lifecycle coverage tests (#729).
- Refactoring test isolation for `cleanup_no_backups` (#864).
- LSP harness: replaced sleep-poll with condvar+drain-bytes for determinism (#846).

### Version Consistency
- Updated 77+ files across all workspace crates, documentation, VS Code extension, and feature catalogs to v0.10.0 (#879).
- Resolved version drift between workspace root and satellite files still referencing v0.9.1 (#884).
- `features.toml` updated with 100% LSP coverage maintained.

### CI & Release Infrastructure
- **Release orchestration**: Turnkey PR-driven 0.x.y release workflow (#872).
- **Concurrency groups**: Prevent duplicate release workflow runs (#890).
- **Asset naming**: Aligned across entire release workflow chain (#890, #902).
- **Release tool installs**: Deterministic git-cliff (#873–#875) and cargo-release (#876, #877) installation.
- **Merge-gate status**: CI now publishes merge-gate commit status checks (#880).
- **Nightly CI stabilized**: Fuzz harness hardening, coverage resilience, clippy cleanup (#860).
- **Docs deploy**: Graceful skip when GitHub Pages is disabled (#859).
- **BDD gate**: Added `--locked` flag and timing receipts (#847).
- **Receipt parsing**: Aligned across CI pipelines (#845).
- **xtask gates**: Fail closed for required timeout/error statuses (#868).

### Architecture Improvements
- Feature governance extracted into 9 microcrates for modularity (#848).
- Module infrastructure crates with Content-Length framing (#857).
- Context-aware Perl LSP status menu (#646).
- Semver-aware benchmark sorting for correct version comparison (#885).

### Dependency Updates
- `rand` 0.9.2 → 0.10.0, `serial_test` 3.3.1 → 3.4.0, `uuid` 1.20.0 → 1.21.0.
- `toml` 0.9.12 → 1.0.3, `aquasecurity/trivy-action` 0.34.0 → 0.34.1.
- VS Code: `@types/node` 25.1.0 → 25.3.0, `@types/tar` 6.1.13 → 7.0.87.

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

## 📊 Release Statistics

| Metric | Value |
|--------|-------|
| PRs Merged | 60+ |
| PR Range | #845 – #911 |
| Tests Passing | 1521 |
| Crates Updated | 80+ |
| Files Changed | 77+ |
| Security Fixes | 4 |
| Performance Fixes | 3 |
| Dependency Updates | 8 |

## 🎯 Support & Community

- **GitHub Issues**: [Report bugs and request features](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [Community discussions and Q&A](https://github.com/EffortlessMetrics/perl-lsp/discussions)
- **Full Changelog**: [CHANGELOG.md](CHANGELOG.md)

## 📜 License

Dual licensed under [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).

---

**Try Perl LSP v0.10.0 today and help shape the future of Perl development!**

🚀 [Get Started Now](docs/GETTING_STARTED.md) | 📖 [Documentation](docs/INDEX.md) | 💬 [Community](https://github.com/EffortlessMetrics/perl-lsp/discussions)
