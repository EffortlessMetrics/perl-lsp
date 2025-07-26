# Tree-sitter Perl TODO

## 🎯 v0.4.0 - Parser Enhancements & Stabilization

### High Priority
- [ ] **Indirect Call Parsing** (Heuristic-based)
  - [ ] Implement `is_indirect_call_pattern()` lookahead logic
  - [ ] Add `parse_indirect_call()` handler
  - [ ] Unit tests for: `print STDOUT "..."`, `new Class`, `method $obj`
  - [ ] Document disambiguation rules

- [ ] **Multi-variable Attribute Support**
  - [ ] Finalize `parse_variable_list_declaration()` for `my ($x :shared, $y :locked)`
  - [ ] Add comprehensive unit tests
  - [ ] Validate against real-world Perl modules

- [ ] **Performance Benchmarking**
  - [ ] Add criterion benchmarks for lexer throughput
  - [ ] Add parser throughput benchmarks
  - [ ] Memory allocation profiling
  - [ ] Benchmark against large CPAN modules

- [ ] **Release Tasks**
  - [ ] Update CHANGELOG.md
  - [ ] Tag v0.4.0
  - [ ] Update documentation

## 🚀 v0.5.0 - Tooling & Integration

### Medium Priority
- [ ] **AST→Tree-sitter Bridge**
  - [ ] Design JSON-compatible AST format
  - [ ] Implement S-expression emitter
  - [ ] Preserve rich node type information
  - [ ] Add field names for Tree-sitter compatibility

- [ ] **Unified Test Suite**
  - [ ] Create `perl-parser-tests` crate
  - [ ] Share test fixtures across all implementations
  - [ ] Add corpus from CPAN modules
  - [ ] Benchmark all parsers with same inputs

- [ ] **Real-World Validation**
  - [ ] Test against top 100 CPAN modules
  - [ ] Parse Perl core test suite
  - [ ] Handle regex-heavy scripts
  - [ ] Document known limitations

## 🔭 v1.0 - Ecosystem Expansion

### Long-term Goals
- [ ] **Diagnostic & Linting APIs**
  - [ ] Unused variable detection
  - [ ] Ambiguous syntax warnings
  - [ ] Recoverable error reporting
  - [ ] Integration with perl-critic rules

- [ ] **AST Transformation Framework**
  - [ ] Visitor pattern implementation
  - [ ] Code formatting engine
  - [ ] Refactoring utilities
  - [ ] Macro expansion support

- [ ] **Language Server Protocol**
  - [ ] LSP server implementation
  - [ ] VS Code extension
  - [ ] Incremental parsing support
  - [ ] Real-time diagnostics

## 🧹 Ongoing Maintenance

- [ ] Improve crate documentation
- [ ] Add `#![deny(missing_docs)]`
- [ ] Clean up public APIs
- [ ] Set up CI/CD pipeline
- [ ] Publish to crates.io
- [ ] Create contribution guidelines

## 📊 Progress Tracking

### Completed ✅
- [x] Core lexer implementation
- [x] Recursive descent parser
- [x] Heredoc support
- [x] Modern Perl features
- [x] 141/141 edge case tests
- [x] Tree-sitter compatibility

### Current Focus 🎯
- Indirect call parsing
- Multi-variable attributes
- Performance optimization

### Blocked ⏸️
- None currently

## 🐛 Known Issues

1. **Comment Preservation**: Comments are currently discarded
2. **Error Recovery**: Limited recovery after syntax errors
3. **Incremental Parsing**: Not yet implemented
4. **Source Filters**: Not supported
5. **Format Declarations**: Partial support only

## 💡 Ideas for Future

- WASM build for browser-based parsing
- Perl-to-Rust transpiler proof of concept
- Integration with existing Perl tooling
- Compatibility layer for PPI users
- Syntax-aware diff tool