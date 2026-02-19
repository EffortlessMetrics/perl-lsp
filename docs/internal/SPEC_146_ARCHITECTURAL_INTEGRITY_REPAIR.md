# SPEC-146: Architectural Integrity Repair for Commented-Out Modules

**Issue**: #146 - Fix architectural integrity problems with commented-out modules (refactoring.rs and tdd_workflow.rs)

**Status**: `spec` ✅ **COMPLETE** - Technical specifications defined
**Priority**: High - Technical debt affecting parser stability
**Target**: v0.8.10 parser crate architectural clean-up

## Executive Summary

This specification addresses critical architectural integrity issues in the perl-parser crate where two modules (`refactoring.rs` and `tdd_workflow.rs`) are commented out in `lib.rs` due to compilation errors, creating technical debt and affecting parser stability.

## Problem Analysis

### Current State Assessment

1. **tdd_workflow.rs** (22KB):
   - **EXISTS** but has compilation errors
   - **COMMENTED OUT** in lib.rs line 221: `// pub mod tdd_workflow;    // TODO: Fix compilation`
   - Contains comprehensive TDD workflow integration for LSP

2. **refactoring.rs**:
   - **MISSING** but referenced in lib.rs line 141: `// pub mod refactoring; // TODO: Fix compilation errors`
   - Should contain refactoring functionality currently spread across other modules

### Identified Compilation Issues

#### tdd_workflow.rs Compilation Errors

**Primary Issue**: Undefined variable `signature` on line 171
```rust
// Line 171 in generate_basic_test method
let args = signature.as_ref().map(|s| vec![s]).unwrap_or_default().iter()
```
- Variable `signature` is not defined in the method scope
- Method parameter `params: &[String]` is available but not used

**Secondary Issues**:
1. **Missing Dependencies**:
   - Uses `tower_lsp::lsp_types` but `tower-lsp` is not in `Cargo.toml`
   - Only `lsp-types = "0.97.0"` is available in dependencies

2. **API Inconsistencies**:
   - Several method calls to `RefactoringSuggester` and `TestRunner` may have API mismatches
   - LSP integration module uses deprecated `tower_lsp` patterns

## Technical Specifications

### 1. tdd_workflow.rs Repair Strategy

#### 1.1 Fix Primary Compilation Error

**Location**: `/crates/perl-parser/src/tdd_workflow.rs:171`

**Current Code**:
```rust
fn generate_basic_test(&self, name: &str, params: &[String]) -> String {
    let args = signature.as_ref().map(|s| vec![s]).unwrap_or_default().iter()  // ❌ ERROR
        .enumerate()
        .map(|(i, _)| format!("'test_value_{}'", i))
        .collect::<Vec<_>>()
        .join(", ");
```

**Fix Implementation**:
```rust
fn generate_basic_test(&self, name: &str, params: &[String]) -> String {
    let args = params.iter()  // ✅ FIXED: Use params parameter instead of undefined signature
        .enumerate()
        .map(|(i, _)| format!("'test_value_{}'", i))
        .collect::<Vec<_>>()
        .join(", ");
```

#### 1.2 Fix LSP Dependencies

**Update LSP Integration Module** (lines 459-555):

**Replace**:
```rust
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Command, Diagnostic as LspDiagnostic,
    DiagnosticSeverity, MessageType, Position, Range, TextEdit, WorkspaceEdit,
};
```

**With**:
```rust
use lsp_types::{
    CodeAction, CodeActionKind, Command, Diagnostic as LspDiagnostic,
    DiagnosticSeverity, MessageType, Position, Range, TextEdit, WorkspaceEdit,
};
```

#### 1.3 API Contract Validation

**TestGenerator Integration**:
- Ensure compatibility with existing `crate::test_generator` API
- Validate `TestFramework`, `TestCase`, and `RefactoringSuggestion` types match

**LSP Integration**:
- Use `lsp_types` instead of `tower_lsp::lsp_types`
- Ensure Position and Range usage follows current LSP patterns

### 2. refactoring.rs Implementation Strategy

#### 2.1 Module Purpose and Scope

Create a centralized refactoring module that consolidates functionality currently spread across:
- `workspace_refactor.rs` (43KB) - Workspace-wide refactoring
- `modernize.rs` (7KB) - Code modernization
- `modernize_refactored.rs` (10KB) - Enhanced modernization
- `code_actions_enhanced.rs` - Refactoring actions

#### 2.2 refactoring.rs Module Structure

**Create**: `/crates/perl-parser/src/refactoring.rs`

```rust
//! Centralized refactoring operations for Perl code
//!
//! This module provides a unified interface for various refactoring operations,
//! consolidating functionality from workspace_refactor, modernize modules, and
//! enhanced code actions.

use crate::ast::Node;
use crate::error::ParseResult;
use crate::workspace_refactor::WorkspaceRefactor;
use crate::modernize::Modernizer;
use crate::code_actions_enhanced::EnhancedCodeActionsProvider;

/// Central refactoring coordinator
pub struct RefactoringEngine {
    workspace_refactor: WorkspaceRefactor,
    modernizer: Modernizer,
    enhanced_actions: EnhancedCodeActionsProvider,
}

/// Types of refactoring operations available
#[derive(Debug, Clone, PartialEq)]
pub enum RefactoringType {
    ExtractVariable,
    ExtractSubroutine,
    RenameSymbol,
    ModernizeCode,
    WorkspaceRename,
    OptimizeImports,
}

/// Result of a refactoring operation
#[derive(Debug, Clone)]
pub struct RefactoringResult {
    pub success: bool,
    pub changes: Vec<RefactoringChange>,
    pub warnings: Vec<String>,
    pub description: String,
}

/// Individual change as part of refactoring
#[derive(Debug, Clone)]
pub struct RefactoringChange {
    pub file_path: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub new_text: String,
}

impl RefactoringEngine {
    pub fn new() -> Self {
        Self {
            workspace_refactor: WorkspaceRefactor::new(),
            modernizer: Modernizer::new(),
            enhanced_actions: EnhancedCodeActionsProvider::new(),
        }
    }

    pub fn perform_refactoring(
        &self,
        refactoring_type: RefactoringType,
        ast: &Node,
        source: &str,
        range: (usize, usize),
    ) -> ParseResult<RefactoringResult> {
        // Implementation delegates to appropriate sub-modules
        todo!("Implement unified refactoring interface")
    }
}
```

### 3. API Contract Definitions

#### 3.1 Parser API Consistency Requirements

**Ensure Compatibility With**:
1. **AST Interface**: All refactoring operations must work with `crate::ast::Node`
2. **Error Handling**: Use `crate::error::ParseResult<T>` for all fallible operations
3. **LSP Integration**: Support `lsp_types` for LSP protocol compliance
4. **Position Mapping**: Use `crate::position` for UTF-8/UTF-16 conversions

#### 3.2 Module Export Requirements

**lib.rs Updates Required**:
```rust
// AFTER fixes are implemented:
pub mod refactoring;      // ✅ Uncomment after implementation
pub mod tdd_workflow;     // ✅ Uncomment after compilation fixes
```

**Public API Exports**:
```rust
// Add to lib.rs exports section
pub use refactoring::{RefactoringEngine, RefactoringType, RefactoringResult};
pub use tdd_workflow::{TddWorkflow, TddConfig, WorkflowState, TddCycleResult};
```

## Implementation Plan

### Phase 1: tdd_workflow.rs Compilation Repair
**Priority**: High - Immediate
**Estimate**: 2-4 hours

1. **Fix undefined `signature` variable** (line 171)
2. **Replace `tower_lsp` with `lsp_types`** imports
3. **Validate API compatibility** with existing modules
4. **Test compilation** with `cargo check -p perl-parser`

### Phase 2: refactoring.rs Module Creation
**Priority**: Medium - Within 1 week
**Estimate**: 1-2 days

1. **Create refactoring.rs** with unified interface
2. **Integrate existing refactoring modules** as dependencies
3. **Implement RefactoringEngine** coordinator
4. **Add comprehensive documentation**

### Phase 3: Integration and Testing
**Priority**: Medium - Post-implementation
**Estimate**: 1 day

1. **Uncomment modules** in lib.rs
2. **Add public exports** to lib.rs
3. **Run comprehensive test suite**
4. **Validate LSP integration**

## Quality Assurance

### Compilation Validation
```bash
# Must pass without errors
cargo check -p perl-parser
cargo clippy -p perl-parser
cargo test -p perl-parser
```

### API Consistency Tests
```bash
# Validate TDD workflow integration
cargo test -p perl-parser --test tdd_workflow_tests

# Validate refactoring functionality
cargo test -p perl-parser --test refactoring_tests
```

### LSP Integration Tests
```bash
# Ensure LSP features work with restored modules
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test
```

## Risk Assessment

### High Risk
- **API Breaking Changes**: Restored modules may require API updates
- **LSP Protocol Compatibility**: tower_lsp vs lsp_types mismatches

### Medium Risk
- **Performance Impact**: Additional modules may affect compilation time
- **Test Coverage**: Restored functionality needs comprehensive testing

### Mitigation Strategies
1. **Incremental Restoration**: Fix one module at a time
2. **Backward Compatibility**: Ensure existing APIs remain functional
3. **Comprehensive Testing**: Run full test suite before/after changes

## Success Criteria

### Primary Goals
- [ ] `tdd_workflow.rs` compiles without errors
- [ ] `refactoring.rs` implements unified refactoring interface
- [ ] Both modules are uncommented in `lib.rs`
- [ ] All existing tests continue to pass

### Secondary Goals
- [ ] Enhanced LSP refactoring capabilities
- [ ] Improved TDD workflow integration
- [ ] Consolidated refactoring API

## API Contract Validation

### 3.3 Validated API Contracts

#### tdd_workflow.rs API Compatibility

**TestGenerator API** ✅ **VALIDATED**:
```rust
// FROM: crate::test_generator
pub struct TestGenerator { /* framework: TestFramework */ }
impl TestGenerator {
    pub fn new(framework: TestFramework) -> Self;
    pub fn generate_tests(&self, ast: &Node, source: &str) -> Vec<TestCase>;
}
```

**TestRunner API** ✅ **VALIDATED**:
```rust
// FROM: crate::test_generator
pub struct TestRunner { /* test_command, watch_mode, coverage */ }
impl TestRunner {
    pub fn new() -> Self;
    pub fn run_tests(&self, test_files: &[String]) -> TestResults;
    pub fn get_coverage(&self) -> Option<CoverageReport>;
}
```

**RefactoringSuggester API** ✅ **VALIDATED**:
```rust
// FROM: crate::test_generator
pub struct RefactoringSuggester { /* suggestions: Vec<RefactoringSuggestion> */ }
impl RefactoringSuggester {
    pub fn new() -> Self;
    pub fn analyze(&mut self, ast: &Node, source: &str) -> Vec<RefactoringSuggestion>;
}

pub struct RefactoringSuggestion {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub category: RefactoringCategory,
    pub code_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefactoringCategory {
    DuplicateCode,
    ComplexMethod,
    LongMethod,
    TooManyParameters,
    DeadCode,
    Performance,
    Naming,
    Structure,
}
```

#### LSP Types Compatibility ✅ **VALIDATED**:
```rust
// AVAILABLE: lsp_types = "0.97.0" in Cargo.toml
use lsp_types::{
    CodeAction, CodeActionKind, Command,
    Diagnostic, DiagnosticSeverity,
    Position, Range, TextEdit, WorkspaceEdit,
};
```

### 3.4 Specific Compilation Fixes Required

#### Fix 1: tdd_workflow.rs Line 171 ✅ **SPECIFIED**
```rust
// CURRENT (BROKEN):
let args = signature.as_ref().map(|s| vec![s]).unwrap_or_default().iter()

// FIXED VERSION:
let args = params.iter()  // Use existing `params: &[String]` parameter
```

#### Fix 2: LSP Dependencies ✅ **SPECIFIED**
```rust
// CURRENT (BROKEN):
use tower_lsp::lsp_types::{...};

// FIXED VERSION:
use lsp_types::{...};  // Use existing lsp_types dependency
```

## Dependencies

### External Dependencies
- `lsp_types = "0.97.0"` (already available) ✅
- `serde` with `derive` feature (already available) ✅

### Internal Dependencies ✅ **VALIDATED**
- `crate::ast` - AST node definitions ✅
- `crate::workspace_refactor` - Workspace refactoring operations ✅
- `crate::modernize` - Code modernization ✅
- `crate::test_generator` - Test generation framework ✅

## Documentation Updates

### Required Documentation
1. **API Documentation**: Add comprehensive docs for restored modules
2. **Integration Guide**: Update LSP implementation guide
3. **Migration Notes**: Document any API changes

### Documentation Locations
- `/docs/TDD_WORKFLOW_GUIDE.md` - TDD integration guide
- `/docs/WORKSPACE_REFACTORING_GUIDE.md` - Refactoring operations guide (already exists)
- Update `/docs/LSP_IMPLEMENTATION_GUIDE.md` with restored functionality

---

**Specification Completed**: ✅
**Next Phase**: Issue Finalizer - Route to implementation agents
**Gate Status**: `spec` → Ready for implementation