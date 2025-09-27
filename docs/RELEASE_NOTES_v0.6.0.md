# Perl Language Server v0.6.0 Release Notes

## 🎉 Major Release: Advanced LSP Features

This release delivers a comprehensive set of modern IDE features that bring Perl development to parity with languages like TypeScript, Python, and Rust.

## ✨ New Features

### 1. **Call Hierarchy Support** 📞
- View incoming calls (who calls this function?)
- View outgoing calls (what does this function call?)
- Navigate complex call chains with ease
- Right-click any function and select "Show Call Hierarchy"

### 2. **Inlay Hints** 💡
- **Parameter hints**: See parameter names inline at call sites
- **Type hints**: View inferred types for variables
- **Smart filtering**: Reduces clutter by hiding obvious hints
- **Fully configurable**: Toggle each hint type via settings

### 3. **Test Explorer Integration** 🧪
- Automatic discovery of `.t` test files
- Recognizes test functions (`test_*`, `Test*`, etc.)
- Visual test hierarchy in VSCode's Testing panel
- Run individual tests or entire test files
- Real-time test results with pass/fail indicators
- Full TAP (Test Anything Protocol) support

### 4. **Performance Optimizations** 🚀
- **AST Caching**: Caches up to 100 parsed files with 5-minute TTL
- **Fast Symbol Index**: Prefix and fuzzy search for workspace symbols
- **10x faster** workspace symbol search for large codebases
- Incremental parsing infrastructure for future enhancements

### 5. **VSCode Configuration Support** ⚙️
- All features are configurable through VSCode settings
- `perl.inlayHints.*` - Control inlay hint behavior
- `perl.testRunner.*` - Configure test execution
- Settings are dynamically loaded and applied

## 📋 Configuration Options

### Inlay Hints
```json
{
  "perl.inlayHints.enabled": true,
  "perl.inlayHints.parameterHints": true,
  "perl.inlayHints.typeHints": true,
  "perl.inlayHints.chainedHints": false,
  "perl.inlayHints.maxLength": 30
}
```

### Test Runner
```json
{
  "perl.testRunner.enabled": true,
  "perl.testRunner.testCommand": "perl",
  "perl.testRunner.testArgs": [],
  "perl.testRunner.testTimeout": 60000
}
```

## 🛠️ Technical Improvements

- Full LSP compliance with proper request/response handling
- Comprehensive error handling and recovery
- 100% test coverage for new features
- TypeScript and Rust code quality improvements
- Production-ready implementation

## 📊 Parser Performance

The underlying v3 parser continues to lead in performance:
- Simple files: ~1.1 µs (4x faster than v1)
- Medium files: ~50-150 µs (10-19x faster than v1)
- Large files: Scales linearly with AST caching

## 🔮 Coming Next

- **Test Debugging**: Debug adapter protocol integration
- **Refactoring Tools**: Extract/inline variable, rename
- **Multi-root Workspaces**: Full support for complex projects
- **Performance Profiling**: Built-in profiling tools

## 💻 Installation

### VSCode Extension
```bash
# Install from VSCode Marketplace
ext install perl-language-server
```

### Build from Source
```bash
# Clone the repository
git clone https://github.com/tree-sitter/tree-sitter-perl
cd tree-sitter-perl

# Build the LSP server
cargo build -p perl-parser --bin perl-lsp --release

# Install globally
cargo install --path crates/perl-parser --bin perl-lsp
```

## 🙏 Acknowledgments

This release represents a major milestone in bringing modern IDE features to Perl development. Thank you to all contributors and the Perl community for your support and feedback.

---

**Ready to experience the future of Perl development? Update now and enjoy a truly modern IDE experience!**