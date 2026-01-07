# Issue: Cross-File Scenarios for Perl Corpus

**Status**: Open  
**Priority**: P1  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks comprehensive multi-file workspace interaction testing. While the parser handles individual Perl files well, there is limited testing of how the parser handles cross-file scenarios such as:

1. **Module imports and exports** - `use`, `require`, `package` declarations across files
2. **Symbol resolution across files** - Finding definitions and references across modules
3. **Workspace indexing** - Building and maintaining workspace symbol indexes
4. **Incremental parsing across files** - How edits in one file affect other files
5. **Cross-file refactoring** - Renaming symbols across multiple files
6. **Package namespace resolution** - Resolving `Package::symbol` references
7. **Import optimization** - Managing imports across a workspace

Without cross-file tests, we cannot ensure:
- LSP provides accurate cross-file navigation
- Workspace indexing is correct and complete
- Cross-file refactoring works correctly
- Incremental parsing handles cross-file dependencies
- Symbol resolution works across package boundaries

## Impact Assessment

**Why This Matters:**

1. **LSP Navigation** - Go-to-definition and find-references must work across files
2. **Workspace Indexing** - Accurate symbol indexing is critical for LSP features
3. **Refactoring** - Cross-file symbol renaming must be safe and complete
4. **Real-World Code** - Real Perl projects span multiple files and modules
5. **Incremental Parsing** - Edits in one file can affect parsing of dependent files
6. **Code Quality** - Accurate cross-file analysis improves developer experience

**Current State:**
- Limited multi-file test scenarios in corpus
- No dedicated cross-file test infrastructure
- LSP cross-file features exist but lack comprehensive tests
- No tests for workspace indexing accuracy
- No tests for cross-file refactoring safety

## Current State

**What's Missing:**

1. **Multi-file test scenarios** - Tests with multiple interacting Perl files
2. **Cross-file symbol resolution tests** - Tests for finding definitions/references across files
3. **Workspace indexing tests** - Tests validating workspace symbol indexes
4. **Incremental cross-file tests** - Tests for incremental parsing with cross-file dependencies
5. **Cross-file refactoring tests** - Tests for safe symbol renaming across files
6. **Import management tests** - Tests for import optimization across workspace
7. **Package namespace tests** - Tests for `Package::symbol` resolution
8. **LSP cross-file tests** - Tests for LSP navigation across files

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) has single-file tests
- [`crates/perl-corpus/src/lib.rs`](../crates/perl-corpus/src/lib.rs) has `parse_dir()` for multi-file parsing
- [`crates/perl-parser/src/`](../crates/perl-parser/src/) has workspace indexing
- LSP cross-file navigation exists but lacks comprehensive tests

## Recommended Path Forward

### Phase 1: Design Multi-File Test Scenarios

**Objective**: Identify cross-file scenarios to test

**Steps:**
1. Analyze common Perl multi-file patterns:
   - Module imports (`use Module;`)
   - Package declarations (`package Package;`)
   - Exported symbols (`@EXPORT`, `@EXPORT_OK`)
   - Inheritance (`@ISA`, `use base`, `use parent`)
   - Role composition (`use Role;`)
   - Namespace resolution (`Package::subroutine()`)
2. Design test scenarios for each pattern:
   - Simple two-file scenarios
   - Multi-module workspace scenarios
   - Complex dependency graphs
   - Circular dependencies
3. Document expected behavior for each scenario:
   - Which symbols should be visible where?
   - How should imports be resolved?
   - What should workspace index contain?
   - How should incremental parsing behave?
4. Create test file structure for multi-file scenarios

**Deliverable**: `docs/cross_file_scenarios.md`

### Phase 2: Create Multi-File Test Infrastructure

**Objective**: Implement multi-file test infrastructure

**Steps:**
1. Create `test_corpus/cross_file/` directory
2. Implement test scenario structure:
   ```
   test_corpus/cross_file/
   ├── simple_import/
   │   ├── main.pl
   │   └── MyModule.pm
   ├── multi_module/
   │   ├── main.pl
   │   ├── Core.pm
   │   ├── Utils.pm
   │   └── Helpers.pm
   ├── namespace_resolution/
   │   ├── main.pl
   │   ├── PackageA.pm
   │   └── PackageB.pm
   └── circular_dependency/
       ├── ModuleA.pm
       └── ModuleB.pm
   ```
3. Extend corpus parser to handle multi-file scenarios:
   - Add workspace context to parsing
   - Track imports and exports
   - Maintain symbol index across files
4. Add metadata tags for cross-file scenarios:
   - `# @tags: cross_file, import, export`
   - `# @workspace: simple_import`
5. Validate multi-file parsing works correctly

**Deliverable**: Multi-file test infrastructure

### Phase 3: Add Cross-File Symbol Resolution Tests

**Objective**: Test symbol resolution across files

**Steps:**
1. Implement cross-file symbol resolution tests:
   ```perl
   # Cross-file symbol resolution test
   # @tags: cross_file, symbol_resolution, definition
   # @workspace: simple_import
   
   # File: MyModule.pm
   package MyModule;
   use v5.36;
   
   our @EXPORT = qw(helper_function);
   
   sub helper_function {
       return "Hello from MyModule";
   }
   
   # File: main.pl
   use v5.36;
   use MyModule;
   
   my $result = helper_function();  # Should resolve to MyModule::helper_function
   ```
2. Validate:
   - Go-to-definition works across files
   - Find-references finds all uses across files
   - Package-qualified symbols resolve correctly
   - Exported symbols are visible
3. Test edge cases:
   - Multiple definitions with same name
   - Shadowed symbols
   - Namespace conflicts
4. Document symbol resolution behavior

**Deliverable**: Cross-file symbol resolution test suite

### Phase 4: Add Workspace Indexing Tests

**Objective**: Test workspace symbol indexing

**Steps:**
1. Implement workspace indexing tests:
   - Test workspace index contains all symbols
   - Test index is correctly updated on file changes
   - Test index handles imports/exports correctly
   - Test index handles package namespaces correctly
2. Validate index accuracy:
   - All symbols from all files are indexed
   - Symbol locations are correct
   - Symbol types are correct
   - Cross-file references are correct
3. Test incremental index updates:
   - Adding a new file updates index
   - Editing a file updates relevant index entries
   - Deleting a file removes index entries
4. Document index behavior

**Deliverable**: Workspace indexing test suite

### Phase 5: Add Cross-File Refactoring Tests

**Objective**: Test cross-file refactoring safety

**Steps:**
1. Implement cross-file refactoring tests:
   - Test symbol renaming across files
   - Test extract subroutine across files
   - Test import optimization across files
   - Test module extraction
2. Validate refactoring safety:
   - All references are updated
   - No false positives
   - No broken imports
   - No namespace conflicts
3. Test refactoring edge cases:
   - Multiple definitions with same name
   - Shadowed symbols
   - Circular dependencies
4. Document refactoring behavior

**Deliverable**: Cross-file refactoring test suite

### Phase 6: Add LSP Cross-File Tests

**Objective**: Test LSP cross-file navigation

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with cross-file tests
2. Implement LSP cross-file tests:
   - `test_lsp_cross_file_definition()` - Go-to-definition across files
   - `test_lsp_cross_file_references()` - Find-references across files
   - `test_lsp_cross_file_rename()` - Symbol rename across files
   - `test_lsp_workspace_symbols()` - Workspace symbol list
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - Correct symbol locations returned
   - All references found across files
   - Rename updates all files
   - Workspace symbols are complete

**Deliverable**: LSP cross-file test suite

## Priority Level

**P1 - High Priority**

This is a P1 issue because:
1. **LSP Core Feature** - Cross-file navigation is critical for LSP
2. **Real-World Usage** - Real Perl projects span multiple files
3. **Refactoring Safety** - Cross-file refactoring must be safe
4. **User Experience** - Accurate cross-file navigation improves developer experience
5. **Production Readiness** - Cannot ship production LSP without cross-file tests
6. **Foundation** - Enables testing of other cross-file features

## Estimated Effort

**Total Effort**: High

- Phase 1 (Design Scenarios): 2-3 days
- Phase 2 (Multi-File Infrastructure): 4-5 days
- Phase 3 (Symbol Resolution Tests): 5-7 days
- Phase 4 (Workspace Indexing Tests): 4-5 days
- Phase 5 (Cross-File Refactoring Tests): 4-5 days
- Phase 6 (LSP Cross-File Tests): 3-4 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Test Coverage Metrics](corpus-coverage-test-metrics.md) - Related coverage measurement

## References

- [`crates/perl-corpus/src/lib.rs`](../crates/perl-corpus/src/lib.rs) - Corpus parsing infrastructure
- [`crates/perl-parser/src/`](../crates/perl-parser/src/) - Workspace indexing
- [Workspace Navigation Guide](../docs/WORKSPACE_NAVIGATION_GUIDE.md) - Cross-file navigation
- [Import Optimizer Guide](../docs/IMPORT_OPTIMIZER_GUIDE.md) - Import management

## Success Criteria

1. Multi-file test scenarios documented and categorized
2. Multi-file test infrastructure implemented
3. Cross-file symbol resolution tests implemented and passing
4. Workspace indexing tests implemented and passing
5. Cross-file refactoring tests implemented and passing
6. LSP cross-file tests implemented and passing
7. Go-to-definition works correctly across files
8. Find-references finds all uses across files
9. Workspace index is accurate and complete
10. Cross-file refactoring is safe and complete

## Open Questions

1. Which multi-file patterns are highest priority?
2. How should circular dependencies be handled?
3. What should happen when multiple files define the same symbol?
4. Should there be version-specific cross-file tests (e.g., Perl 5.36+ signatures)?
5. How should workspace indexing handle large workspaces?
