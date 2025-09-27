# Perl LSP Development Roadmap

## Executive Summary
This roadmap outlines a systematic approach to:
1. Complete missing LSP features (15% gap)
2. Convert 520+ mock tests to real tests
3. Clean up technical debt and improve code quality

**Timeline**: 12 weeks (3 months)
**Goal**: Production-grade LSP with 100% real test coverage

---

## Phase 1: Foundation & Quick Wins (Weeks 1-2)
*Focus: High-impact, low-effort features + test infrastructure*

### Week 1: Critical Missing Features
#### LSP Features
- [ ] **Document Highlight** (`textDocument/documentHighlight`)
  - Reuse existing reference finder
  - Add highlight kind (Text, Read, Write)
  - Test: 5 real tests covering various symbols
  
- [ ] **Document Lifecycle Events**
  - [ ] `textDocument/didSave` - Post-save hooks
  - [ ] `textDocument/didClose` - Cleanup resources
  - [ ] `textDocument/willSave` - Pre-save validation
  - Test: 3 tests per event (9 total)

#### Test Infrastructure
- [ ] Create test conversion framework
  - [ ] Helper functions for real LSP server setup
  - [ ] Standardized request/response validation
  - [ ] Performance measurement utilities
  - [ ] Document conversion process

### Week 2: Declaration Support & Test Conversion
#### LSP Features
- [ ] **Declaration** (`textDocument/declaration`)
  - Distinguish `my $var` (declaration) from `$var = 5` (definition)
  - Support for subroutines, packages, constants
  - Test: 8 real tests for different declaration types

#### Test Conversion (Batch 1: 50 tests)
- [ ] Convert `lsp_integration_test.rs` (3 tests) - Remove MockIO
- [ ] Convert `lsp_code_actions_tests.rs` (9 tests)
- [ ] Convert `lsp_completion_tests.rs` (17 tests)
- [ ] Convert `lsp_builtins_test.rs` (9 tests)
- [ ] Convert `lsp_folding_ranges_test.rs` (7 tests)
- [ ] Convert `lsp_document_symbols_test.rs` (7 tests)

**Milestone 1**: Core features complete, 50 real tests converted

---

## Phase 2: Enhanced Navigation & Testing (Weeks 3-5)
*Focus: Advanced navigation features + systematic test conversion*

### Week 3: Document Links & Smart Selection
#### LSP Features
- [ ] **Document Links** (`textDocument/documentLink`)
  - [ ] URLs in comments and POD
  - [ ] Module names (`use Module::Name`)
  - [ ] File paths in strings
  - [ ] CPAN module links
  - Test: 10 real tests

- [ ] **Selection Range** (`textDocument/selectionRange`)
  - [ ] AST-based selection expansion
  - [ ] Variable → Expression → Statement → Block
  - Test: 8 real tests

#### Test Conversion (Batch 2: 75 tests)
- [ ] Convert `lsp_advanced_features_test.rs` (23 tests)
- [ ] Convert `lsp_signature_integration_test.rs` (13 tests)
- [ ] Convert `lsp_edge_cases_test.rs` (13 tests)
- [ ] Convert `lsp_error_recovery.rs` (11 tests)
- [ ] Convert `lsp_golden_tests.rs` (6 tests)
- [ ] Convert `lsp_code_lens_reference_test.rs` (2 tests)
- [ ] Convert `lsp_execute_command_tests.rs` (4 tests)

### Week 4: Type System Features
#### LSP Features
- [ ] **Type Definition** (`textDocument/typeDefinition`)
  - [ ] Blessed references to packages
  - [ ] Moose/Moo type constraints
  - [ ] Role compositions
  - Test: 12 real tests

- [ ] **Implementation** (`textDocument/implementation`)
  - [ ] Find role implementations
  - [ ] Method overrides in subclasses
  - [ ] Interface implementations
  - Test: 10 real tests

#### Test Conversion (Batch 3: 75 tests)
- [ ] Convert `lsp_e2e_user_stories.rs` (16 tests)
- [ ] Convert `lsp_critical_user_stories.rs` (5 tests)
- [ ] Convert `lsp_full_coverage_user_stories.rs` (16 tests)
- [ ] Convert `lsp_missing_user_stories.rs` (6 tests)
- [ ] Convert `lsp_unhappy_paths.rs` (18 tests)
- [ ] Convert `lsp_encoding_edge_cases.rs` (14 tests)

### Week 5: On-Type Formatting & Performance
#### LSP Features
- [ ] **On-Type Formatting** (`textDocument/onTypeFormatting`)
  - [ ] Auto-indent on `{`, `}`
  - [ ] Align hash arrows `=>`
  - [ ] Smart semicolon insertion
  - [ ] POD formatting
  - Test: 15 real tests

#### Test Conversion (Batch 4: 75 tests)
- [ ] Convert `lsp_filesystem_failures.rs` (15 tests)
- [ ] Convert `lsp_protocol_violations.rs` (26 tests)
- [ ] Convert `lsp_concurrency.rs` (10 tests)
- [ ] Convert `lsp_stress_tests.rs` (10 tests)
- [ ] Convert `lsp_memory_pressure.rs` (14 tests)

#### Performance Optimization
- [ ] Benchmark all LSP operations
- [ ] Optimize hot paths (parsing, symbol lookup)
- [ ] Add caching for expensive operations
- [ ] Memory usage profiling

**Milestone 2**: Navigation features complete, 275 real tests (53% converted)

---

## Phase 3: Workspace & Debugging Features (Weeks 6-8)
*Focus: Multi-file support + remaining test conversion*

### Week 6: Workspace Management
#### LSP Features
- [ ] **Workspace Folders** (`workspace/workspaceFolders`)
  - [ ] Multi-root workspace support
  - [ ] Cross-project symbol search
  - [ ] Project-specific settings
  - Test: 10 real tests

- [ ] **File Watching** (`workspace/didChangeWatchedFiles`)
  - [ ] Monitor external file changes
  - [ ] Update symbol index on changes
  - [ ] Invalidate caches appropriately
  - Test: 8 real tests

- [ ] **Workspace Edits** (`workspace/applyEdit`)
  - [ ] Multi-file refactoring
  - [ ] Transactional edits
  - [ ] Undo/redo support
  - Test: 12 real tests

#### Test Conversion (Batch 5: 75 tests)
- [ ] Convert `lsp_integration_tests.rs` (16 tests)
- [ ] Convert `lsp_real_world_integration.rs` (8 tests)
- [ ] Convert `lsp_security_edge_cases.rs` (12 tests)
- [ ] Convert `lsp_final_coverage.rs` (6 tests)
- [ ] Convert `lsp_master_integration_test.rs` (3 tests)
- [ ] Convert `lsp_performance_benchmarks.rs` (30 tests)

### Week 7: Advanced Diagnostics & Analysis
#### LSP Features
- [ ] **Pull Diagnostics** (`textDocument/diagnostic`)
  - [ ] On-demand diagnostic computation
  - [ ] Partial document diagnostics
  - [ ] Related information links
  - Test: 10 real tests

- [ ] **Inline Values** (`textDocument/inlineValue`)
  - [ ] Variable value preview (debugging)
  - [ ] Expression evaluation
  - [ ] Watch expressions
  - Test: 8 real tests

#### Test Conversion (Batch 6: 75 tests)
- [ ] Convert remaining test files
- [ ] Create new tests for uncovered scenarios
- [ ] Add integration tests for multi-file operations
- [ ] Add stress tests for large codebases

### Week 8: Polish & Documentation
#### Code Quality
- [ ] **Refactor duplicate code**
  - [ ] Extract common LSP response builders
  - [ ] Consolidate error handling
  - [ ] Standardize logging

- [ ] **Error Handling**
  - [ ] Graceful degradation for unsupported features
  - [ ] Better error messages
  - [ ] Recovery strategies for parser failures

#### Test Conversion (Final Batch: 95 tests)
- [ ] Convert all remaining mock tests
- [ ] Add missing edge case tests
- [ ] Create performance regression tests
- [ ] Add fuzzing tests for robustness

**Milestone 3**: All workspace features complete, 520 real tests (100% converted)

---

## Phase 4: Optimization & Production Hardening (Weeks 9-10)
*Focus: Performance, reliability, and production readiness*

### Week 9: Performance Optimization
- [ ] **Parser Performance**
  - [ ] Incremental parsing implementation
  - [ ] Parallel parsing for multiple files
  - [ ] Lazy AST construction
  - Target: <50ms for 5000-line files

- [ ] **Memory Optimization**
  - [ ] Implement LRU cache for parsed files
  - [ ] Memory pool for AST nodes
  - [ ] String interning for identifiers
  - Target: <100MB for 100-file workspace

- [ ] **Response Time Optimization**
  - [ ] Async request processing
  - [ ] Request prioritization
  - [ ] Cancellation support
  - Target: <100ms for all interactive operations

### Week 10: Production Hardening
- [ ] **Reliability**
  - [ ] Crash recovery mechanisms
  - [ ] State persistence
  - [ ] Graceful shutdown
  - [ ] Connection retry logic

- [ ] **Monitoring & Telemetry**
  - [ ] Performance metrics collection
  - [ ] Error tracking
  - [ ] Usage analytics (opt-in)
  - [ ] Debug logging improvements

- [ ] **Security**
  - [ ] Input validation
  - [ ] Path traversal prevention
  - [ ] Resource consumption limits
  - [ ] Sandboxing untrusted code

**Milestone 4**: Production-ready performance and reliability

---

## Phase 5: Integration & Deployment (Weeks 11-12)
*Focus: Editor integration and deployment preparation*

### Week 11: Editor Integration
- [ ] **VS Code Extension**
  - [ ] Update extension to use all features
  - [ ] Add configuration UI
  - [ ] Create feature showcase
  - [ ] Performance profiling

- [ ] **Neovim Integration**
  - [ ] Update Neovim config examples
  - [ ] Test with popular Neovim distributions
  - [ ] Create installation guide

- [ ] **Other Editors**
  - [ ] Emacs configuration
  - [ ] Sublime Text package
  - [ ] IntelliJ/IDEA plugin guidance

### Week 12: Documentation & Release
- [ ] **User Documentation**
  - [ ] Feature overview
  - [ ] Installation guides per platform
  - [ ] Configuration reference
  - [ ] Troubleshooting guide

- [ ] **Developer Documentation**
  - [ ] Architecture overview
  - [ ] Contributing guide
  - [ ] API documentation
  - [ ] Test writing guide

- [ ] **Release Preparation**
  - [ ] Version bump to 1.0.0
  - [ ] Changelog generation
  - [ ] Release notes
  - [ ] Announcement blog post

**Milestone 5**: Version 1.0.0 released

---

## Success Metrics

### Quantitative Goals
- ✅ **Features**: 100% LSP specification coverage for Perl-relevant features
- ✅ **Tests**: 520 real tests (0 mocks/stubs)
- ✅ **Performance**: <100ms response time for all operations
- ✅ **Memory**: <100MB for typical workspace
- ✅ **Reliability**: 99.9% uptime in production use

### Qualitative Goals
- ✅ Clean, maintainable codebase
- ✅ Comprehensive documentation
- ✅ Active community engagement
- ✅ Positive user feedback
- ✅ Easy installation process

---

## Risk Mitigation

### Technical Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Parser performance issues | High | Incremental parsing, caching |
| Memory leaks | High | Profiling, automated testing |
| Complex Perl syntax edge cases | Medium | Extensive test corpus |
| Breaking changes in LSP spec | Low | Version pinning, gradual adoption |

### Resource Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Developer availability | High | Clear documentation, modular design |
| Test conversion complexity | Medium | Automation tools, templates |
| User adoption | Medium | Good documentation, easy setup |

---

## Dependencies

### Technical Dependencies
- Rust stable (1.70+)
- Tree-sitter Perl grammar
- LSP types crate
- Tokio for async runtime

### Resource Requirements
- 1-2 developers full-time
- CI/CD infrastructure
- Test corpus of real Perl code
- User feedback channel

---

## Parallel Workstreams

These can be worked on independently:

### Stream 1: Feature Development
- Owner: Core developer
- Focus: Implementing missing LSP features
- Timeline: Weeks 1-8

### Stream 2: Test Conversion
- Owner: Test engineer or contributor
- Focus: Converting mock tests to real tests
- Timeline: Weeks 1-8 (parallel)

### Stream 3: Documentation
- Owner: Technical writer or contributor
- Focus: User and developer documentation
- Timeline: Weeks 6-12

### Stream 4: Performance
- Owner: Performance engineer
- Focus: Optimization and benchmarking
- Timeline: Weeks 5-10

---

## Next Steps

### Immediate Actions (This Week)
1. [ ] Review and approve roadmap
2. [ ] Assign owners to workstreams
3. [ ] Set up project tracking (GitHub Projects/Issues)
4. [ ] Create test conversion templates
5. [ ] Begin Phase 1 implementation

### Communication Plan
- Weekly progress updates
- Bi-weekly community calls
- Monthly blog posts
- Continuous GitHub discussions

---

## Appendix: Test Conversion Template

```rust
// BEFORE: Mock test
#[test]
fn test_feature_mock() {
    let mut context = ExtendedTestContext::new();
    context.mock_response("completion", json!([]));
    // ... mock assertions
}

// AFTER: Real test
#[test]
fn test_feature_real() {
    let server = TestServer::new();
    let response = server.request("textDocument/completion", json!({
        "textDocument": { "uri": "file:///test.pl" },
        "position": { "line": 0, "character": 5 }
    }));
    assert!(response["items"].as_array().unwrap().len() > 0);
    // ... real assertions
}
```

---

## Version History
- v1.0 - Initial roadmap (2024-02)
- v1.1 - Added test conversion details
- v1.2 - Added parallel workstreams

---

*This is a living document. Updates will be made as the project progresses.*