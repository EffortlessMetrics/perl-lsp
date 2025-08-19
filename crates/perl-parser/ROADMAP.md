# Perl LSP Implementation Roadmap

## Current State (v0.8.3-rc.1)
**Parser Infrastructure: ~80% Complete**
**LSP Wiring: ~30% Complete**
**Overall Functionality: ~35% Available to Users**

**UPDATE**: Investigation reveals the parser has ~70% of required infrastructure built but not wired to LSP. See `LSP_WIRING_OPPORTUNITIES.md` for details. This roadmap has been updated to reflect that much work is wiring, not reimplementation.

## Immediate Priorities (Q1 2025)

### 🔴 Critical: Quick Wins by Wiring Existing Infrastructure
- [ ] **Enable workspace feature by default** (1 line change!)
  - [ ] Add `workspace` to default features in Cargo.toml
  - [ ] This immediately enables cross-file navigation
- [ ] **Wire existing WorkspaceIndex to LSP** (~200 lines)
  - [ ] Connect `find_definition()` to LSP handler
  - [ ] Connect `find_references()` to LSP handler
  - [ ] Enable workspace symbol search
- [ ] **Complete existing refactoring actions** (~300 lines)
  - [ ] Make `extract_variable` return real edits
  - [ ] Make `extract_subroutine` return real edits
  - [ ] Wire to LSP code actions

### 🟡 High Priority: Complete Partial Implementations
- [ ] **Semantic Analyzer Integration** (Exists, needs wiring)
  - [ ] Wire type inference to hover
  - [ ] Connect to diagnostics for unused variables
  - [ ] Expose symbol information to completion

- [ ] **Module Resolution** (Basic version exists)
  - [ ] Extend existing `resolve_module_to_path()`
  - [ ] Add local::lib support
  - [ ] Cache module locations

- [ ] **Code Completion** (Infrastructure exists)
  - [ ] Wire SymbolIndex to completion provider
  - [ ] Use semantic analyzer for context
  - [ ] Add package member completion using existing data

## Phase 1: Foundation (Q2 2025)

### Workspace Infrastructure
```rust
// Currently returns empty index
workspace_index.rs -> Implement real indexing
- [ ] Parse all .pl/.pm files on initialization
- [ ] Build symbol dependency graph
- [ ] Track module locations
- [ ] Cache for performance
```

### Module Resolution
```rust
// Currently incomplete
- [ ] Implement @INC path resolution
- [ ] Support local::lib
- [ ] Handle relative imports
- [ ] Cache module locations
```

### Real Incremental Parsing
```rust
// Currently does full reparse
incremental_v2.rs -> Implement tree reuse
- [ ] Implement tree-sitter style incremental updates
- [ ] Track changed regions
- [ ] Reuse unchanged subtrees
- [ ] Benchmark improvements
```

## Phase 2: Complete Stubs (Q3 2025)

### Workspace Refactoring
Transform `workspace_refactor.rs` from stub to working:
- [ ] **rename_symbol**
  - [ ] Find all occurrences across workspace
  - [ ] Handle package-qualified names
  - [ ] Update import statements
  - [ ] Preview changes

- [ ] **extract_module**
  - [ ] Analyze dependencies
  - [ ] Generate new module file
  - [ ] Update imports in original
  - [ ] Handle exports

- [ ] **move_subroutine**
  - [ ] Detect dependencies
  - [ ] Update callers
  - [ ] Fix imports/exports
  - [ ] Handle method calls

### Import Optimization
Transform `import_optimizer.rs` from stub to working:
- [ ] **Unused import detection**
  - [ ] Track symbol usage
  - [ ] Identify unused imports
  - [ ] Handle implicit imports

- [ ] **Missing import detection**
  - [ ] Identify undefined symbols
  - [ ] Suggest modules to import
  - [ ] Auto-import on completion

- [ ] **Import organization**
  - [ ] Sort imports
  - [ ] Group by type
  - [ ] Remove duplicates
  - [ ] Optimize use statements

### Dead Code Detection
Transform `dead_code_detector.rs` from stub to working:
- [ ] **Unused subroutines**
  - [ ] Build call graph
  - [ ] Find unreachable subs
  - [ ] Handle dynamic calls

- [ ] **Unused variables**
  - [ ] Track variable usage
  - [ ] Handle closures
  - [ ] Consider string interpolation

- [ ] **Unreachable code**
  - [ ] Control flow analysis
  - [ ] Detect dead branches
  - [ ] Handle constant conditions

## Phase 3: Advanced Features (Q4 2025)

### Debug Adapter Protocol
Transform `debug_adapter.rs` from stub to working:
- [ ] Integrate with Perl debugger
- [ ] Breakpoint management
- [ ] Step/continue/pause
- [ ] Variable inspection
- [ ] Call stack navigation
- [ ] Watch expressions
- [ ] REPL integration

### Type System
- [ ] Infer types from usage
- [ ] Track type constraints
- [ ] Support type annotations (comments)
- [ ] Provide type-based completions
- [ ] Type mismatch warnings

### Advanced Analysis
- [ ] Data flow analysis
- [ ] Taint tracking
- [ ] Security vulnerability detection
- [ ] Performance suggestions
- [ ] Complexity metrics

## Phase 4: Production Ready (2026)

### Performance Optimization
- [ ] Lazy parsing for large files
- [ ] Parallel workspace indexing  
- [ ] Incremental analysis
- [ ] Memory optimization
- [ ] Cache optimization

### Enterprise Features
- [ ] Multi-root workspace support
- [ ] Remote development support
- [ ] Custom perl interpreter support
- [ ] Perl version compatibility checking
- [ ] Integration with build systems

### Polish
- [ ] Comprehensive documentation
- [ ] Video tutorials
- [ ] Example configurations
- [ ] Migration guides
- [ ] Performance profiling

## Version Milestones

### v0.4.0 - Honest Reset
- Renumber to reflect actual functionality
- Mark all stubs clearly
- Update all documentation

### v0.5.0 - Workspace Basics
- Real workspace indexing
- Cross-file navigation
- Module resolution

### v0.6.0 - Complete Core
- All completion features
- Full navigation support
- Real incremental parsing

### v0.7.0 - Refactoring Works
- Workspace refactoring functional
- Import optimization functional
- Dead code detection functional

### v0.8.0 - Advanced Features
- Debug adapter functional
- Type inference working
- Advanced analysis

### v0.9.0 - Beta
- All features implemented
- Performance optimized
- Comprehensive testing

### v1.0.0 - Production
- Stable API
- Full documentation
- Enterprise ready
- Performance guarantees

## Success Metrics

### Phase 1 Complete When:
- [ ] Can navigate between files in a real Perl project
- [ ] Completion includes package members
- [ ] Workspace symbols actually searches all files
- [ ] Tests verify functionality, not just shape

### Phase 2 Complete When:
- [ ] Can refactor across multiple files
- [ ] Import optimization produces real changes
- [ ] Dead code detection finds actual dead code
- [ ] All stubs replaced with implementations

### Phase 3 Complete When:
- [ ] Can debug Perl scripts through LSP
- [ ] Type inference provides meaningful insights
- [ ] Advanced analysis catches real issues
- [ ] Performance meets targets on large codebases

### Phase 4 Complete When:
- [ ] Used in production by multiple organizations
- [ ] Performance competitive with other LSPs
- [ ] Full feature parity with top-tier language servers
- [ ] Community actively contributing

## How to Contribute

### For New Contributors:
1. Pick a stub function (marked "stub implementation")
2. Write tests for expected behavior FIRST
3. Implement incrementally
4. Update status in this roadmap

### For Experienced Contributors:
1. Tackle workspace indexing (highest impact)
2. Design incremental parsing system
3. Implement cross-file analysis
4. Create benchmarking suite

### For Organizations:
1. Sponsor specific feature development
2. Provide real-world test cases
3. Contribute enterprise requirements
4. Fund full-time development

## Realistic Timeline

Given current development pace and assuming part-time contribution:
- **2025 Q1-Q2**: Foundation and critical fixes
- **2025 Q3-Q4**: Complete major stubs
- **2026 Q1-Q2**: Advanced features
- **2026 Q3-Q4**: Production readiness

With full-time development, timeline could be halved.

---

*This roadmap reflects the actual state of the codebase as of 2025-01-19. It provides a realistic path from the current ~35% implementation to a production-ready language server.*