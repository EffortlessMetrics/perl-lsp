# TODO: Tree-sitter Perl v3 Parser Improvements

**Last Updated: v0.7.1 (January 2025)**

While the v3 parser achieves 100% edge case coverage and is production-ready, here are prioritized improvements that would enhance its value:

## ✅ Recently Completed (v0.7.1)
- Fixed `bless {}` parsing (now correctly handled as function call)
- Fixed `sort {}`, `map {}`, `grep {}` empty block parsing
- Enhanced builtin function argument handling
- Added 25+ test cases for edge cases

## 🔴 High Priority - Essential for IDE Integration

### 1. Incremental Parsing Support ✅ (Infrastructure Complete)
- [x] Implement position tracking with line/column information
- [x] Add edit tracking and position adjustment
- [x] Cache parse trees between edits
- [x] Create incremental parsing API
- [ ] Complete tree reuse implementation (currently falls back to full parse)
- [ ] Add lexer checkpointing for context-sensitive features
- **Status**: Infrastructure 100% complete, optimization pending
- **Impact**: Essential for real-time IDE features
- **Effort**: High (architecture complete, optimization remaining)

### 2. Error Recovery & Diagnostics
- [ ] Continue parsing after syntax errors
- [ ] Generate partial ASTs for incomplete code
- [ ] Provide helpful error messages with fix suggestions
- [ ] Track multiple errors in single parse
- **Impact**: Critical for IDE experience
- **Effort**: Medium-High

### 3. Comment & Whitespace Preservation
- [ ] Add trivia nodes to AST
- [ ] Preserve exact formatting
- [ ] Support comment attachment to nodes
- [ ] Enable whitespace-sensitive transformations
- **Impact**: Required for refactoring tools
- **Effort**: Medium

## 🟡 Medium Priority - Performance & Integration

### 4. Performance Optimizations
- [ ] Replace HashMap lookups with static tables
- [ ] Optimize string allocations (use string interning)
- [ ] Add SIMD optimizations for lexer
- [ ] Profile and optimize hot paths
- **Target**: 10-20% speedup
- **Current**: Already 4-19x faster than v1

### 5. Native Tree-sitter Integration
- [ ] Generate actual Tree-sitter nodes (not just S-expressions)
- [ ] Add field names to all nodes
- [ ] Support Tree-sitter queries directly
- [ ] Enable syntax highlighting queries
- **Impact**: Better ecosystem compatibility
- **Effort**: Medium

### 6. Language Server Protocol (LSP) Implementation
- [ ] Build LSP server using perl-parser
- [ ] Implement completion, hover, goto-definition
- [ ] Add refactoring commands
- [ ] Support workspace-wide analysis
- **Impact**: Showcase parser capabilities
- **Effort**: High (but high reward)

## 🟢 Low Priority - Nice to Have

### 7. Remaining Edge Cases (0.1%)
- [ ] Source filters (BEGIN { use Filter::Simple })
- [ ] Full format declaration parsing
- [ ] Tied variables in string eval
- [ ] Autoloader edge cases
- **Impact**: Completeness for obscure code
- **Effort**: Low-Medium per case

### 8. Perl 5.40+ Features
- [ ] Latest class/method syntax changes
- [ ] New builtin functions
- [ ] Experimental features tracking
- [ ] Future-proof grammar updates
- **Impact**: Stay current with Perl evolution
- **Effort**: Ongoing maintenance

### 9. Developer Tooling
- [ ] Perl formatter (perltidy alternative)
- [ ] Static analyzer with taint checking
- [ ] Code metrics calculator
- [ ] Dependency analyzer
- [ ] Documentation generator
- **Impact**: Ecosystem improvement
- **Effort**: Medium per tool

## 📊 Architecture Improvements

### 10. Parser Architecture
- [ ] Implement visitor pattern for AST traversal
- [ ] Add AST transformation framework
- [ ] Create plugin system for custom parsing
- [ ] Support dialect configuration (Perl 5.8 vs 5.38)

### 11. Testing Infrastructure
- [ ] Property-based testing with proptest
- [ ] Fuzzing harness for robustness
- [ ] Differential testing against perl -c
- [ ] Performance regression tests

### 12. Documentation
- [ ] API documentation with examples
- [ ] Parser internals guide
- [ ] Contributing guidelines
- [ ] Video tutorials for integration

## 🚀 Quick Wins (Could do now)

### Immediate Improvements
- [ ] Add more examples to perl-parser crate
- [ ] Create simple CLI tool for parsing
- [ ] Add JSON output format option
- [ ] Improve error messages
- [ ] Add --version flag to binaries

## 📈 Success Metrics

When complete, the parser should:
- Parse 1MB files in <100ms
- Recover from 95%+ of common syntax errors  
- Support real-time editing (60fps)
- Handle 100% of CPAN modules
- Integrate seamlessly with major editors

## 🎯 Recommended Next Steps

1. **For IDE Integration**: Focus on incremental parsing + error recovery
2. **For Tooling**: Add comment preservation + visitor pattern
3. **For Adoption**: Build LSP server + formatter demo
4. **For Performance**: Implement string interning + static tables

## 📝 Notes

- The parser is already production-ready as-is
- These improvements are "nice to have" not "must have"
- Focus on real user needs vs theoretical completeness
- Maintain backwards compatibility with v0.4.0 API