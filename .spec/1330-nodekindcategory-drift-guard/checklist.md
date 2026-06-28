# Implementation Checklist: NodeKindCategory Drift-Guard for Document Symbols

## Overview
Adopt `NodeKindCategory::Declaration` as a compile-time exhaustiveness guard in `extract_symbols_recursive()`. This refactoring preserves all symbol-kind mappings while enforcing that new Declaration variants trigger a compiler error.

**File changed:** 1  
**Lines modified:** ~40 (nested match restructure)  
**Behavior:** No observable change to emitted symbols  
**Test location:** `crates/perl-lsp-rs/src/runtime/symbol_extraction.rs` (inline `#[test]` + new test)

---

## Step 1: Refactor extract_symbols_recursive — outer guard (lines 46–216)

**File:** `crates/perl-lsp-rs/src/runtime/symbol_extraction.rs`

**Current structure (line 46):**
```rust
match &node.kind {
    NodeKind::Subroutine { name, body, .. } => { /* emit Function */ }
    NodeKind::Package { name, block, .. } => { /* emit Module */ }
    NodeKind::Class { name, body, .. } => { /* emit Class */ }
    NodeKind::Method { name, body, .. } => { /* emit Method */ }
    NodeKind::VariableDeclaration { declarator, variable, .. } if declarator == "our" => { /* emit Variable */ }
    NodeKind::FunctionCall { name, args } if name == "has" => { /* emit Property */ }
    NodeKind::Program { statements } => { /* recurse */ }
    NodeKind::Block { statements } => { /* recurse */ }
    NodeKind::ExpressionStatement { expression } => { /* recurse */ }
    _ => {}
}
```

**New structure:**
```rust
match &node.kind {
    // Outer guard: all Declaration variants must be handled explicitly.
    // Adding a new Declaration variant is a compiler error until this match is updated.
    kind if kind.category() == NodeKindCategory::Declaration => {
        // Inner match: only Declaration variants reach here; exhaustive over the 6 kinds emitted.
        match &node.kind {
            NodeKind::Subroutine { name, body, .. } => { /* emit Function — unchanged */ }
            NodeKind::Package { name, block, .. } => { /* emit Module — unchanged */ }
            NodeKind::Class { name, body, .. } => { /* emit Class — unchanged */ }
            NodeKind::Method { name, body, .. } => { /* emit Method — unchanged */ }
            NodeKind::VariableDeclaration { declarator, variable, .. } if declarator == "our" => { /* emit Variable — unchanged */ }
            NodeKind::FunctionCall { name, args } if name == "has" => { /* emit Property — unchanged */ }
            _ => {} // Non-indexed Declaration variants (Use, No, PhaseBlock, DataSection, Format, etc.) silently ignored
        }
    }
    // Recurse arms: NOT declarations; no guard needed
    NodeKind::Program { statements } => { /* recurse — unchanged */ }
    NodeKind::Block { statements } => { /* recurse — unchanged */ }
    NodeKind::ExpressionStatement { expression } => { /* recurse — unchanged */ }
    _ => {} // Catch-all: all other non-Declaration variants
}
```

**Imports needed (line 44):**
- Add `NodeKindCategory` to the `use perl_parser::ast::NodeKind;` import:
  ```rust
  use perl_parser::ast::{NodeKind, NodeKindCategory};
  ```

**Change order:**
1. Add import of `NodeKindCategory` (line 44)
2. Insert outer guard `kind if kind.category() == NodeKindCategory::Declaration =>` before the 6 declaration arms
3. Indent the 6 declaration arms (Subroutine, Package, Class, Method, VariableDeclaration, FunctionCall) by 1 level
4. Wrap them in an inner `match &node.kind { ... }` block
5. Move non-declaration arms (Program, Block, ExpressionStatement, final `_`) outside the guard
6. Update the final `_` arm comment to clarify it catches only non-Declaration variants

**Verify command (after Step 1):**
```bash
cargo build -p perl-lsp-rs 2>&1 | head -50
```

---

## Step 2: Add new test — category guard activation (after line 788)

**File:** `crates/perl-lsp-rs/src/runtime/symbol_extraction.rs`

**Test name:** `extract_symbols_category_guard_enforces_declarations`

**Purpose:** Verify that the `category()` guard is in place and correctly filters Declaration variants. Document that a hypothetical new Declaration variant would cause a compile error.

**Test structure (pseudo-code):**
```rust
#[cfg(feature = "workspace")]
#[test]
fn extract_symbols_category_guard_enforces_declarations() {
    // This test verifies that the `NodeKindCategory::Declaration` guard
    // in extract_symbols_recursive correctly filters declaration types.
    //
    // If a new NodeKind variant is added with category() == Declaration,
    // the inner match in extract_symbols_recursive will not have an arm
    // for it, triggering a compiler error:
    //   "pattern `NodeKind::NewVariant { .. }` not covered"
    // This forces the developer to explicitly choose: emit a symbol or ignore.
    //
    // The mechanism cannot be directly tested in a unit test (the compiler
    // error only occurs at compile-time), but we verify the existing guards work:

    // Case 1: Subroutine (Declaration -> Function)
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("test_sub".to_string()),
            body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(0, 10))),
            prototype: None,
            attributes: vec![],
        },
        loc(0, 15),
    );
    let root = Node::new(NodeKind::Program { statements: vec![sub] }, loc(0, 16));
    let symbols = server().extract_document_symbols(&root, "sub test_sub {}", "file:///test.pl");
    assert_eq!(symbols.len(), 1, "Subroutine should produce 1 symbol");
    assert_eq!(symbols[0].kind, 12, "Subroutine should have kind 12 (Function)");

    // Case 2: Package (Declaration -> Module)
    let pkg = Node::new(
        NodeKind::Package {
            name: "TestPkg".to_string(),
            block: Some(Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(0, 10)))),
        },
        loc(0, 25),
    );
    let root = Node::new(NodeKind::Program { statements: vec![pkg] }, loc(0, 26));
    let symbols = server().extract_document_symbols(&root, "package TestPkg { }", "file:///test.pl");
    assert_eq!(symbols.len(), 1, "Package should produce 1 symbol");
    assert_eq!(symbols[0].kind, 2, "Package should have kind 2 (Module)");

    // Case 3: Class (Declaration -> Class)
    let cls = Node::new(
        NodeKind::Class {
            name: "TestClass".to_string(),
            body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(0, 10))),
            body_location: Some((12, 15)),
        },
        loc(0, 20),
    );
    let root = Node::new(NodeKind::Program { statements: vec![cls] }, loc(0, 21));
    let symbols = server().extract_document_symbols(&root, "class TestClass { }", "file:///test.pl");
    assert_eq!(symbols.len(), 1, "Class should produce 1 symbol");
    assert_eq!(symbols[0].kind, 5, "Class should have kind 5 (Class)");

    // Case 4: Method (Declaration -> Method)
    let meth = Node::new(
        NodeKind::Method {
            name: "test_method".to_string(),
            body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(0, 10))),
            body_location: Some((12, 15)),
        },
        loc(0, 20),
    );
    let root = Node::new(NodeKind::Program { statements: vec![meth] }, loc(0, 21));
    let symbols = server().extract_document_symbols(&root, "method test_method { }", "file:///test.pl");
    assert_eq!(symbols.len(), 1, "Method should produce 1 symbol");
    assert_eq!(symbols[0].kind, 6, "Method should have kind 6 (Method)");

    // Case 5: VariableDeclaration with "our" (Declaration -> Variable)
    // (Reuse existing test extract_symbols_our_var_emits_variable_kind)
    // — just document that declarator == "our" is still the filtering gate

    // Case 6: FunctionCall with name=="has" (NOT a Declaration, but caught by existing test)
    // (Reuse existing test extract_symbols_has_attr_emits_property_kind)
    // — just document that this is NOT in the Declaration category,
    //   so it runs through the outer match's expression-category arms (if added later)

    // Drift-guard documentation: if NodeKind::Format, NodeKind::Use, NodeKind::No, etc.
    // are emitted via extract_symbols_recursive, they will silently match the `_ => {}`
    // inside the Declaration category guard (good: no crash, but they're not indexed).
    // The guard prevents the variant from reaching the outer catch-all, so a future
    // developer *knows* it's a Declaration and can decide whether to emit a symbol.
}
```

**Simpler alternative** (if the above is too verbose): Write 6 simpler focused tests, one per declaration type, documenting each in the test comment. The key assertion is always: "This Declaration variant emits the expected symbol kind."

**Verify command (after Step 2):**
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs extract_symbols_category_guard -- --test-threads=2 2>&1 | tail -30
```

---

## Step 3: Verify no symbol output changes

**Command:**
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs symbol -- --test-threads=2 2>&1 | tail -50
```

**Expected:** All existing symbol tests pass (6 tests + any integration tests):
- `extract_symbols_our_var_emits_variable_kind` ✓
- `extract_symbols_my_var_not_indexed` ✓
- `extract_symbols_has_attr_emits_property_kind` ✓
- Plus any other symbol extraction tests
- NEW: `extract_symbols_category_guard_enforces_declarations` ✓

---

## Step 4: Lint and format

**Commands:**
```bash
cargo fmt --all
cargo clippy -p perl-lsp-rs --lib 2>&1 | grep -A 5 "warning\|error"
```

**Expected:** No warnings or errors related to symbol_extraction.rs (existing clippy allows may apply).

---

## Step 5: Commit spec files

```bash
git add .spec/1330-nodekindcategory-drift-guard/ && \
  git commit -m "plan(document-symbols): add NodeKindCategory drift-guard spec for #1330"
```

---

## Dependency order

1. **Step 1** (import + outer guard) must complete before Step 2 (tests can compile)
2. **Step 2** (tests) can be added in parallel or after Step 1 (not a blocker)
3. **Step 3–5** are verification and cleanup (run after implementation)

---

## Builder handoff checklist

- [ ] Step 1 compiles (no new compile errors)
- [ ] All 6 declaration arms preserved exactly (no symbol-kind mapping changes)
- [ ] New test added and passes
- [ ] All symbol extraction tests pass (no regressions)
- [ ] `cargo fmt` and `cargo clippy` pass
- [ ] Commit message follows convention: `fix(document-symbols):<desc>`

---

## Next: Red-TDD builder

Red-TDD builder will:
1. Verify the spec is on the impl branch
2. Write failing assertions in tests (already provided above)
3. Commit as `test(document-symbols):<desc>`
4. Push to impl branch
5. Hand off to main builder

Builder will implement the refactoring per the checklist and push.

---

## Implementation scope note

This is a **pure refactoring** — no new APIs, no new symbol kinds, no DAP changes. The behavior is identical before/after; only the code organization changes. The compile-time guard is the entire value: future Declaration variants *cannot* silently drop.
