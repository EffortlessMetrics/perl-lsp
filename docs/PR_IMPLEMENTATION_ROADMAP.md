# PR Implementation Roadmap

## High-Priority Implementation PRs

After closing 7 duplicate PRs, focus on these high-value implementations that address partially-completed functionality:

### 🥇 Priority 1: Import Optimizer (#12)
**Estimated Effort**: 2-3 days  
**Business Value**: High - Enables LSP import organization  
**Technical Risk**: Low - Architecture complete

#### Current State
- ✅ Complete type definitions (`ImportAnalysis`, `UnusedImport`, etc.)
- ✅ Integration with code actions and workspace refactor  
- ❌ Core analysis logic returns stub results

#### Implementation Tasks
1. **Parse import statements** 
   - Detect `use Module::Name qw(symbols)` patterns
   - Detect `require Module::Name` statements
   - Extract module names and imported symbols

2. **Symbol usage analysis**
   - Track variable/function references throughout file
   - Match references to imported symbols
   - Identify unused imports

3. **Duplicate detection**
   - Find multiple imports of same module
   - Consolidate import statements

#### Success Criteria
- `analyze_file()` returns real analysis results
- `generate_optimized_imports()` produces consolidated imports  
- Integration tests pass with real Perl code

---

### 🥈 Priority 2: Workspace Refactor Utilities (#40)  
**Estimated Effort**: 1-2 weeks
**Business Value**: High - Enables cross-file refactoring
**Technical Risk**: Medium - Complex cross-file operations

#### Current State
- ✅ Complete API structure in `workspace_refactor.rs`
- ✅ Integration with LSP code actions
- ❌ Stub implementations for core operations

#### Implementation Tasks
1. **Cross-file reference finding**
   - Use `WorkspaceIndex` to find symbol references
   - Handle package-qualified names (`Package::symbol`)
   - Support variable, function, and package references

2. **Extract variable refactoring**
   - Identify expression to extract
   - Generate variable declaration
   - Replace expression with variable reference

3. **Rename symbol refactoring**  
   - Find all references to symbol across workspace
   - Generate text edits for renaming
   - Handle scope and package context

#### Success Criteria
- `find_all_references()` returns real cross-file results
- `extract_variable()` generates working refactorings
- Integration with LSP rename and code actions

---

### 🥉 Priority 3: Workspace Refactor Tools (#7)
**Estimated Effort**: 1-2 weeks  
**Business Value**: High - Complements #40
**Technical Risk**: Medium - Parser operation complexity

#### Current State
- ✅ Test infrastructure and integration points
- ❌ Parser operations return placeholder results

#### Implementation Tasks
1. **AST-based code analysis**
   - Parse and analyze code structures
   - Identify refactoring opportunities
   - Generate transformation plans

2. **Text edit generation**
   - Convert AST transformations to LSP text edits
   - Handle whitespace and formatting preservation
   - Ensure valid Perl syntax after edits

#### Success Criteria
- Parser operations return real analysis results
- Text edits produce syntactically valid code
- Integration tests pass with complex refactoring scenarios

---

## Enhancement PRs for Future Consideration

### Testing & Quality Improvements
- **#51** - Test assertion improvements (Low effort, good quality impact)
- **#44** - Hashed password tests (Security best practice, trivial effort)  
- **#29** - Semantic token testing (Medium effort, focused LSP validation)

### Tooling & Performance  
- **#31** - C benchmark utilities (Medium effort, performance tooling enhancement)
- **#30** - Benchmark save functionality (Low effort, trend analysis capability)

### Feature Enhancements (Need Evaluation)
- **#53** - Position tracking (May duplicate existing `LineStartsCache`)
- **#5** - Workspace symbols container info (May enhance existing implementation)
- **#2** - Package-aware completions (May enhance existing completion system)

## Implementation Guidelines

### Code Quality Standards
- Follow existing patterns in the codebase
- Use defensive security practices (no secrets in code)
- Maintain microsecond-level performance targets
- Add comprehensive test coverage

### Integration Points
- **Import Optimizer**: Integrates with `CodeActionsProvider` and `workspace_refactor.rs`
- **Workspace Refactor**: Uses `WorkspaceIndex` for cross-file operations  
- **Testing**: Use existing `tests/support/` infrastructure for consistency

### Success Metrics
- All advertised LSP features remain 100% functional
- Performance benchmarks show no regression
- Integration tests demonstrate real-world usage
- Documentation updated with Diataxis framework

## Timeline Estimate

**Phase 1** (Week 1): Close duplicate PRs, implement Import Optimizer (#12)  
**Phase 2** (Weeks 2-3): Implement Workspace Refactor Utilities (#40)
**Phase 3** (Weeks 4-5): Implement Workspace Refactor Tools (#7)  
**Phase 4** (Week 6): Enhancement PR evaluation and selective implementation

**Total estimated effort**: 6 weeks for complete high-priority implementation
**Expected outcome**: Fully-featured workspace refactoring and import optimization in LSP server