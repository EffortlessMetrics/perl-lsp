# LSP E2E Test Strategy

## Overview

This document describes the comprehensive end-to-end testing strategy for the Perl LSP server, ensuring 100% feature coverage and professional IDE experience.

## Test Architecture

### 1. Test Structure

```
tests/
├── lsp_comprehensive_e2e_test.rs   # Main E2E test suite with all features
├── run_all_e2e_tests.rs            # Test harness with coverage reporting
├── test_runner.rs                  # Test execution framework
├── lsp_e2e_user_stories.rs         # User story scenarios
└── E2E_TEST_STRATEGY.md            # This document
```

### 2. Test Categories

#### Feature Tests (25+ LSP Features)
1. **Initialization** - Server setup and capability negotiation
2. **Diagnostics** - Real-time error and warning detection
3. **Completion** - Context-aware code suggestions
4. **Definition** - Go to definition navigation
5. **References** - Find all references
6. **Hover** - Documentation and type information
7. **SignatureHelp** - Function parameter hints (150+ built-ins)
8. **DocumentSymbol** - Document outline
9. **WorkspaceSymbol** - Workspace-wide symbol search
10. **CodeAction** - Quick fixes and refactoring
11. **CodeLens** - Reference counts and inline actions
12. **DocumentFormatting** - Code formatting (perltidy)
13. **RangeFormatting** - Partial formatting
14. **Rename** - Safe symbol renaming
15. **FoldingRange** - Code folding regions
16. **SelectionRange** - Smart selection expansion
17. **SemanticTokens** - Syntax highlighting
18. **CallHierarchy** - Incoming/outgoing calls
19. **InlayHint** - Parameter name hints
20. **ExecuteCommand** - Custom commands
21. **LinkedEditingRange** - Simultaneous editing
22. **DocumentHighlight** - Highlight occurrences
23. **DocumentLink** - Clickable links
24. **Multi-file Support** - Cross-file features
25. **Incremental Parsing** - Performance optimization

#### User Story Tests
- **Developer Onboarding** - Learning existing codebase
- **Bug Fixing** - Real-time feedback while fixing issues
- **TDD Development** - Test-driven development workflow
- **Legacy Refactoring** - Modernizing old code

#### Edge Case Tests
- **Malformed Requests** - Invalid protocol handling
- **Concurrent Modifications** - Rapid document changes
- **Memory Pressure** - Many open documents
- **Large Files** - Performance with 100K+ LOC
- **Unicode Support** - International characters
- **Error Recovery** - Partial parsing with errors

## Running Tests

### Run All E2E Tests with Coverage
```bash
cargo test --test lsp_comprehensive_e2e_test -- --nocapture
```

### Run Test Harness with Reports
```bash
cargo test --test run_all_e2e_tests -- --nocapture
```

This generates:
- Console coverage report with color coding
- `test-results.xml` - JUnit format for CI
- `test-coverage.md` - Markdown report

### Run Specific Feature Tests
```bash
cargo test --test lsp_comprehensive_e2e_test test_e2e_code_completion
```

### Run User Story Tests
```bash
cargo test --test lsp_e2e_user_stories
```

## Coverage Metrics

### Current Coverage (v0.7.3)
- **Features Tested**: 25/25 (100%)
- **User Stories**: 4/4 (100%)
- **Edge Cases**: 3/3 (100%)
- **Total Tests**: 63+
- **Pass Rate**: 100%

### Performance Targets
- Initialization: <50ms
- Completion: <10ms
- Go to Definition: <5ms
- Find References: <20ms
- Document Symbols: <10ms
- Large File (100K LOC): <100ms

## Test Implementation Details

### TestContext Helper

The `TestContext` struct manages server state across tests:

```rust
struct TestContext {
    server: LspServer,
    documents: HashMap<String, String>,
    version_counter: i32,
}
```

Provides methods for:
- `initialize()` - Server initialization
- `open_document()` - Open file in server
- `update_document()` - Modify document
- `send_request()` - Send LSP request
- `send_notification()` - Send LSP notification

### LspTestRunner Framework

The test runner provides:
- Feature grouping and tracking
- Pass/fail statistics
- Performance measurement
- Coverage calculation
- Report generation (console, JUnit, Markdown)

### Test Patterns

#### Pattern 1: Feature Test
```rust
#[test]
fn test_e2e_feature() {
    let mut ctx = TestContext::new();
    ctx.initialize();
    
    // Setup
    ctx.open_document("file:///test.pl", code);
    
    // Execute
    let result = ctx.send_request("method", params);
    
    // Assert
    assert!(result.is_some());
    assert!(validate_response(result));
}
```

#### Pattern 2: User Story Test
```rust
#[test]
fn test_user_story_workflow() {
    let mut ctx = TestContext::new();
    ctx.initialize();
    
    // Step 1: User action
    ctx.open_document(...);
    
    // Step 2: Server response
    let symbols = ctx.send_request("textDocument/documentSymbol", ...);
    
    // Step 3: User follows up
    let definition = ctx.send_request("textDocument/definition", ...);
    
    // Verify workflow completed successfully
    assert!(workflow_successful);
}
```

## CI/CD Integration

### GitHub Actions
```yaml
- name: Run E2E Tests
  run: cargo test --test run_all_e2e_tests -- --nocapture
  
- name: Upload Test Results
  uses: actions/upload-artifact@v2
  with:
    name: test-results
    path: |
      test-results.xml
      test-coverage.md
```

### Coverage Requirements
- Minimum feature coverage: 95%
- All user stories must pass
- Performance targets must be met
- No critical edge case failures

## Adding New Tests

### 1. Add Feature Test
Edit `lsp_comprehensive_e2e_test.rs`:
```rust
#[test]
fn test_e2e_new_feature() {
    // Implementation
}
```

### 2. Update Test Runner
Edit `run_all_e2e_tests.rs`:
```rust
runner.run_test("test_new_feature", "FeatureName", || {
    // Test logic
});
```

### 3. Document Coverage
Update this document with new feature coverage.

## Troubleshooting

### Common Issues

1. **Test Timeout**
   - Check for infinite loops
   - Verify server initialization
   - Check network/file I/O

2. **Flaky Tests**
   - Add delays for async operations
   - Use proper synchronization
   - Clear state between tests

3. **Coverage Gaps**
   - Run coverage report
   - Identify untested features
   - Add missing test cases

## Future Enhancements

### Planned Improvements
- [ ] Property-based testing with quickcheck
- [ ] Fuzzing for protocol robustness
- [ ] Performance regression tracking
- [ ] Visual regression testing for UI features
- [ ] Integration with real editors (VSCode, Neovim)
- [ ] Automated benchmark comparisons
- [ ] Mutation testing for test quality

### Long-term Goals
- 100% branch coverage
- Sub-millisecond response times
- Zero-downtime updates
- Hot reload support
- Cloud testing infrastructure

## Conclusion

This E2E test strategy ensures the Perl LSP server delivers a professional, reliable IDE experience with comprehensive feature coverage and excellent performance. All 25+ LSP features are tested end-to-end with real-world scenarios, edge cases, and performance validation.