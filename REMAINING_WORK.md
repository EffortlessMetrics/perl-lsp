# Remaining Work Items

## 🎯 Immediate Priorities (Next Sprint)

### Parser Fixes
1. **Operator Precedence Issues**
   - Fix `or`, `and`, `not` operators (currently not parsing correctly)
   - Ensure proper precedence levels match Perl's perlop documentation
   - Add comprehensive operator precedence tests

2. **Built-in Function Signatures**
   - Complete signature database (currently ~40 functions)
   - Add remaining ~200+ Perl built-in functions
   - Include parameter types and documentation

3. **Edge Case Fixes**
   - Indirect object syntax in complex contexts
   - Quote-like operators with unusual delimiters
   - Complex prototype parsing edge cases

### LSP Enhancements
1. **Multi-file Support**
   - Cross-file symbol resolution
   - Package/module dependency tracking
   - Workspace-wide refactoring

2. **Performance Optimization**
   - Symbol caching for large files
   - Lazy AST construction
   - Background indexing

3. **Additional Code Actions**
   - Extract variable/function
   - Inline variable/function
   - Convert between quote styles
   - Add/remove use statements

## 📦 Distribution & Packaging

### Publishing
- [ ] Publish to crates.io
- [ ] Create GitHub releases with binaries
- [ ] Set up automated release pipeline

### Package Managers
- [ ] Homebrew formula (macOS/Linux)
- [ ] APT package (Debian/Ubuntu)
- [ ] RPM package (Fedora/RHEL)
- [ ] AUR package (Arch Linux)
- [ ] Chocolatey package (Windows)

### Editor Extensions
- [ ] VSCode extension (marketplace release)
- [ ] Neovim plugin (as separate repo)
- [ ] Emacs package (MELPA)
- [ ] Sublime Text package

## 🔧 Technical Debt

### Code Quality
- [ ] Remove dead code warnings in test_runner.rs
- [ ] Clean up unused debug_adapter.rs code
- [ ] Consolidate duplicate test utilities
- [ ] Improve error handling consistency

### Documentation
- [ ] API documentation for all public functions
- [ ] Architecture diagrams
- [ ] Performance tuning guide
- [ ] Contributing guidelines update

### Testing
- [ ] Increase test coverage to >90%
- [ ] Add fuzzing tests
- [ ] Benchmark regression tests
- [ ] Cross-platform CI improvements

## 🚀 Future Features (v0.7+)

### Advanced LSP Features
- [ ] Code formatting (Perl::Tidy integration)
- [ ] Call hierarchy navigation
- [ ] Code lens (inline test running)
- [ ] Workspace symbols search
- [ ] Semantic highlighting improvements

### Parser Enhancements
- [ ] Perl 7 syntax preparation
- [ ] Better error recovery
- [ ] Streaming parser for huge files
- [ ] Custom pragma support

### Developer Experience
- [ ] Language server installer script
- [ ] Configuration wizard
- [ ] Performance profiler
- [ ] Debug adapter protocol (DAP)

## 📝 Documentation Needs

### User Documentation
- [ ] Getting Started guide
- [ ] Editor setup tutorials
- [ ] Troubleshooting guide
- [ ] FAQ section

### Developer Documentation
- [ ] Parser architecture deep dive
- [ ] LSP protocol implementation notes
- [ ] Performance optimization guide
- [ ] Plugin development guide

## 🐛 Known Issues to Fix

### High Priority
1. Parser doesn't handle `or`/`and`/`not` operators
2. Some built-in functions missing from signature help
3. Workspace-wide operations not implemented

### Medium Priority
1. Memory usage could be optimized for very large files
2. Some Unicode edge cases in identifiers
3. Complex heredoc combinations may fail

### Low Priority
1. Formatting not implemented (needs Perl::Tidy)
2. Some code actions are placeholders
3. Debug adapter not fully implemented

## 📊 Success Metrics to Track

### Adoption
- [ ] Set up download/install tracking
- [ ] Create user feedback channels
- [ ] Monitor GitHub stars/forks
- [ ] Track issue resolution time

### Performance
- [ ] Continuous benchmark monitoring
- [ ] Memory usage tracking
- [ ] Response time metrics
- [ ] Large file handling tests

### Quality
- [ ] Code coverage reports
- [ ] Bug report tracking
- [ ] User satisfaction surveys
- [ ] Editor integration success rate

## 🎯 Definition of Done

For v1.0.0 release:
- [ ] All high-priority bugs fixed
- [ ] >95% test coverage
- [ ] Published to major package managers
- [ ] Official editor extensions released
- [ ] Complete user documentation
- [ ] Performance benchmarks documented
- [ ] Security audit completed
- [ ] Community feedback incorporated

---

*This document tracks all remaining work. Update as items are completed or new ones discovered.*

*Last updated: January 2025*