# Perl LSP v0.10.0 — Public Alpha

## Release Date

February 28, 2026

## Overview

Perl LSP v0.10.0 is the largest release in the project's history, spanning **85 merged PRs** across the full 80+ crate workspace. This release delivers security hardening, build reliability, crates.io publishing readiness, comprehensive documentation, performance optimizations, and code quality improvements. All crates are version-consistent, publish-ready, and backed by 1521 passing tests with a clean security audit.

## 🚀 Key Highlights

- **Fast & Native**: Recursive descent parser written in pure Rust (1–150μs typical).
- **Substantially Complete**: 100% LSP 3.18 coverage (53/53 user-visible, 97/97 protocol).
- **High Performance**: Sub-millisecond incremental updates with 70–99% node reuse.
- **1521 Tests Passing**: Comprehensive test suite with adaptive threading.
- **Security Audited**: Zero known vulnerabilities (`cargo audit` clean) plus 7 proactive security fixes.
- **crates.io Ready**: All 80+ crates verified for public publishing.
- **Semver Compatible**: No breaking API changes from v0.9.1.

---

## 🎯 What's New in v0.10.0 — Complete PR Listing

### 🔧 Build & Compilation Fixes

| PR | Description |
|----|-------------|
| #858 | Harden checksum verification and stabilize incremental parsing CI |
| #882 | Document highlight for modern Perl syntax (try/catch, signatures, methods) |
| #892 | Address unresolved review comments from PRs #881 and #882 |
| #894 | Resolve cargo doc warnings across workspace |
| #896 | Resolve document highlight test regressions |
| #901 | Resolve clippy warnings across all targets |
| #905 | Improve error logging in LSP providers |
| #907 | Normalize workspace inheritance in publish-allowlist Cargo.toml files |
| #920 | Repair tree-sitter-perl C binding crate so it builds standalone |
| #922 | Use valid crates.io category slug for perl-corpus |

### 📝 Documentation

| PR | Description |
|----|-------------|
| #886 | Create copilot instructions for AI-assisted development |
| #887 | Fix missing documentation warnings for FrameworkKind and FrameworkFlags |
| #888 | Update README.md and ROADMAP.md dates and status |
| #893 | Finalize RELEASE_NOTES.md and CHANGELOG.md for v0.10.0 |
| #903 | Add public API documentation to leaf crates |
| #904 | Add public API documentation to perl-parser |
| #909 | Polish CONTRIBUTING.md for public release |
| #913 | Add quality badges to README |
| #914 | Add usage examples to crate documentation |
| #916 | Expand crate-level documentation for mid-tier crates |
| #917 | Comprehensive v0.10.0 changelog and release notes |
| #918 | Add missing package manager install methods and VS Code Marketplace link |
| #919 | Add doc comments to pub mod declarations in perl-dap |
| #921 | Update security policy for v0.10.0 release |

### 🔒 Security

| PR | Description |
|----|-------------|
| #633 | 🛡️ **[HIGH]** Fix Argument Injection in TestRunner |
| #640 | 🛡️ **[HIGH]** Fix path traversal in debug adapter launch |
| #647 | 🛡️ **[MEDIUM]** Fix safe evaluation bypass for iterator/IO ops |
| #861 | Remediate minimatch CVEs in vscode-extension lockfile |
| #910 | Harden installer scripts for security and reliability |
| #911 | Harden GitHub Actions workflow configurations (SHA-pinned actions) |
| #923 | Harden workflow permissions, concurrency, and secret scoping |

### 🧪 Testing

| PR | Description |
|----|-------------|
| #729 | Add inlineValues lifecycle coverage tests |
| #846 | Replace sleep-poll with condvar+drain-bytes in LSP harness |
| #864 | Isolate cleanup_no_backups backup root in refactoring tests |
| #899 | Add glob-expressions corpus test fixtures |
| #900 | Add tie-interface corpus test fixtures |
| #908 | Enable ignored benchmark test and fix placeholder assertions |

### 🚀 Release Infrastructure

| PR | Description |
|----|-------------|
| #845 | Align receipt parsing and serialize BDD tests |
| #847 | Add `--locked` to BDD gate and timing receipts |
| #859 | Skip docs deploy when GitHub Pages is disabled |
| #860 | Stabilize nightly CI: fuzz harness hardening, coverage resilience, clippy cleanup |
| #865 | Unblock crates.io dry-run packaging for workspace crates |
| #867 | Fix crates publish workflow for workspace dev-dependency verification cycles |
| #868 | xtask gates: fail closed for required timeout/error statuses |
| #870 | Correct publish-crates dry-run quoting error |
| #871 | crates.io public release readiness and orchestration hardening |
| #872 | Prepare PR-driven 0.x.y release flow |
| #873 | Install git-cliff via cargo for deterministic workflow |
| #874 | Install git-cliff from latest Linux asset |
| #875 | Correct git-cliff extraction path |
| #876 | Install cargo-release from released binary assets |
| #877 | Use temp dirs for release-tool binary installs |
| #878 | Release v0.10.0 |
| #879 | Bump workspace versions to 0.10.0 |
| #880 | Publish merge-gate commit status checks |
| #881 | v0.10.0 release candidate — build fixes, version bump, code quality |
| #890 | Correct release workflow asset names and concurrency groups |
| #902 | Align asset naming across release workflow chain |
| #915 | Add perl-dap to distribution templates and fix build-packages crate name |

### 🎨 Code Quality

| PR | Description |
|----|-------------|
| #632 | ⚡ Avoid deep AST cloning in ScopeAnalyzer |
| #638 | ⚡ Optimize scope analysis parameter check and fix double reporting |
| #645 | ⚡ Optimize symbol extraction regex compilation |
| #646 | Context-aware Perl LSP status menu |
| #848 | Extract feature governance into 9 microcrates |
| #857 | Module infrastructure crates, Content-Length framing, and LSP hardening |
| #862 | Preserve rebased local history from pre-merge line |
| #883 | Update test metrics and housekeeping |
| #884 | Fix remaining v0.9.x version references |
| #885 | v0.10.0 release polish — lockfile sync and changelog |
| #889 | Remove stale tracked files and harden .gitignore |
| #895 | Remove unused dependencies flagged by cargo-machete |
| #898 | Update debt ledger after cleanup campaign |

### 📦 Package Management

| PR | Description |
|----|-------------|
| #849 | Bump `@types/node` 25.1.0 → 25.3.0 |
| #850 | Bump `@types/tar` 6.1.13 → 7.0.87 |
| #851 | Bump `aquasecurity/trivy-action` 0.34.0 → 0.34.1 |
| #852 | Bump the dependencies group with 2 updates |
| #853 | Bump `toml` 0.9.12 → 1.0.3 |
| #854 | Bump `serial_test` 3.3.1 → 3.4.0 |
| #855 | Bump `rand` 0.9.2 → 0.10.0 |
| #856 | Bump `uuid` 1.20.0 → 1.21.0 |
| #863 | VS Code extension: fix packaging/installability blockers |
| #866 | Include runtime node deps in VSIX packaging |
| #869 | Add vscode-extension package-lock for npm CI smoke |
| #897 | Expand crates.io publish allowlist with verified crates |
| #906 | Polish VS Code extension for marketplace readiness |

---

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
| **Total PRs Merged** | **85** |
| PR Range | #632 – #923 |
| Tests Passing | 1521 |
| Crates Updated | 80+ |
| 🔧 Build & Compilation Fixes | 10 |
| 📝 Documentation | 14 |
| 🔒 Security | 7 |
| 🧪 Testing | 6 |
| 🚀 Release Infrastructure | 22 |
| 🎨 Code Quality | 13 |
| 📦 Package Management | 13 |

## 🎯 Support & Community

- **GitHub Issues**: [Report bugs and request features](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [Community discussions and Q&A](https://github.com/EffortlessMetrics/perl-lsp/discussions)
- **Full Changelog**: [CHANGELOG.md](CHANGELOG.md)

## 📜 License

Dual licensed under [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).

---

**Try Perl LSP v0.10.0 today and help shape the future of Perl development!**

🚀 [Get Started Now](../tutorials/GETTING_STARTED.md) | 📖 [Documentation](docs/INDEX.md) | 💬 [Community](https://github.com/EffortlessMetrics/perl-lsp/discussions)
