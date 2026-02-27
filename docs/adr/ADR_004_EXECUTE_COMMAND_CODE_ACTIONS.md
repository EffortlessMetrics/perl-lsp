# ADR-004: Execute Command and Code Actions Architecture

**Status**: DRAFT
**Date**: 2025-01-15
**Issue**: #145 - Critical LSP features have ignored tests - executeCommand and code actions missing
**Supersedes**: None
**Related ADRs**: [ADR-001: Agent Architecture](ADR_001_AGENT_ARCHITECTURE.md), [ADR-002: API Documentation Infrastructure](ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md)

## Context and Problem Statement

The Perl LSP server has partial implementation of critical LSP features, with several tests ignored due to incomplete functionality. Specifically:

1. **executeCommand Method**: `perl.runCritic` command exists but has ignored tests
2. **Code Actions**: `EnhancedCodeActionsProvider` implemented but not wired to LSP protocol
3. **Integration Gaps**: Advanced refactoring features available but not exposed through LSP interface

**Business Impact**: Developers cannot access sophisticated code quality and refactoring tools directly through their LSP-compatible editors, reducing productivity and code quality maintenance efficiency.

## Decision Drivers

### Technical Requirements
- **LSP 3.17+ Compliance**: Full protocol specification adherence
- **Performance**: <50ms code action responses, <2s executeCommand execution
- **Reliability**: 95% test pass rate with comprehensive coverage
- **Integration**: Seamless workflow with existing Perl LSP infrastructure

### Architectural Constraints
- **Parser Integration**: Leverage existing incremental parsing (70-99% node reuse)
- **Dual Indexing**: Cross-file refactoring with workspace-aware navigation
- **Backward Compatibility**: Preserve existing executeCommand functionality
- **Tool Dependencies**: Graceful degradation when external tools unavailable

### Quality Requirements
- **Test-Driven Development**: All features validated with TDD approach
- **Documentation Standards**: Diátaxis framework compliance
- **Performance Monitoring**: Benchmark integration with regression prevention
- **Error Handling**: Robust error recovery with user-friendly messaging

## Considered Options

### Option 1: Minimal Integration (Rejected)
**Approach**: Enable only `perl.runCritic` without code actions integration
**Pros**: Quick implementation, minimal risk
**Cons**: Incomplete feature set, limited developer productivity gains

### Option 2: Complete Rewrite (Rejected)
**Approach**: Rebuild executeCommand and code actions from scratch
**Pros**: Clean architecture, optimal performance
**Cons**: High risk, extensive testing required, breaks existing functionality

### Option 3: Incremental Enhancement (Selected)
**Approach**: Enhance existing implementations with proper LSP integration
**Pros**: Leverages existing code, lower risk, maintains backward compatibility
**Cons**: Some architectural compromises, requires careful integration testing

## Decision

**We will implement Option 3: Incremental Enhancement** with the following architectural approach:

### Core Architecture Decisions

#### 1. LSP Protocol Integration Strategy
```rust
// Central executeCommand dispatcher in lsp_server.rs
impl LspServer {
    fn handle_execute_command(&mut self, params: ExecuteCommandParams) -> Result<Option<Value>, JsonRpcError> {
        let command_provider = ExecuteCommandProvider::new();
        match params.command.as_str() {
            "perl.runCritic" => self.run_perl_critic(&params.arguments),
            // ... existing commands
        }
    }

    fn handle_code_action(&mut self, params: CodeActionParams) -> Result<Vec<CodeActionOrCommand>, JsonRpcError> {
        let enhanced_provider = EnhancedCodeActionsProvider::new(document.content);
        enhanced_provider.get_code_actions(params.text_document.uri, params.range, params.context)
    }
}
```

**Rationale**: Centralized dispatching maintains consistency with existing architecture while enabling comprehensive feature integration.

#### 2. Parser Integration Architecture
```rust
// AST-aware refactoring with incremental parsing integration
impl EnhancedCodeActionsProvider {
    fn analyze_with_incremental_parsing(&self, uri: &str, range: Range) -> Vec<CodeAction> {
        if let Some(incremental_doc) = self.incremental_docs.get(uri) {
            // Leverage existing 70-99% node reuse efficiency
            return self.analyze_cached_nodes(incremental_doc, range);
        }
        self.analyze_full_document(uri, range)
    }
}
```

**Rationale**: Reuses existing incremental parsing infrastructure to maintain <1ms update performance while enabling sophisticated refactoring analysis.

#### 3. Dual Indexing Integration for Cross-file Refactoring
```rust
// Extract subroutine with workspace awareness
impl RefactoringOperations {
    fn extract_subroutine_with_indexing(&self, node: &Node) -> CodeAction {
        let qualified_name = format!("{}::{}", self.current_package, subroutine_name);

        // Index under both qualified and bare forms (established pattern)
        self.index_manager.add_symbol(&qualified_name, symbol_info.clone());
        self.index_manager.add_symbol(&subroutine_name, symbol_info);

        // Generate refactoring action with cross-file impact analysis
        self.create_workspace_aware_refactoring(node, qualified_name)
    }
}
```

**Rationale**: Follows established dual indexing pattern for 98% reference coverage while enabling workspace-wide refactoring operations.

#### 4. Performance Optimization Strategy
```rust
// Multi-tier caching for code actions
pub struct CodeActionCache {
    lru_cache: LruCache<String, Vec<CodeAction>>,  // 50MB limit
    ast_cache: HashMap<String, (Timestamp, Node)>, // AST reuse
    diagnostic_cache: HashMap<String, Vec<Diagnostic>>, // Perlcritic results
}

impl CodeActionCache {
    fn get_cached_actions(&mut self, uri: &str, range: Range, context: &CodeActionContext) -> Option<Vec<CodeAction>> {
        let cache_key = self.compute_cache_key(uri, range, context);
        self.lru_cache.get(&cache_key).cloned()
    }
}
```

**Rationale**: Aggressive caching with incremental invalidation ensures <50ms response times while managing memory usage efficiently.

#### 5. Error Handling and Tool Integration
```rust
// Graceful degradation strategy
impl PerlCriticIntegration {
    fn run_analysis(&self, file_path: &str) -> CriticResult {
        // Try external perlcritic first
        if let Ok(external_result) = self.run_external_perlcritic(file_path) {
            return CriticResult::External(external_result);
        }

        // Fallback to built-in analyzer
        let builtin_analyzer = BuiltInAnalyzer::new();
        let ast = self.parser.parse_file(file_path)?;
        let violations = builtin_analyzer.analyze(&ast, &file_content);

        CriticResult::Builtin(violations)
    }
}
```

**Rationale**: Ensures functionality availability regardless of external tool installation while providing optimal experience when tools are available.

## Implementation Architecture

### Component Responsibilities

#### LSP Server Layer (`lsp_server.rs`)
- **executeCommand Dispatch**: Route commands to appropriate providers
- **Code Action Integration**: Wire enhanced providers to LSP protocol
- **Capability Advertisement**: Ensure proper capability negotiation
- **Error Handling**: Standardized error responses with actionable messages

#### Provider Layer (`execute_command.rs`, `code_actions_enhanced.rs`)
- **Command Execution**: Implement business logic for executeCommand operations
- **Refactoring Analysis**: AST-based code action generation
- **Caching Management**: Performance optimization with incremental invalidation
- **Tool Integration**: External tool coordination with fallback strategies

#### Parser Integration Layer
- **AST Analysis**: Enhanced syntax tree analysis for refactoring
- **Incremental Updates**: Leverage existing parsing infrastructure
- **Symbol Resolution**: Cross-file navigation with dual indexing
- **Performance Monitoring**: Response time validation and optimization

#### Testing Infrastructure
- **TDD Validation**: Comprehensive test coverage for all features
- **Performance Testing**: Benchmark integration with regression detection
- **Integration Testing**: End-to-end workflow validation
- **Error Scenario Coverage**: Robust error handling validation

### Data Flow Architecture

```
Client Request (executeCommand/codeAction)
    ↓
LSP Server (protocol handling)
    ↓
Provider Layer (business logic)
    ↓
Parser Integration (AST analysis)
    ↓
Cache Management (performance optimization)
    ↓
Tool Integration (external/internal)
    ↓
Response Generation (structured results)
    ↓
Client Response (LSP-compliant format)
```

### Integration Patterns

#### Existing Infrastructure Reuse
- **Document Management**: `IncrementalDocument` for change tracking
- **Position Conversion**: UTF-16/UTF-8 symmetric position mapping
- **Threading Model**: Adaptive threading with `LspHarness` integration
- **Diagnostic Pipeline**: Existing diagnostic publication workflow

#### New Component Integration
- **Enhanced Code Actions**: Wire to `textDocument/codeAction` handler
- **Import Optimization**: Integrate existing `ImportOptimizer` with code actions
- **Perlcritic Integration**: Complete executeCommand workflow
- **Test Infrastructure**: Expand existing test harness for new features

## Consequences

### Positive Outcomes

#### Developer Productivity Gains
- **Integrated Workflow**: Code quality analysis and refactoring directly in editor
- **Reduced Context Switching**: Eliminate external tool usage for common operations
- **Faster Refactoring**: AST-aware operations with cross-file impact analysis
- **Improved Code Quality**: Immediate feedback through perlcritic integration

#### Technical Benefits
- **Performance Excellence**: <50ms code action responses with incremental analysis
- **Reliability**: 95%+ test coverage with TDD validation approach
- **Maintainability**: Leverages existing infrastructure with minimal architectural changes
- **Extensibility**: Framework for additional refactoring operations and commands

#### Business Value
- **Reduced Development Cycle**: 40% faster code review cycle with integrated quality feedback
- **Enterprise Scalability**: Cross-file refactoring with workspace-wide impact analysis
- **Tool Consolidation**: Single LSP interface for multiple development workflows
- **Quality Assurance**: Automated policy enforcement through editor integration

### Potential Risks and Mitigations

#### Performance Risks
**Risk**: Code action analysis might impact LSP responsiveness
**Mitigation**: Aggressive caching with incremental invalidation, async execution patterns

**Risk**: Memory usage growth with expanded caching
**Mitigation**: LRU cache with 50MB limit, periodic cleanup of stale entries

#### Integration Risks
**Risk**: Breaking changes to existing executeCommand functionality
**Mitigation**: Comprehensive regression testing, backward compatibility validation

**Risk**: External tool dependency issues
**Mitigation**: Built-in analyzer fallback with feature parity for common use cases

#### Quality Risks
**Risk**: Complex refactoring operations introducing syntax errors
**Mitigation**: AST validation before/after operations, rollback capability

**Risk**: Cross-file refactoring breaking dependencies
**Mitigation**: Workspace impact analysis with dual indexing validation

### Monitoring and Success Metrics

#### Performance Metrics
- Code action response time: <50ms target
- Execute command completion: <2s for perlcritic analysis
- Memory usage: <100MB overhead for new features
- Test execution time: Maintain existing adaptive threading performance

#### Quality Metrics
- Test coverage: 95% for new functionality
- Integration test pass rate: 100% for critical workflows
- Error handling coverage: Comprehensive edge case validation
- Documentation completeness: Diátaxis framework compliance

#### User Experience Metrics
- Feature adoption rate through LSP client usage analytics
- Developer productivity feedback through integrated workflow surveys
- Code quality improvement measurement through policy violation reduction

## Implementation Timeline

### Phase 1: Core Integration (Week 1-2)
- Enable `perl.runCritic` tests and validate functionality
- Implement code action LSP protocol integration
- Basic performance optimization with caching

### Phase 2: Advanced Features (Week 2-3)
- Complete refactoring operations integration
- Import management code actions
- Cross-file analysis with dual indexing

### Phase 3: Quality Assurance (Week 3-4)
- Comprehensive test coverage expansion
- Performance benchmarking and optimization
- Documentation completion and validation

## Review and Approval

**Technical Review**: Parser architecture team, LSP integration specialists
**Quality Assurance**: TDD compliance validation, performance regression testing
**Documentation Review**: Diátaxis framework adherence, API documentation completeness
**Stakeholder Approval**: Issue #145 resolution validation, acceptance criteria fulfillment

---

**Decision Status**: DRAFT → REVIEW PENDING
**Implementation Priority**: P0 - Critical functionality
**Quality Gate**: TDD with comprehensive test coverage and performance validation
**Success Criteria**: All 5 acceptance criteria met