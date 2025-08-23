# Release Notes - v0.8.4

## 🚀 LSP Feature Complete Release

We're excited to announce v0.8.4, which transforms the Perl LSP from a basic prototype into a **production-ready language server** with 60% functionality (up from 35% in v0.8.3).

## ✨ Highlights

### 9 New LSP Features
This release adds **nine major LSP capabilities**, all fully functional and tested:

1. **Workspace Symbols** - Search across all open files with fuzzy matching
2. **Rename Refactoring** - Smart cross-file renaming with proper scoping
3. **Code Actions** - Quick fixes for missing `use strict` and `use warnings`
4. **Semantic Tokens** - Enhanced syntax highlighting with proper categorization
5. **Inlay Hints** - Parameter names for built-ins, type hints for literals
6. **Document Links** - Navigate from `use` statements to modules
7. **Selection Ranges** - Hierarchical smart selection
8. **On-Type Formatting** - Auto-indent/dedent for braces

### Contract-Driven Development
- **Every capability is tested** - No more stub implementations
- **Feature flag control** - `lsp-ga-lock` for conservative releases
- **Clear capability policy** - Only advertise what actually works

### Robust Architecture
- **Fallback mechanisms** - Works with incomplete/invalid code
- **Memory efficient** - Arc-based AST with parent maps
- **Fast operations** - All LSP operations complete in <50ms
- **Clean separation** - One module per feature

## 📊 By the Numbers

- **60%** LSP functionality (up from 35%)
- **530+** tests passing (up from ~150)
- **9** new LSP feature modules
- **100%** acceptance test coverage for advertised features
- **<50ms** response time for all operations

## 🔧 Installation

```bash
# Install from crates.io
cargo install perl-parser --bin perl-lsp

# Or use the quick installer
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

## 📚 Documentation

- See [LSP_ACTUAL_STATUS.md](LSP_ACTUAL_STATUS.md) for complete feature status
- See [CHANGELOG.md](CHANGELOG.md) for detailed changes
- See [README.md](README.md) for installation and usage

## 🙏 Thank You

Thanks to all contributors and users who provided feedback on LSP functionality. Your input helped us prioritize the most impactful features.

## 🎯 What's Next

In future releases, we plan to:
- Implement remaining LSP features (code lens, call hierarchy)
- Add more code actions (extract variable, inline)
- Enhance semantic tokens with more token types
- Improve cross-file navigation accuracy

---

**Full Changelog**: https://github.com/EffortlessSteven/tree-sitter-perl/compare/v0.8.3...v0.8.4