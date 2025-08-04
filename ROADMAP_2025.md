# tree-sitter-perl Roadmap 2025

## Current State (v0.5.0) ✅

- **Three complete parsers**: v1 (C), v2 (Pest), v3 (Native)
- **Full LSP server** with 8 professional IDE features
- **100% edge case coverage** with v3 parser
- **World-class performance**: 1-150µs parsing

## Q1 2025: v0.6.0 - Enhanced IDE Experience

### LSP Features
- [ ] **Code Formatting** - Perl::Tidy integration
- [ ] **Advanced Refactoring**
  - [ ] Extract/inline variable
  - [ ] Extract subroutine  
  - [ ] Safe rename across files
- [ ] **Code Lens** - Run tests, show references
- [ ] **Quick Fixes** - Auto-import, fix syntax

### Editor Integration
- [ ] **VSCode Extension** - Official marketplace release
- [ ] **Neovim Plugin** - Native Lua implementation
- [ ] **Emacs Package** - MELPA distribution

**Deliverables**: Professional IDE experience for Perl developers

## Q2 2025: v0.7.0 - Performance at Scale

### Incremental Parsing
- [ ] Parse only changed regions
- [ ] Persistent AST caching
- [ ] Background indexing
- [ ] Memory-mapped files

### Performance Targets
- [ ] <1ms incremental updates
- [ ] <100ms for 10K LOC files
- [ ] <10ms LSP response time
- [ ] <1GB memory for 1M LOC

**Deliverables**: Enterprise-scale performance

## Q3 2025: v0.8.0 - AI & Automation

### MCP Integration
- [ ] Model Context Protocol server
- [ ] Natural language code search
- [ ] AI-powered refactoring
- [ ] Automated code reviews

### Static Analysis
- [ ] Security vulnerability scanner
- [ ] Complexity metrics
- [ ] Dead code detection
- [ ] SARIF output format

**Deliverables**: AI-assisted development tools

## Q4 2025: v0.9.0 - Modern Perl Support

### Language Features
- [ ] Perl 7 syntax preparation
- [ ] Enhanced signatures
- [ ] Coroutine support
- [ ] Type annotations (optional)

### Migration Tools
- [ ] Perl 5→7 compatibility checker
- [ ] Automated upgrades
- [ ] Deprecation warnings

**Deliverables**: Future-proof parser for Perl evolution

## Community Goals

### Adoption Targets
- 10,000+ VSCode extension installs
- 100+ GitHub stars
- 50+ contributors
- 5+ enterprise users

### Ecosystem
- Active Discord community
- Monthly releases
- Conference talks
- Video tutorials

## How to Contribute

### High Priority Areas
1. **VSCode Extension** - Biggest user impact
2. **Code Formatting** - Most requested feature
3. **Performance** - Always critical
4. **Documentation** - User success

### Getting Started
```bash
# Clone and build
git clone https://github.com/tree-sitter-perl/tree-sitter-perl
cd tree-sitter-perl
cargo build --all

# Run tests
cargo test --all

# Try the LSP
cargo run -p perl-parser --bin perl-lsp -- --stdio
```

## Release Schedule

- **v0.5.0** - January 2025 - LSP Foundation ✅
- **v0.6.0** - April 2025 - Enhanced IDE
- **v0.7.0** - July 2025 - Performance
- **v0.8.0** - October 2025 - AI Features
- **v0.9.0** - January 2026 - Modern Perl

---

For detailed feature planning, see [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md).

*Join us in building the future of Perl tooling!*