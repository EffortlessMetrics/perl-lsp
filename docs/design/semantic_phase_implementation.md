# Semantic Analyzer Phase 1→2→3 Implementation Guide

**Issue**: #188
**Status**: Phase 1 50% (6/12), Ready for Completion
**Timeline**: Phase 1 (1-2 days), Phase 2 (2-3 days), Phase 3 (1-2 days)

---

## Current Progress Summary

### ✅ Implemented (6/12 Phase 1 handlers)
1. `VariableListDeclaration` - lines 685-728
2. `Ternary` - lines 732-737
3. `ArrayLiteral` - lines 739-744
4. `HashLiteral` - lines 746-752
5. `Try` - lines 754-766
6. `PhaseBlock` - lines 768-776

### ❌ Remaining Phase 1 (6/12 handlers)
7. `ExpressionStatement`
8. `Do`
9. `Eval`
10. `VariableWithAttributes`
11. `Unary`
12. `Readline`

---

## Phase 1: Complete Critical LSP Features (6 handlers, ~2-4 hours)

### Handler 7: ExpressionStatement

**Location**: Add after `PhaseBlock` handler (~line 777)
**Complexity**: Trivial (wrapper node)
**Time**: 5 minutes

```rust
NodeKind::ExpressionStatement { expression } => {
    // ExpressionStatement is just a wrapper - analyze the inner expression
    self.analyze_node(expression, scope_id);
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_expression_statement_semantic
```

**Why this matters**: Enables semantic analysis for top-level expressions like `$x + 10;`

---

### Handler 8: Do

**Location**: Add after `ExpressionStatement` (~line 782)
**Complexity**: Simple (block wrapper)
**Time**: 10 minutes

```rust
NodeKind::Do { block } => {
    // Do blocks: my $value = do { calculate() };
    // The block can return a value but doesn't create a new scope
    self.analyze_node(block, scope_id);
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_do_block_semantic
```

**Why this matters**: Enables semantic analysis for do-block expressions (common in functional Perl)

---

### Handler 9: Eval

**Location**: Add after `Do` (~line 788)
**Complexity**: Medium (creates new scope)
**Time**: 15 minutes

```rust
NodeKind::Eval { block, is_string } => {
    // Eval creates a new scope for exception handling
    // Block eval: eval { risky_operation() };
    // String eval: eval "code";

    if let Some(block_node) = block {
        // Block eval - analyze in new scope
        let eval_scope = self.get_scope_for(node, ScopeKind::Block);
        self.analyze_node(block_node, eval_scope);
    } else if *is_string {
        // String eval - can't analyze dynamic code
        // But we can still track the eval keyword
        self.semantic_tokens.push(SemanticToken {
            location: node.location,
            token_type: SemanticTokenType::Keyword,
            modifiers: vec![],
        });
    }
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_eval_block_semantic
```

**Why this matters**: Enables scope-aware analysis for eval blocks (critical for exception handling)

---

### Handler 10: VariableWithAttributes

**Location**: Add after `Eval` (~line 808)
**Complexity**: Medium (attribute handling)
**Time**: 20 minutes

```rust
NodeKind::VariableWithAttributes { variable, attributes } => {
    // Variables with attributes: my $shared :shared = 42;
    // Attributes like :lvalue, :shared, :unique

    // First analyze the variable itself
    self.analyze_node(variable, scope_id);

    // Track attributes as modifiers
    for attr in attributes {
        self.semantic_tokens.push(SemanticToken {
            location: attr.location,
            token_type: SemanticTokenType::Modifier,
            modifiers: vec![],
        });
    }
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_variable_with_attributes_semantic
```

**Why this matters**: Enables proper highlighting for modern Perl attributes (threading, lvalue subs)

---

### Handler 11: Unary

**Location**: Add after `VariableWithAttributes` (~line 827)
**Complexity**: Simple (recursive)
**Time**: 10 minutes

```rust
NodeKind::Unary { operator, operand } => {
    // Unary operators: ++$x, --$x, -$x, !$x, ~$x, etc.

    // Track the operator token
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    // Analyze the operand
    self.analyze_node(operand, scope_id);
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_unary_operators_semantic
```

**Why this matters**: Enables operator highlighting for all unary operations (common in Perl)

---

### Handler 12: Readline

**Location**: Add after `Unary` (~line 843)
**Complexity**: Medium (filehandle handling)
**Time**: 15 minutes

```rust
NodeKind::Readline { filehandle, operator } => {
    // Readline operators: <STDIN>, <$fh>, <>, <>
    // The diamond operator and file handle reads

    // Track the operator token (< >)
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    // If there's a filehandle expression, analyze it
    if let Some(fh) = filehandle {
        self.analyze_node(fh, scope_id);
    }
}
```

**Test verification**:
```bash
cargo test -p perl-parser --test semantic_smoke_tests test_readline_operator_semantic
```

**Why this matters**: Enables proper analysis for I/O operations (ubiquitous in Perl scripts)

---

## Phase 1 Acceptance Criteria Validation

After implementing all 6 handlers, run:

```bash
# All Phase 1 smoke tests should pass
cargo test -p perl-parser --test semantic_smoke_tests | grep -E "test_.*semantic.*ok"

# Should show 13 passed, 8 ignored
cargo test -p perl-parser --test semantic_smoke_tests

# Existing semantic.rs unit tests should still pass
cargo test -p perl-parser --lib semantic

# CI gates should remain green
just ci-gate
```

**Expected results**:
- ✅ 13/13 Phase 1 smoke tests pass
- ✅ 14/14 semantic.rs unit tests pass
- ✅ 0 regressions in existing tests
- ✅ Performance maintained: <1ms incremental updates
- ✅ Zero clippy warnings

---

## Phase 2: Enhanced Features (8 handlers, ~4-6 hours)

### Overview
Phase 2 adds operators, method calls, and advanced control flow. These enable:
- Complete syntax highlighting for all operators
- Method call navigation
- Import/module tracking

### Handler List

1. **Substitution** (`s///` operator)
2. **Transliteration** (`tr///` operator)
3. **MethodCall** (object method invocations)
4. **Reference** (`\$x`, `\@arr`)
5. **Dereference** (`$$ref`, `@$ref`)
6. **Use** (use/require statements)
7. **Given**/**When** (switch statements)
8. **Return**/**Next**/**Last**/**Redo** (control flow)

### Implementation Template

```rust
// Add to semantic.rs after Phase 1 handlers

// ============================================================================
// Phase 2: Enhanced Features (Issue #188)
// ============================================================================

NodeKind::Substitution { pattern, replacement, modifiers, .. } => {
    // Substitution operator: s/pattern/replacement/modifiers
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    // Analyze pattern and replacement expressions
    if let Some(pattern_node) = pattern {
        self.analyze_node(pattern_node, scope_id);
    }
    if let Some(replacement_node) = replacement {
        self.analyze_node(replacement_node, scope_id);
    }
}

NodeKind::Transliteration { search_list, replacement_list, modifiers, .. } => {
    // Transliteration operator: tr/abc/xyz/
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    // These are typically strings, no further analysis needed
}

NodeKind::MethodCall { invocant, method, arguments } => {
    // Method call: $obj->method(@args)

    // Analyze the invocant (object)
    self.analyze_node(invocant, scope_id);

    // Track the method name
    self.semantic_tokens.push(SemanticToken {
        location: method.location,
        token_type: SemanticTokenType::Method,
        modifiers: vec![],
    });

    // Generate hover info for method
    if let Some(method_name) = self.extract_identifier_name(method) {
        self.hover_info.insert(
            method.location,
            HoverInfo {
                signature: format!("->{}()", method_name),
                documentation: None,
                details: vec!["Method call".to_string()],
            },
        );
    }

    // Analyze arguments
    for arg in arguments {
        self.analyze_node(arg, scope_id);
    }
}

NodeKind::Reference { referent } => {
    // Reference operator: \$x, \@arr, \&func
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    self.analyze_node(referent, scope_id);
}

NodeKind::Dereference { expression, dereference_type } => {
    // Dereference operator: $$ref, @$ref, %$ref, &$ref
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    self.analyze_node(expression, scope_id);
}

NodeKind::Use { module, imports, version } => {
    // Use statement: use Module qw(imports);

    // Track the use keyword
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Keyword,
        modifiers: vec![],
    });

    // Track the module name
    if let Some(mod_node) = module {
        self.semantic_tokens.push(SemanticToken {
            location: mod_node.location,
            token_type: SemanticTokenType::Namespace,
            modifiers: vec![SemanticTokenModifier::DefaultLibrary],
        });
    }

    // Track imported symbols
    for import in imports {
        self.semantic_tokens.push(SemanticToken {
            location: import.location,
            token_type: SemanticTokenType::Function,
            modifiers: vec![],
        });
    }
}

NodeKind::Given { expression, when_blocks, default_block } => {
    // Given/when (switch): given ($x) { when (1) {...} }

    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Keyword,
        modifiers: vec![],
    });

    // Analyze the switch expression
    self.analyze_node(expression, scope_id);

    // Analyze each when block
    for when_block in when_blocks {
        self.analyze_node(when_block, scope_id);
    }

    // Analyze default block if present
    if let Some(default) = default_block {
        self.analyze_node(default, scope_id);
    }
}

NodeKind::When { condition, block } => {
    // When clause in given/when
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Keyword,
        modifiers: vec![],
    });

    self.analyze_node(condition, scope_id);
    self.analyze_node(block, scope_id);
}

NodeKind::Return { value } => {
    // Return statement
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::KeywordControl,
        modifiers: vec![],
    });

    if let Some(return_value) = value {
        self.analyze_node(return_value, scope_id);
    }
}

NodeKind::Next { label } | NodeKind::Last { label } | NodeKind::Redo { label } => {
    // Loop control: next, last, redo
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::KeywordControl,
        modifiers: vec![],
    });

    if let Some(label_node) = label {
        self.semantic_tokens.push(SemanticToken {
            location: label_node.location,
            token_type: SemanticTokenType::Label,
            modifiers: vec![],
        });
    }
}

// ============================================================================
// End of Phase 2
// ============================================================================
```

### Phase 2 Test Strategy

Create `test_phase2_handlers` smoke tests:

```rust
#[test]
fn test_phase2_substitution() {
    let code = r#"my $text = "hello"; $text =~ s/hello/world/g;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();
    let op_tokens: Vec<_> = tokens.iter()
        .filter(|t| matches!(t.token_type, SemanticTokenType::Operator))
        .collect();

    assert!(!op_tokens.is_empty(), "Should have operator tokens");
}

#[test]
fn test_phase2_method_call() {
    let code = r#"my $obj = Foo->new(); $obj->process();"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();
    let method_tokens: Vec<_> = tokens.iter()
        .filter(|t| matches!(t.token_type, SemanticTokenType::Method))
        .collect();

    assert!(method_tokens.len() >= 2, "Should have method tokens for new() and process()");
}
```

---

## Phase 3: Complete Coverage (remaining handlers, ~3-4 hours)

### Overview
Phase 3 achieves 100% AST node type coverage, handling all remaining edge cases:

- Postfix loops
- Format blocks
- File test operators
- Prototypes and signatures
- Special forms

### Implementation Strategy

1. **List all remaining NodeKind variants**:
   ```bash
   rg "pub enum NodeKind" crates/perl-parser/src/ast.rs -A 200 | grep "^    [A-Z]"
   ```

2. **For each unhandled variant**, add a handler following patterns from Phase 1/2

3. **Remove the catch-all**:
   ```rust
   // REMOVE THIS:
   _ => {
       // Handle other node types as needed
   }

   // REPLACE WITH:
   // (All specific handlers above)
   ```

4. **Add exhaustiveness check**:
   ```rust
   // At the top of the match statement in analyze_node():
   #[deny(unreachable_patterns)]
   match &node.kind {
       // ... all handlers ...
   }
   ```

### Phase 3 Handlers (Quick Reference)

```rust
NodeKind::PostfixLoop { condition, body, loop_type } => {
    // say $_ for @items;
    self.analyze_node(body, scope_id);
    self.analyze_node(condition, scope_id);
}

NodeKind::Format { name, body } => {
    // format NAME = ...
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Keyword,
        modifiers: vec![],
    });
}

NodeKind::FileTest { operator, operand } => {
    // -e $file, -d $path, -r $filename
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });

    self.analyze_node(operand, scope_id);
}

NodeKind::Prototype { signature } => {
    // sub foo ($$$) { ... }
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Type,
        modifiers: vec![],
    });
}

NodeKind::Signature { parameters } => {
    // sub foo($x, $y) { ... }  (modern Perl signatures)
    for param in parameters {
        self.semantic_tokens.push(SemanticToken {
            location: param.location,
            token_type: SemanticTokenType::Parameter,
            modifiers: vec![SemanticTokenModifier::Declaration],
        });
    }
}
```

---

## Incremental Development Workflow

### Step-by-step process for each handler

1. **Write the handler** (5-20 minutes)
   - Copy template from this guide
   - Adjust for specific NodeKind fields
   - Add semantic tokens as needed

2. **Write/verify the test** (5-10 minutes)
   ```bash
   cargo test -p perl-parser --test semantic_smoke_tests test_<handler_name>_semantic
   ```

3. **Check for regressions** (2 minutes)
   ```bash
   cargo test -p perl-parser --lib semantic
   cargo test -p perl-parser --test semantic_smoke_tests
   ```

4. **Commit with clear message** (1 minute)
   ```bash
   git add crates/perl-parser/src/semantic.rs
   git commit -m "feat(#188): add <NodeKind> handler for semantic analyzer

   - Implements semantic token generation for <feature>
   - Adds hover information for <symbols>
   - Test: test_<handler>_semantic passing
   - Part of Phase <1|2|3> (handler <X>/<Y>)"
   ```

5. **Move to next handler** (repeat)

### Batch testing commands

```bash
# After implementing 3-4 handlers, run comprehensive checks:
cargo test -p perl-parser --test semantic_smoke_tests
cargo test -p perl-parser --lib semantic
cargo clippy -p perl-parser
cargo fmt -- --check

# Validate performance hasn't regressed:
cargo test -p perl-parser --test incremental_parsing_tests
```

---

## Performance Validation Checklist

After each phase, validate these targets:

```bash
# Incremental parsing still <1ms
cargo test -p perl-parser --test incremental_parsing_tests

# Semantic token generation is efficient
cargo bench -p perl-parser semantic_tokens

# No memory leaks
cargo test -p perl-parser --test semantic_smoke_tests -- --nocapture
# Watch for warnings about dropped resources
```

**Target metrics**:
- ✅ Analysis time: O(n) where n = AST nodes
- ✅ Memory usage: ~1-1.5MB per 10K lines
- ✅ Incremental update: <1ms maintained
- ✅ Token generation: <50μs per token

---

## Common Pitfalls & Solutions

### Pitfall 1: Forgetting to recurse
❌ **Wrong**:
```rust
NodeKind::ArrayLiteral { elements } => {
    // Oops - forgot to analyze elements!
}
```

✅ **Right**:
```rust
NodeKind::ArrayLiteral { elements } => {
    for elem in elements {
        self.analyze_node(elem, scope_id);  // Recurse!
    }
}
```

### Pitfall 2: Wrong scope ID
❌ **Wrong**:
```rust
NodeKind::Eval { block } => {
    self.analyze_node(block, scope_id);  // Wrong - should create new scope
}
```

✅ **Right**:
```rust
NodeKind::Eval { block } => {
    let eval_scope = self.get_scope_for(node, ScopeKind::Block);
    self.analyze_node(block, eval_scope);  // Correct!
}
```

### Pitfall 3: Missing semantic tokens
❌ **Wrong**:
```rust
NodeKind::Unary { operator, operand } => {
    self.analyze_node(operand, scope_id);  // Missing operator token!
}
```

✅ **Right**:
```rust
NodeKind::Unary { operator, operand } => {
    self.semantic_tokens.push(SemanticToken {
        location: node.location,
        token_type: SemanticTokenType::Operator,
        modifiers: vec![],
    });
    self.analyze_node(operand, scope_id);
}
```

---

## Success Metrics

### Phase 1 Complete (Target: 2 days max)
- ✅ 12/12 critical handlers implemented
- ✅ 13/13 smoke tests passing
- ✅ 0 test regressions
- ✅ <1ms incremental parsing maintained
- ✅ CI gates green

### Phase 2 Complete (Target: 2-3 days)
- ✅ 20/20 enhanced handlers implemented
- ✅ 21/21 smoke tests passing (13 Phase 1 + 8 Phase 2)
- ✅ Method call hover working
- ✅ Use/require tracking functional

### Phase 3 Complete (Target: 1-2 days)
- ✅ 100% AST node coverage (no catch-all)
- ✅ All 50+ smoke tests passing
- ✅ `#[deny(unreachable_patterns)]` enabled
- ✅ Performance targets met
- ✅ Documentation complete

### Overall Sprint B Success
- ✅ Semantic analyzer 100% complete
- ✅ SemanticModel API integrated
- ✅ At least 1 LSP feature migrated to new API
- ✅ Issue #188 closed with confidence

---

## Integration with Sprint B Timeline

| Days | Phase | Deliverable |
|------|-------|-------------|
| **1-2** | Phase 1 Complete | 12/12 handlers, SemanticModel API integrated |
| **3-4** | LSP Migration | Wire definition/hover to SemanticModel |
| **5-6** | Phase 2 Start | 8 enhanced handlers + tests |
| **7-8** | Phase 2 Complete | Method calls, operators functional |
| **9** | Phase 3 | Complete remaining handlers, 100% coverage |

**Buffer**: 2-3 days built into Sprint B estimate (15-16 hours planned, ~12 hours actual for #188)

---

## Quick Reference: All Node Kinds to Handle

```
Phase 1 (Critical - 6 remaining):
[✅] VariableListDeclaration
[✅] Ternary
[✅] ArrayLiteral
[✅] HashLiteral
[✅] Try
[✅] PhaseBlock
[❌] ExpressionStatement
[❌] Do
[❌] Eval
[❌] VariableWithAttributes
[❌] Unary
[❌] Readline

Phase 2 (Enhanced - 8 total):
[❌] Substitution
[❌] Transliteration
[❌] MethodCall
[❌] Reference
[❌] Dereference
[❌] Use/Require
[❌] Given/When
[❌] Return/Next/Last/Redo

Phase 3 (Complete - TBD):
[❌] PostfixLoop
[❌] Format
[❌] FileTest
[❌] Prototype
[❌] Signature
[❌] ... (remaining NodeKind variants)
```

---

*Ready for immediate execution - follow this guide step-by-step for systematic completion!*
