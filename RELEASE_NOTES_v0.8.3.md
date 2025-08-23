# Release Notes - v0.8.3

## 🎯 Parser Improvements

### Hash Literal Parsing
- **Fixed**: `{ key => value }` now correctly produces `HashLiteral` nodes instead of wrapping in `Block`
- **Context-aware**: Statement context produces `(block (hash ...))`, expression context produces `(hash ...)`
- **Refactored**: Shared `build_list_or_hash()` utility eliminates code duplication

### Parenthesized Expressions
- **Fixed**: Word operators in parentheses now parse correctly (e.g., `($a or $b)`)
- **Smart detection**: Parenthesized lists emit `HashLiteral` only when real `=>` is present
- **Consistent**: Even number of elements with fat arrow produces hash, otherwise array

### Quote Words (qw)
- **Fixed**: `qw()` now produces `ArrayLiteral` nodes with proper word elements
- **Complete support**: All delimiter types work correctly (`()`, `[]`, `{}`, `<>`, `/…/`, `!…!`, etc.)
- **Test coverage**: Added comprehensive tests for all delimiter variants

## 🔧 LSP Enhancements

### Go-to-Definition
- **Fixed**: Now uses `DeclarationProvider` for accurate function location finding
- **Improved**: Correctly resolves function calls to their definitions
- **Test coverage**: All 33 LSP E2E tests passing

### Inlay Hints
- **Enhanced**: Recognizes `HashLiteral` nodes even when wrapped in blocks
- **Smart detection**: KV-array heuristic retained for legacy shapes
- **Type inference**: Better handling of hash references like `{ key => "value" }`

## ✅ Quality Improvements

### Code Organization
- **Refactored**: Hash/array detection logic into shared utility for consistency
- **Tests added**: Statement vs expression context test documents intended behavior
- **Clean code**: No dead code, all tests meaningful

### Test Results
- ✅ All 147 parser library tests passing
- ✅ All 33 LSP comprehensive E2E tests passing
- ✅ All 10 bless parsing tests passing
- ✅ All 10 integration tests passing
- ✅ 100% edge case coverage maintained

## 📦 Installation

### From crates.io
```bash
cargo install perl-parser --bin perl-lsp
```

### Quick Install (Linux/macOS)
```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

### Homebrew (macOS)
```bash
brew tap tree-sitter-perl/tap
brew install perl-lsp
```

## 🚀 What's Next

- Enhanced cross-file navigation
- Workspace-wide refactoring operations
- Import optimization
- Debug adapter protocol support

## 💡 Notes

This release focuses on parser accuracy and LSP reliability. The parser now correctly handles all edge cases around hash literals, parenthesized expressions, and quote words, while the LSP server provides accurate go-to-definition support.