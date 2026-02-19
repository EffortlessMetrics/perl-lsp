# 🚀 Perl LSP v{{VERSION}} - {{RELEASE_TITLE}}

## Release Date
{{RELEASE_DATE}}

## 📋 Overview

{{RELEASE_OVERVIEW}}

## ✨ Key Highlights

{{KEY_HIGHLIGHTS}}

## 🎯 Major Features

### {{FEATURE_1_TITLE}}
{{FEATURE_1_DESCRIPTION}}

### {{FEATURE_2_TITLE}}
{{FEATURE_2_DESCRIPTION}}

### {{FEATURE_3_TITLE}}
{{FEATURE_3_DESCRIPTION}}

## 🛠️ Installation

### Quick Install
```bash
# Install LSP server
cargo install perl-lsp

# Install DAP server (optional)
cargo install perl-dap

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.sh | bash
```

### Package Managers

#### Homebrew (macOS/Linux)
```bash
brew install perl-lsp
```

#### Chocolatey (Windows)
```powershell
choco install perl-lsp
```

#### Scoop (Windows)
```powershell
scoop install perl-lsp
```

### Editor Setup

#### VS Code
```json
{
    "perl-lsp.serverPath": "/path/to/perl-lsp",
    "perl-lsp.args": ["--stdio"]
}
```

#### Neovim
```lua
require('lspconfig').perl_lsp.setup{
    cmd = { "perl-lsp", "--stdio" }
}
```

#### Emacs
```elisp
(add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))
(lsp-register-client
    (make-lsp-client :new-connection (lsp-stdio-connection "perl-lsp")
                    :activation-fn (lsp-activate-on "perl")
                    :server-id 'perl-lsp))
```

## 📊 Platform Support

| Platform | Architecture | Status | Download |
|----------|-------------|--------|----------|
| Linux (GNU) | x86_64 | ✅ Tier 1 | [perl-lsp-x86_64-unknown-linux-gnu.tar.gz]({{DOWNLOAD_URL_LINUX_X64}}) |
| Linux (musl) | x86_64 | ✅ Tier 1 | [perl-lsp-x86_64-unknown-linux-musl.tar.gz]({{DOWNLOAD_URL_LINUX_MUSL}}) |
| Linux (GNU) | aarch64 | ✅ Tier 1 | [perl-lsp-aarch64-unknown-linux-gnu.tar.gz]({{DOWNLOAD_URL_LINUX_ARM64}}) |
| macOS | x86_64 | ✅ Tier 1 | [perl-lsp-x86_64-apple-darwin.tar.gz]({{DOWNLOAD_URL_MACOS_X64}}) |
| macOS | aarch64 | ✅ Tier 1 | [perl-lsp-aarch64-apple-darwin.tar.gz]({{DOWNLOAD_URL_MACOS_ARM64}}) |
| Windows | x86_64 | ✅ Tier 1 | [perl-lsp-x86_64-pc-windows-msvc.zip]({{DOWNLOAD_URL_WINDOWS}}) |

## 🔄 Migration Guide

{{MIGRATION_GUIDE}}

## 📈 Performance

| Operation | Previous | Current | Improvement |
|-----------|----------|---------|-------------|
| {{PERF_METRIC_1}} | {{PERF_OLD_1}} | {{PERF_NEW_1}} | {{PERF_IMPROVEMENT_1}} |
| {{PERF_METRIC_2}} | {{PERF_OLD_2}} | {{PERF_NEW_2}} | {{PERF_IMPROVEMENT_2}} |
| {{PERF_METRIC_3}} | {{PERF_OLD_3}} | {{PERF_NEW_3}} | {{PERF_IMPROVEMENT_3}} |

## 🐛 Bug Fixes

- {{BUG_FIX_1}}
- {{BUG_FIX_2}}
- {{BUG_FIX_3}}

## 🔧 Breaking Changes

{{BREAKING_CHANGES}}

## 🛡️ Security

{{SECURITY_FIXES}}

## 📚 Documentation

- [Getting Started Guide]({{DOCS_BASE_URL}}/GETTING_STARTED.md)
- [API Documentation]({{DOCS_BASE_URL}}/INDEX.md)
- [Migration Guide]({{DOCS_BASE_URL}}/MIGRATION.md)
- [Troubleshooting]({{DOCS_BASE_URL}}/TROUBLESHOOTING.md)

## 🤝 Contributors

{{CONTRIBUTORS}}

Thank you to everyone who contributed to this release!

## 🔗 Links

- [Homepage]({{HOMEPAGE_URL}})
- [Documentation]({{DOCS_BASE_URL}})
- [GitHub Repository]({{REPO_URL}})
- [Crates.io]({{CRATES_IO_URL}})
- [VS Code Marketplace]({{VSCODE_MARKETPLACE_URL}})

## 📋 Verification

```bash
# Verify installation
perl-lsp --version

# Check LSP capabilities
perl-lsp --capabilities

# Run health check
nix develop -c just health
```

## 🎯 Support

- **GitHub Issues**: [Report bugs and request features]({{REPO_URL}}/issues)
- **Discussions**: [Community discussions and Q&A]({{REPO_URL}}/discussions)
- **Security**: Report security issues to security@perl-lsp.org

---

## 📜 License

Dual licensed under:
- [MIT License]({{REPO_URL}}/blob/main/LICENSE-MIT)
- [Apache License 2.0]({{REPO_URL}}/blob/main/LICENSE-APACHE)

---

**Download Perl LSP v{{VERSION}} today and experience the future of Perl development!**

🚀 [Get Started Now]({{DOCS_BASE_URL}}/GETTING_STARTED.md) | 📖 [Documentation]({{DOCS_BASE_URL}}) | 💬 [Community]({{REPO_URL}}/discussions)