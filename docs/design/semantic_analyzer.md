# Semantic Analyzer Enhancement Design
<!-- Design Document: Comprehensive AST Node Type Coverage for Semantic Analysis -->
<!-- Issue: #188 -->
<!-- Author: Claude Code (2025-11-19) -->
<!-- Status: PHASE 1 - ✅ COMPLETE (2025-11-20) -->

> **🎯 IMPLEMENTATION GOAL**: Achieve complete AST node type coverage in semantic analyzer to enable rich IDE features including semantic tokens, hover information, symbol navigation, and code intelligence.
>
> **✅ IMPLEMENTATION STATUS** (2025-11-20): Phase 1 is **100% complete** with all 12 critical node handlers implemented, SemanticModel wrapper added, and LSP integration validated. Issues #220, #221, #227 delivered AST integration and comprehensive test coverage. Ready for Phase 2/3 enhancement.

## Executive Summary

The semantic analyzer in `crates/perl-parser/src/semantic.rs` now handles all Phase 1 critical node types with comprehensive LSP integration. This design outlines the phased approach to achieve 100% AST node type coverage, enabling comprehensive LSP features.

**Current State** (2025-11-20): ✅ Phase 1 complete - 12/12 critical handlers implemented, SemanticModel wrapper added, LSP integration validated
**Target State**: 100% AST node coverage with rich semantic information for all Perl constructs
**Completed Effort**: Phase 1 complete (8 story points), Phases 2-3 remaining (6 story points)

---

## Problem Statement

### Current Limitations

From `semantic.rs:684-687`:
```rust
_ => {
    // Handle other node types as needed
}
```

The analyzer currently handles:
- ✅ Variables (declarations, references)
- ✅ Subroutines (declarations, calls)
- ✅ Packages
- ✅ Control flow (if, while, for, foreach)
- ✅ Basic literals (string, number, regex)
- ✅ Operators (binary, assignment)
- ✅ **Phase 1 Complete: All 12 critical handlers implemented** (2025-11-20)

### Node Type Coverage Status

**✅ Phase 1 COMPLETE** (All 12 critical handlers implemented, 8 story points):
- ✅ `ExpressionStatement` - Wrapper for top-level expressions
- ✅ `Try` - Modern try/catch error handling
- ✅ `Eval` - Eval blocks and string eval
- ✅ `Do` - Do blocks
- ✅ `VariableListDeclaration` - Multi-variable declarations (`my ($x, $y)`)
- ✅ `VariableWithAttributes` - Variables with attributes (`:shared`, `:lvalue`)
- ✅ `Ternary` - Conditional expressions (`$x ? $y : $z`)
- ✅ `Unary` - Unary operators (`++`, `--`, `!`, `-`)
- ✅ `Readline` - Diamond operator (`<>`, `<STDIN>`)
- ✅ `ArrayLiteral` - Array constructors (`[]`)
- ✅ `HashLiteral` - Hash constructors (`{}`)
- ✅ `PhaseBlock` - BEGIN/END/CHECK/INIT/UNITCHECK blocks

**Important for completeness** (Phase 2, 3 story points):
- `Substitution` - s/// operators
- `Transliteration` - tr/// operators
- `MethodCall` - Object method invocations
- `Reference` - Ref operators (`\$x`, `\@arr`)
- `Dereference` - Deref operators (`$$ref`, `@$ref`)
- `Use` - use/require statements
- `Given`/`When` - Switch statements
- `Return`/`Next`/`Last`/`Redo` - Control flow keywords

**Advanced features** (Phase 3, 3 story points):
- `PostfixLoop` - Postfix for/while
- `Format` - Format declarations
- `FileTest` - File test operators (`-e`, `-d`, etc.)
- `Prototype` - Subroutine prototypes
- `Signature` - Modern subroutine signatures
- Additional operators and special forms

---

## Design Principles

### 1. Incremental Enhancement
- **Don't break existing functionality**: All current tests must pass
- **Additive changes only**: New node handlers extend, don't replace
- **Backward compatibility**: Existing LSP features continue to work

### 2. Consistency with Existing Patterns
- **Follow established token type mapping**: Use existing `SemanticTokenType` enum
- **Maintain scope tracking**: Respect lexical scoping rules
- **Extract documentation**: Use `extract_documentation()` for all declarations
- **Generate hover info**: Provide rich context for all symbols

### 3. Performance Characteristics
- **O(n) traversal**: Single-pass AST walk
- **Lazy evaluation**: Compute hover info on-demand where possible
- **Memory efficiency**: ~1MB per 10K lines target maintained
- **Incremental updates**: Support <1ms updates for typical changes

### 4. LSP Workflow Integration
```
Parse → Index → Navigate → Complete → Analyze
         ^
         |
    Semantic Analyzer: Complete coverage of all Perl constructs
```

---

## Phase 1: Critical LSP Features (Days 1-5, 8 story points)

### Goal
Handle all node types essential for basic LSP functionality: syntax highlighting, hover, and navigation.

### Implementation Strategy

#### 1.1 Expression Wrappers
```rust
NodeKind::ExpressionStatement { expression } => {
    // Unwrap and analyze the inner expression
    self.analyze_node(expression, scope_id);
}
```

#### 1.2 Error Handling Constructs
```rust
NodeKind::Try { block, catches } => {
    self.analyze_node(block, scope_id);
    for catch in catches {
        self.analyze_node(catch, scope_id);
    }
}

NodeKind::Eval { block } => {
    // Eval creates a new scope
    let eval_scope = self.get_scope_for(node, ScopeKind::Block);
    self.analyze_node(block, eval_scope);
}

NodeKind::Do { block } => {
    self.analyze_node(block, scope_id);
}
```

#### 1.3 Multi-Variable Declarations
```rust
NodeKind::VariableListDeclaration { declarator, variables, initializer } => {
    for var in variables {
        // Each variable gets semantic token + hover
        self.semantic_tokens.push(SemanticToken {
            location: var.location,
            token_type: SemanticTokenType::VariableDeclaration,
            modifiers: vec![SemanticTokenModifier::Declaration],
        });
    }
    if let Some(init) = initializer {
        self.analyze_node(init, scope_id);
    }
}
```

#### 1.4 Phase Blocks
```rust
NodeKind::PhaseBlock { phase, block } => {
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Keyword,
        modifiers: vec![],
    });
    self.analyze_node(block, scope_id);
}
```

#### 1.5 Operators and Expressions
```rust
NodeKind::Ternary { condition, then_expr, else_expr } => {
    self.analyze_node(condition, scope_id);
    self.analyze_node(then_expr, scope_id);
    self.analyze_node(else_expr, scope_id);
}

NodeKind::Unary { operand, .. } => {
    self.analyze_node(operand, scope_id);
}
```

#### 1.6 Literals and Constructors
```rust
NodeKind::ArrayLiteral { elements } => {
    for elem in elements {
        self.analyze_node(elem, scope_id);
    }
}

NodeKind::HashLiteral { pairs } => {
    for (key, value) in pairs {
        self.analyze_node(key, scope_id);
        self.analyze_node(value, scope_id);
    }
}

NodeKind::Readline { filehandle } => {
    if let Some(fh) = filehandle {
        self.analyze_node(fh, scope_id);
    }
}
```

### Acceptance Criteria (Phase 1)
- [ ] All 12 critical node types handled with semantic tokens
- [ ] Hover information generated for declarations
- [ ] Existing tests continue to pass (0 regressions)
- [ ] New smoke tests validate each node type (12 tests minimum)
- [ ] CI gates remain green (just ci-gate)
- [ ] Performance maintained: <1ms incremental updates

---

## Phase 2: Enhanced Features (Days 6-7, 3 story points)

### Goal
Handle operators, method calls, and advanced control flow for complete IDE experience.

### Implementation

#### 2.1 Operators
```rust
NodeKind::Substitution { pattern, replacement, .. } => {
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });
    self.analyze_node(pattern, scope_id);
    self.analyze_node(replacement, scope_id);
}

NodeKind::Transliteration { .. } => {
    // Similar to substitution
}
```

#### 2.2 Method Calls
```rust
NodeKind::MethodCall { invocant, method, args } => {
    self.analyze_node(invocant, scope_id);

    self.semantic_tokens.push(SemanticToken {
        location: method.location,
        token_type: SemanticTokenType::Method,
        modifiers: vec![],
    });

    for arg in args {
        self.analyze_node(arg, scope_id);
    }
}
```

#### 2.3 References and Dereferencing
```rust
NodeKind::Reference { referent } => {
    self.analyze_node(referent, scope_id);
}

NodeKind::Dereference { expression, .. } => {
    self.analyze_node(expression, scope_id);
}
```

#### 2.4 Use/Require
```rust
NodeKind::Use { module, imports, .. } => {
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Namespace,
        modifiers: vec![SemanticTokenModifier::DefaultLibrary],
    });

    // Track imports for completion/navigation
}
```

### Acceptance Criteria (Phase 2)
- [ ] Method calls have correct semantic tokens
- [ ] Operators highlighted consistently
- [ ] Use/require statements tracked for module resolution
- [ ] 20+ additional smoke tests cover new handlers

---

## Phase 3: Complete Coverage (Days 8-9, 3 story points)

### Goal
Handle all remaining AST node types for 100% coverage.

### Implementation
- Postfix loops
- Format blocks
- File test operators
- Prototypes and signatures
- Special forms and edge cases

### Acceptance Criteria (Phase 3)
- [ ] 100% AST node type coverage (no catch-all)
- [ ] All smoke tests passing
- [ ] Documentation complete for all handlers
- [ ] Performance benchmarks validated

---

## Testing Strategy

### Smoke Test Pattern
```rust
#[test]
fn test_try_block_semantic() {
    let code = r#"
try {
    my $x = risky_operation();
} catch {
    warn "Error: $_";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should have semantic tokens for keywords, variables
    let tokens = analyzer.semantic_tokens();
    assert!(tokens.iter().any(|t| matches!(
        t.token_type,
        SemanticTokenType::Keyword
    )));

    // Should have hover info for $x
    let x_symbols = analyzer.symbol_table().find_symbol(
        "x",
        0,
        SymbolKind::ScalarVariable
    );
    assert!(!x_symbols.is_empty());
}
```

### Test Organization
- **Unit tests**: One test per node type handler
- **Integration tests**: Complex multi-node scenarios
- **Regression tests**: Preserve existing functionality
- **Performance tests**: Validate <1ms update target

---

## Implementation Checklist

### Phase 1 (Sprint B Days 1-5)
- [ ] Add `ExpressionStatement` handler
- [ ] Add `Try`/`Eval`/`Do` handlers
- [ ] Add `VariableListDeclaration` handler
- [ ] Add `VariableWithAttributes` handler
- [ ] Add `Ternary`/`Unary` handlers
- [ ] Add `Readline` handler
- [ ] Add `ArrayLiteral`/`HashLiteral` handlers
- [ ] Add `PhaseBlock` handler
- [ ] Write 12 smoke tests
- [ ] Update documentation
- [ ] Validate CI gates

### Phase 2 (Sprint B Days 6-7)
- [ ] Add operator handlers (Substitution, Transliteration)
- [ ] Add `MethodCall` handler with hover
- [ ] Add `Reference`/`Dereference` handlers
- [ ] Add `Use`/`Require` handlers with import tracking
- [ ] Add `Given`/`When` handlers
- [ ] Add control flow keyword handlers
- [ ] Write 20+ additional tests

### Phase 3 (Sprint B Days 8-9)
- [ ] Add remaining node type handlers
- [ ] Remove catch-all `_` pattern
- [ ] Add `#[deny(unreachable_patterns)]` to ensure completeness
- [ ] Complete documentation for all handlers
- [ ] Final performance validation

---

## Performance Targets

| Metric | Current | Phase 1 Target | Phase 3 Target |
|--------|---------|----------------|----------------|
| AST Node Coverage | ~40% | ~75% | 100% |
| Analysis Time | O(n) | O(n) | O(n) |
| Memory per 10K lines | ~1MB | ~1.2MB | ~1.5MB |
| Incremental Update | <1ms | <1ms | <1ms |
| Semantic Token Count | ~300/1K lines | ~450/1K lines | ~600/1K lines |

---

## Integration with LSP Features

### Semantic Tokens (textDocument/semanticTokens)
- **Enhanced**: Complete syntax highlighting for all Perl constructs
- **Performance**: Incremental token generation
- **Quality**: Consistent token types across all node types

### Hover (textDocument/hover)
- **Enhanced**: Rich information for all declarations
- **Context**: Documentation extraction from POD/comments
- **Navigation**: "Go to definition" links embedded

### Symbol Navigation (textDocument/definition, references)
- **Enhanced**: Accurate symbol resolution for all variable/function types
- **Cross-package**: Qualified name handling
- **Scope-aware**: Lexical scoping rules enforced

### Code Intelligence
- **Completion**: Context-aware suggestions
- **Diagnostics**: Undefined variable/function detection
- **Refactoring**: Safe rename and extract operations

---

## Risk Mitigation

### Risk: Breaking Existing Tests
- **Mitigation**: Run `just ci-gate` after each handler addition
- **Validation**: Maintain 274+ test pass rate throughout

### Risk: Performance Degradation
- **Mitigation**: Benchmark after each phase
- **Target**: <1ms incremental update maintained

### Risk: Scope Creep
- **Mitigation**: Strict phase boundaries, defer advanced features to Phase 3
- **Timeline**: 2-3 day buffer built into Sprint B estimate

---

## Documentation Requirements

### Code Comments
- Each new handler must have:
  - Purpose comment
  - Example Perl code
  - Semantic token type explanation

### User-Facing Docs
- Update `docs/LSP_IMPLEMENTATION_GUIDE.md` with new semantic token types
- Add examples to `docs/LSP_DEVELOPMENT_GUIDE.md`

---

## Success Metrics

### Phase 1 Success
- ✅ 12 critical node types handled
- ✅ 12 new smoke tests passing
- ✅ 0 test regressions
- ✅ CI gates green

### Phase 2 Success
- ✅ 20+ additional node types handled
- ✅ 32+ total smoke tests
- ✅ Enhanced method call support
- ✅ Operator highlighting complete

### Phase 3 Success
- ✅ 100% AST node coverage
- ✅ All tests passing (50+ smoke tests)
- ✅ Performance targets met
- ✅ Documentation complete
- ✅ No unreachable patterns in match statement

---

## Future Enhancements (Post-Sprint B)

1. **Type Inference**: Track variable types for smarter completion
2. **Data Flow Analysis**: Detect unused variables, dead code
3. **Cross-File Analysis**: Full workspace symbol resolution
4. **Refactoring Safety**: Impact analysis for renames
5. **Performance Optimization**: Incremental semantic token caching

---

## References

- Issue #188: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/188
- Sprint B Meta: Issue #213
- LSP Specification: https://microsoft.github.io/language-server-protocol/
- AST Definition: `crates/perl-parser/src/ast.rs`
- Current Analyzer: `crates/perl-parser/src/semantic.rs`

---

*Design Document Version: 1.1*
*Last Updated: 2025-11-20*
*Status: ✅ Phase 1 Complete - Ready for Phase 2/3 Enhancement*
