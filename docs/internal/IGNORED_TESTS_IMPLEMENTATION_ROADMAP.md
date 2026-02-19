# Ignored Tests Implementation Roadmap
<!-- Labels: testing:roadmap, implementation:prioritized, lsp:comprehensive -->

**Issue Context**: #145 - Critical LSP features have ignored tests - executeCommand and code actions missing
**Document Purpose**: Map ignored tests to implementation requirements and prioritization strategy
**Created**: 2025-01-15

## Executive Summary

This document provides a comprehensive analysis of currently ignored tests in the Perl LSP test suite, mapping them to specific implementation requirements and establishing a prioritized roadmap for systematic test enablement. The analysis focuses on tests directly related to Issue #145 while providing context for broader test infrastructure improvements.

## Ignored Test Analysis by Priority

### P0 - Critical for Issue #145 Resolution

#### 1. Execute Command Tests
**File**: `crates/perl-lsp/tests/lsp_behavioral_tests.rs`
**Test Function**: `test_execute_command_perlcritic`
**Ignore Reason**: `"executeCommand not implemented - required for perlcritic integration"`

**Current Status Analysis**:
- ✅ **Implementation EXISTS**: `run_perl_critic` method implemented in `lsp_server.rs`
- ✅ **perlcritic Integration**: Both external and built-in analyzer support
- ✅ **Command Registration**: `perl.runCritic` included in supported commands
- ❌ **Test IGNORED**: Due to incomplete understanding of implementation status

**Implementation Gap**:
```rust
// The test is ignored but the functionality appears to be implemented
#[test]
#[ignore = "executeCommand not implemented - required for perlcritic integration"]
fn test_execute_command_perlcritic() {
    // Execute perl.runCritic command (with extended timeout for potential external tool)
    let execute_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "perl.runCritic",
            "arguments": ["/test.pl"]
        },
        "id": 123
    });
}
```

**Action Required**:
1. Remove `#[ignore]` attribute and validate test execution
2. Update test with realistic expectations for external/built-in analyzer scenarios
3. Add comprehensive error handling validation

**Acceptance Criteria Mapping**: AC2:runCritic, AC4:enabledTests

### P1 - High Priority for Complete Feature Set

#### 2. Code Action Integration Tests
**File**: Multiple files with code action test infrastructure
**Implementation Status**:
- ✅ **Provider EXISTS**: `EnhancedCodeActionsProvider` implemented
- ❌ **LSP INTEGRATION**: Not wired to `textDocument/codeAction` handler

**Required Integration**:
```rust
// Missing handler in lsp_server.rs
fn handle_code_action(&mut self, params: CodeActionParams) -> Result<Vec<CodeActionOrCommand>, JsonRpcError> {
    let document = self.get_document(&params.text_document.uri)?;
    let enhanced_provider = EnhancedCodeActionsProvider::new(document.content.clone());

    let actions = enhanced_provider.get_enhanced_refactoring_actions(
        &document.ast,
        (params.range.start.byte, params.range.end.byte)
    );

    // Convert to LSP protocol format
    Ok(actions.into_iter().map(|action| CodeActionOrCommand::CodeAction(action)).collect())
}
```

**Action Required**:
1. Wire `EnhancedCodeActionsProvider` to LSP server
2. Add capability advertisement in initialization
3. Enable related ignored tests for validation

**Acceptance Criteria Mapping**: AC3:codeActions, AC5:integration

### P2 - Important for Protocol Compliance

#### 3. Code Lens Tests
**Files**:
- `crates/perl-lsp/tests/lsp_code_lens_reference_test.rs`
- `crates/perl-lsp/tests/lsp_comprehensive_3_17_test.rs`

**Test Functions**:
- `test_code_lens_reference_counting`
- `test_code_lens_package_references`
- `test_code_lens_3_17`

**Ignore Reason**: `"Code lens is not advertised by default (partial implementation)"`

**Analysis**:
- ✅ **Implementation EXISTS**: Code lens functionality implemented
- ❌ **Not Advertised**: Server capabilities don't include code lens by default
- ❌ **Partial Feature**: Implementation exists but not enabled in standard configuration

**Implementation Gap**:
```rust
// Server capabilities missing code lens advertisement
"capabilities": {
    "codeLensProvider": {
        "resolveProvider": true
    }
}
```

**Action Required**:
1. Review code lens implementation completeness
2. Decide on default enablement strategy
3. Update server capabilities if appropriate
4. Remove ignores or update with clear reason for optional feature

**Priority Justification**: P2 - Not critical for Issue #145 but important for comprehensive LSP support

### P3 - Protocol Robustness (Lower Priority)

#### 4. Malformed Frame Tests
**File**: `crates/perl-lsp/tests/lsp_malformed_frames_test.rs`
**Test Functions**: 5 ignored tests for protocol edge cases
**Ignore Reason**: Various server-specific behavior and implementation-specific handling

**Analysis**: Edge case protocol handling tests that validate robustness but not core functionality.
**Action**: Review after P0-P2 completion, enable where implementation behavior is defined

#### 5. Protocol Invariant Tests
**File**: `crates/perl-lsp/tests/lsp_invariants_test.rs`
**Test Functions**: 3 ignored tests for protocol compliance validation
**Ignore Reason**: Implementation-specific behavior, timing sensitivity, auto-generated IDs

**Analysis**: Protocol compliance validation tests that may need adjustment for current implementation patterns.
**Action**: Review for applicability to current LSP server architecture

#### 6. Unhappy Path Tests
**File**: `crates/perl-lsp/tests/lsp_unhappy_paths.rs`
**Test Functions**: 10 ignored tests for error handling scenarios
**Ignore Reason**: Various edge cases and error recovery scenarios

**Analysis**: Comprehensive error handling validation that should be addressed for production readiness.
**Action**: Systematic review and enablement as part of quality assurance phase

### P4 - Development Tools (Optional)

#### 7. Performance Benchmarks
**File**: `crates/perl-lsp/tests/lsp_performance_benchmarks.rs`
**Test Function**: `benchmark_summary`
**Ignore Reason**: `"Run with --ignored to execute"`

**Analysis**: Development tool for performance analysis, correctly ignored for normal test runs.
**Action**: No change required - working as intended

#### 8. Capability Snapshots
**File**: `crates/perl-lsp/tests/lsp_capabilities_snapshot.rs`
**Test Function**: `regenerate_snapshots`
**Ignore Reason**: Utility for snapshot regeneration

**Analysis**: Development utility, correctly ignored for normal test runs.
**Action**: No change required - working as intended

## Implementation Roadmap by Phase

### Phase 1: Critical executeCommand Integration (Week 1)
**Objective**: Resolve Issue #145 core requirements

#### Week 1.1: perl.runCritic Test Enablement
- [ ] Remove `#[ignore]` from `test_execute_command_perlcritic` in `lsp_behavioral_tests.rs`
- [ ] Validate test execution with existing implementation
- [ ] Fix any discovered integration issues
- [ ] Add comprehensive error scenario coverage

**Test Command**:
```bash
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_perlcritic
```

**Success Criteria**:
- Test passes with both external perlcritic and built-in analyzer scenarios
- Proper error handling for tool unavailability
- Response format matches LSP protocol expectations

#### Week 1.2: Execute Command Infrastructure Validation
- [ ] Verify all `executeCommandProvider` capabilities are properly advertised
- [ ] Validate command registration in server initialization
- [ ] Test comprehensive command parameter validation

**Test Command**:
```bash
cargo test -p perl-lsp --test lsp_execute_command_tests
```

### Phase 2: Code Action Integration (Week 2)
**Objective**: Enable advanced refactoring through LSP protocol

#### Week 2.1: LSP Server Code Action Handler
- [ ] Implement `handle_code_action` method in `lsp_server.rs`
- [ ] Wire `EnhancedCodeActionsProvider` to LSP protocol
- [ ] Add `codeActionProvider` to server capabilities

**Implementation Target**:
```rust
fn handle_code_action(&mut self, params: CodeActionParams) -> Result<Vec<CodeActionOrCommand>, JsonRpcError> {
    // Integration code here
}
```

#### Week 2.2: Code Action Test Infrastructure
- [ ] Identify and enable relevant code action tests
- [ ] Add comprehensive refactoring workflow validation
- [ ] Test cross-file refactoring with dual indexing

**Test Command**:
```bash
cargo test -p perl-lsp -- code_action
```

### Phase 3: Protocol Compliance and Robustness (Week 3)
**Objective**: Comprehensive LSP protocol support

#### Week 3.1: Code Lens Decision and Implementation
- [ ] Review code lens implementation completeness
- [ ] Decide on default enablement strategy
- [ ] Update server capabilities and test expectations

#### Week 3.2: Error Handling and Edge Cases
- [ ] Systematic review of unhappy path tests
- [ ] Enable appropriate error handling tests
- [ ] Validate graceful degradation scenarios

### Phase 4: Quality Assurance and Performance (Week 4)
**Objective**: Production readiness validation

#### Week 4.1: Performance Validation
- [ ] Run performance benchmarks with new features
- [ ] Validate response time requirements (<50ms code actions)
- [ ] Memory usage analysis with expanded functionality

#### Week 4.2: Integration Testing
- [ ] End-to-end workflow testing
- [ ] Cross-platform compatibility validation
- [ ] Comprehensive regression testing

## Test Enablement Decision Framework

### Criteria for Test Enablement
1. **Functionality Exists**: Implementation is present and functional
2. **Feature Complete**: Implementation meets test expectations
3. **Protocol Compliant**: Behavior aligns with LSP specification
4. **Stable**: Test passes consistently across environments
5. **Valuable**: Test provides meaningful validation of important functionality

### Criteria for Keeping Tests Ignored
1. **Development Tool**: Test is a utility for development, not validation
2. **Optional Feature**: Feature is intentionally disabled by default
3. **Implementation Specific**: Test depends on undefined server-specific behavior
4. **Environment Dependent**: Test requires specific external dependencies
5. **Performance Tool**: Test is a benchmark, not a validation

### Test Status Documentation Requirements
For tests that remain ignored, update ignore reasons to be specific and actionable:

```rust
// Good: Specific, actionable reason
#[ignore = "Code lens disabled by default - enable in server config to test"]

// Bad: Vague, non-actionable reason
#[ignore = "doesn't work"]

// Good: Clear development tool distinction
#[ignore = "Development utility - run with --ignored to execute benchmarks"]
```

## Success Metrics and Validation

### Quantitative Metrics
- **Test Pass Rate**: Target 95% for enabled tests
- **Feature Coverage**: 100% of Issue #145 acceptance criteria validated
- **Performance**: All response time requirements met
- **Integration**: End-to-end workflows validated

### Qualitative Assessments
- **Protocol Compliance**: LSP 3.17+ specification adherence
- **Developer Experience**: Seamless editor integration
- **Error Handling**: Graceful degradation and recovery
- **Documentation**: Clear test purpose and expectations

### Validation Commands
```bash
# Core functionality validation
cargo test -p perl-lsp --test lsp_execute_command_tests
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_perlcritic

# Code actions validation
cargo test -p perl-lsp -- code_action

# Performance validation
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Comprehensive integration
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test
```

## Risk Management

### Implementation Risks
**Risk**: Enabling tests reveals significant implementation gaps
**Mitigation**: Systematic enablement with immediate issue resolution

**Risk**: Test enablement introduces instability
**Mitigation**: Comprehensive regression testing after each enablement

**Risk**: External tool dependencies cause test failures in CI
**Mitigation**: Graceful degradation testing for tool unavailability scenarios

### Schedule Risks
**Risk**: Test issues block Issue #145 resolution
**Mitigation**: Prioritize P0 tests for critical path completion

**Risk**: Scope creep from comprehensive test enablement
**Mitigation**: Clear phase boundaries with success criteria gates

## Conclusion

The ignored tests analysis reveals that Issue #145 can be resolved primarily through test enablement rather than major implementation work. The `perl.runCritic` functionality appears to be implemented but untested, and code actions need LSP protocol integration.

**Key Findings**:
1. **executeCommand**: Implementation exists, tests need enablement
2. **Code Actions**: Provider exists, LSP integration needed
3. **Protocol Support**: Comprehensive but selectively advertised

**Success Strategy**: Systematic phase-based enablement with clear success criteria and risk mitigation at each stage.

---

**Document Status**: COMPLETE
**Next Action**: Phase 1 implementation with P0 test enablement
**Quality Gate**: All enabled tests pass with comprehensive validation
**Integration Target**: Issue #145 acceptance criteria fulfillment