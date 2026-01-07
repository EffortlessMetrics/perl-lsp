# Issue: Integration Testing for Perl Corpus

**Status**: Open  
**Priority**: P1  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks integration tests that combine multiple Perl features in realistic, complex scenarios. While individual syntax features are well-tested (regex, source filters, XS/FFI, modern features, etc.), there is limited evidence of tests that verify these features work correctly when combined.

Without integration tests, we cannot ensure:
1. **Feature interaction correctness** - Features work correctly when used together
2. **Real-world scenario coverage** - Complex, realistic code patterns are tested
3. **Cross-feature validation** - No unexpected interactions between different features
4. **End-to-end workflows** - Complete user workflows from start to finish
5. **Edge case discovery** - Complex interactions that break parsing

## Impact Assessment

**Why This Matters:**

1. **Production Confidence**: Real-world Perl code combines multiple features; isolated tests don't catch interaction bugs
2. **Regression Prevention**: Changes to one feature can break interactions with others
3. **User Experience**: LSP users work with complex files, not isolated syntax samples
4. **Bug Discovery**: Many bugs only manifest when features are combined
5. **Documentation Quality**: Integration tests serve as usage examples

**Current State:**
- Each test file focuses on a specific feature area (regex, XS, modern features, etc.)
- No tests combine features from multiple areas
- No multi-file workspace scenarios
- No end-to-end workflow tests
- LSP tests are isolated, not integration-focused

## Current State

**What's Missing:**

1. **Multi-feature test files** - Tests combining 2+ major feature areas
2. **Workspace scenarios** - Multi-file projects with interdependent modules
3. **End-to-end workflows** - Tests covering complete user workflows
4. **Complex interaction tests** - Tests for feature combinations that may conflict
5. **Real-world patterns** - Tests based on actual CPAN module patterns
6. **LSP integration tests** - Tests validating LSP behavior on complex files
7. **Incremental parsing tests** - Tests validating parser behavior on incremental edits

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) contains feature-specific test files
- [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) has LSP integration tests
- No corpus-level integration test infrastructure
- No multi-file test workspace

## Recommended Path Forward

### Phase 1: Design Integration Test Scenarios

**Objective**: Identify realistic multi-feature scenarios to test

**Steps:**
1. Analyze common CPAN module patterns
2. Identify feature combinations that are commonly used together:
   - Modern features + regex + XS
   - Source filters + OO + pragmas
   - Try/catch + signatures + class/field
   - Legacy syntax + modern features (migration scenarios)
3. Document 10-15 high-priority integration scenarios
4. Define success criteria for each scenario
5. Map scenarios to existing corpus features

**Deliverable**: `docs/integration_test_scenarios.md`

### Phase 2: Create Integration Test Files

**Objective**: Implement integration test files for identified scenarios

**Steps:**
1. Create `test_corpus/integration/` directory
2. For each scenario, create a test file combining features:
   - Use existing test files as building blocks
   - Add comments explaining the integration points
   - Include metadata tags for all combined features
   - Add expected behavior documentation
3. Example scenarios to implement:
   ```perl
   # Integration: Modern features + XS + regex
   use v5.36;
   use strict;
   use warnings;
   
   package MyApp {
       use Moo;
       use XS::Module;
       
       sub process_data {
           my ($input) = @_;
           # Modern signature
           my ($result) = XS::Module::complex_function($input);
           
           # Advanced regex with code assertions
           $result =~ s/(?{ uc($1) })/replacement/;
           
           return $result;
       }
   }
   ```
4. Validate each test file parses correctly
5. Ensure LSP provides correct diagnostics and symbols

**Deliverable**: 10-15 integration test files in [`test_corpus/integration/`](../test_corpus/integration/)

### Phase 3: Create Multi-File Workspace Tests

**Objective**: Test cross-file interactions and workspace-level features

**Steps:**
1. Create `test_corpus/workspace/` directory
2. Design multi-file project structures:
   - Main module importing multiple sub-modules
   - Versioned packages
   - Cross-package symbol references
   - Use/require statements
3. Implement test infrastructure for workspace parsing:
   - Parse all files in workspace
   - Validate cross-file references resolve correctly
   - Test dual indexing (qualified and bare names)
   - Verify LSP workspace navigation
4. Add incremental parsing tests:
   - Edit one file, verify other files unaffected
   - Add symbol in one file, verify references update
   - Delete symbol, verify references invalidated
5. Document workspace test expectations

**Deliverable**: Workspace test infrastructure and 5-10 test workspaces

### Phase 4: Add LSP Integration Tests

**Objective**: Validate LSP behavior on complex integration scenarios

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with integration test suite
2. Implement tests for each integration scenario:
   - `test_lsp_integration_modern_xs_regex()` - Test LSP on combined features
   - `test_lsp_workspace_cross_file_navigation()` - Test workspace navigation
   - `test_lsp_incremental_edit_integration()` - Test incremental parsing
   - `test_lsp_diagnostics_integration()` - Test diagnostic accuracy
   - `test_lsp_symbols_integration()` - Test symbol extraction
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - No false positives in diagnostics
   - Correct symbol locations
   - Proper folding ranges
   - Accurate completion suggestions
5. Add performance benchmarks for integration scenarios

**Deliverable**: LSP integration test suite in [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/)

### Phase 5: Implement End-to-End Workflow Tests

**Objective**: Test complete user workflows from start to finish

**Steps:**
1. Define common user workflows:
   - Create new module with modern features
   - Add XS functionality to existing module
   - Refactor code using regex and source filters
   - Debug complex multi-file project
2. For each workflow, create test:
   - Start with empty workspace
   - Perform series of edits (simulating user actions)
   - Validate parser state after each edit
   - Verify LSP responses are correct
3. Add workflow test automation:
   - Scripted edit sequences
   - Expected state validation
   - Performance measurement
4. Document workflow test results

**Deliverable**: End-to-end workflow test suite

## Priority Level

**P1 - High Priority**

This is a P1 issue because:
1. Critical for production readiness - real code uses multiple features together
2. High risk area - interaction bugs are common and hard to detect
3. LSP relevance - LSP users work with complex, multi-file projects
4. Foundation for other improvements - enables better testing of all features
5. User-facing impact - affects real-world usage patterns

## Estimated Effort

**Total Effort**: Medium-High

- Phase 1 (Scenario Design): 2-3 days
- Phase 2 (Integration Tests): 5-7 days
- Phase 3 (Workspace Tests): 4-6 days
- Phase 4 (LSP Integration): 3-5 days
- Phase 5 (Workflow Tests): 4-6 days

## Related Issues

- [Test Coverage Metrics](corpus-coverage-test-metrics.md) - Provides coverage data to guide integration test priorities
- [Cross-File Scenarios](corpus-coverage-cross-file-scenarios.md) - Related workspace testing focus

## References

- [`test_corpus/README.md`](../test_corpus/README.md) - Current corpus structure
- [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) - Existing LSP test infrastructure
- [CPAN Top 100 Modules](https://www.cpan.org/) - Source of real-world usage patterns
- [Modern Perl Book](https://modernperlbooks.com/books/modern_perl/) - Modern Perl usage patterns

## Success Criteria

1. Integration test scenarios documented and prioritized
2. 10-15 integration test files created and validated
3. Workspace test infrastructure implemented
4. LSP integration tests passing for all scenarios
5. End-to-end workflow tests covering common user actions
6. No interaction bugs discovered in existing features
7. Performance benchmarks established for integration scenarios
8. Documentation updated with integration test examples

## Open Questions

1. Which feature combinations are highest priority for testing?
2. Should integration tests be added to the main corpus or a separate directory?
3. How many integration scenarios are sufficient for production readiness?
4. Should integration tests be version-specific (e.g., Perl 5.36 only)?
5. What performance thresholds are acceptable for integration scenarios?
