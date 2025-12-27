# Semantic Analyzer Phase 2 Implementation Guide

**Status**: Phase 1 Complete (12/12 critical handlers) | Phase 2 Ready for Implementation
**Last Updated**: 2025-11-21
**Target Audience**: Contributors implementing Phase 2 node handlers

## Table of Contents

1. [Overview](#overview)
2. [Phase 2 Scope](#phase-2-scope)
3. [Handler Implementation Pattern](#handler-implementation-pattern)
4. [Testing Workflow](#testing-workflow)
5. [Local CI Protocol](#local-ci-protocol)
6. [Common Pitfalls](#common-pitfalls)
7. [Integration Checklist](#integration-checklist)

---

## Overview

### What Phase 2 Adds

Phase 2 extends the semantic analyzer with **advanced Perl language constructs** that build on Phase 1's foundation:

- **Method Call Resolution**: `$obj->method()`, `Class->static_method()`
- **Substitution Operators**: `s/pattern/replacement/`, `tr/a-z/A-Z/`
- **Reference Operations**: `\$scalar`, `\@array`, `\%hash`, dereferencing
- **Module Import Analysis**: `use Module qw(symbols)`, `require "file.pm"`

### Why Phase 2 Matters

Phase 1 provides **lexical symbol resolution** (variables, subroutines, packages). Phase 2 enables:

1. **Object-Oriented Navigation**: Jump to method definitions across inheritance hierarchies
2. **Pattern Matching Context**: Understand substitution operators in LSP hover/completion
3. **Module Dependency Analysis**: Resolve imported symbols to their source files
4. **Advanced Refactoring**: Rename methods across class hierarchies, extract modules

### Architecture Context

```
Phase 1 Foundation (Complete)
├── Symbol Definition Tracking (scalars, arrays, hashes, subs, packages)
├── Lexical Scope Management (my/our/local, nested blocks)
├── Package Namespace Resolution (Package::sub, $Package::var)
└── AST Node Visitor Pattern (handle_* methods)

Phase 2 Extensions (Target)
├── Method Call Resolution (OO dispatch, inheritance chains)
├── Substitution Context Analysis (pattern/replacement scopes)
├── Reference Type Tracking (scalar refs, array refs, hash refs)
└── Module Import Indexing (use/require symbol resolution)
```

---

## Phase 2 Scope

### Target Node Types

**Priority 1 (Core Language Features)**:
- `method_call_expression` - `$obj->method()`, `Class->new()`
- `substitution_operator` - `s/find/replace/g`, `tr/a-z/A-Z/`
- `reference_expression` - `\$var`, `\@array`, `\%hash`
- `dereference_expression` - `$$ref`, `@$arrayref`, `%$hashref`

**Priority 2 (Module System)**:
- `use_statement` - `use Module qw(func1 func2);`
- `require_expression` - `require "File.pm";`

**Priority 3 (Advanced Features)**:
- `blessed_expression` - `bless $obj, 'Class';`
- `isa_expression` - `$obj->isa('Class')`
- `can_expression` - `$obj->can('method')`

### Out of Scope (Future Phases)

- **Regex Capture Groups**: Requires control flow analysis (Phase 3)
- **Typeglob Manipulation**: Complex symbol table operations (Phase 4)
- **Eval Context**: Dynamic code execution (Phase 4)
- **Tie Operations**: Magic variable handling (Phase 4)

---

## Handler Implementation Pattern

### Step 1: Choose Your Node Type

Select a node type from the Phase 2 scope based on:

1. **Dependency Order**: Implement foundational nodes first (e.g., `reference_expression` before `dereference_expression`)
2. **Complexity**: Start with simpler nodes (e.g., `use_statement` before `method_call_expression` with inheritance)
3. **Testing Coverage**: Pick nodes with existing parser tests in `perl-corpus`

**Example Decision Tree**:
```
Start with use_statement (no dependencies, well-tested)
  → Then require_expression (similar pattern)
    → Then reference_expression (foundational for derefs)
      → Then dereference_expression (builds on refs)
        → Finally method_call_expression (most complex, requires inheritance analysis)
```

### Step 2: Analyze Existing Parser Tests

**Before writing any code**, study how the parser handles your node type:

```bash
# Find existing parser tests for your node
cd /home/steven/code/Rust/perl-lsp/review
cargo test -p perl-parser -- use_statement --nocapture

# Check tree-sitter grammar definition
grep -A 20 "use_statement" crates/tree-sitter-perl/grammar.js

# Examine AST structure with debug parsing
echo 'use Foo qw(bar baz);' | perl-lsp --debug-ast
```

**Key Questions to Answer**:
- What child nodes exist? (identifier, qualified_name, qw_list, etc.)
- What text fields are available? (module name, version, import list)
- What edge cases exist? (no import list, pragma modules, version numbers)

### Step 3: Implement the Handler Method

**Template Pattern** (`/crates/perl-parser/src/semantic.rs`):

```rust
/// Handles `use Module qw(symbols);` statements
///
/// Tracks imported symbols and resolves them to the source module.
/// Supports three patterns:
/// 1. `use Module;` - imports default exports
/// 2. `use Module qw(sym1 sym2);` - imports specific symbols
/// 3. `use Module ();` - no imports (compile-time effects only)
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    // 1. Extract module name (required field)
    let module_name = node
        .child_by_field_name("module")
        .and_then(|n| self.extract_text(n, source))
        .unwrap_or_default();

    // 2. Extract import list (optional field)
    let import_list = node
        .child_by_field_name("imports")
        .map(|n| self.extract_import_symbols(n, source))
        .unwrap_or_default();

    // 3. Define the import statement itself
    self.define_symbol(
        module_name.clone(),
        SymbolKind::Module,
        node.start_byte(),
    );

    // 4. Define each imported symbol in current scope
    for symbol in import_list {
        self.define_symbol(
            symbol.clone(),
            SymbolKind::Function, // May be variable/constant in reality
            node.start_byte(),
        );
    }

    // 5. Recurse into child nodes (if needed)
    self.visit_children(node, source);
}
```

**Handler Responsibilities**:
1. **Extract Relevant Data**: Module names, symbol lists, qualifiers
2. **Define Symbols**: Add to current scope with appropriate `SymbolKind`
3. **Track References**: Record where symbols are imported from
4. **Maintain Scope**: Respect lexical/package scope boundaries
5. **Recurse Appropriately**: Visit child nodes that may contain nested definitions

### Step 4: Add Helper Methods (If Needed)

For complex extraction logic, add private helper methods:

```rust
/// Extracts symbol names from qw() or explicit import lists
fn extract_import_symbols(&self, node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    // Handle qw(foo bar baz) syntax
    if node.kind() == "qw_list" {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "qw_element" {
                if let Some(text) = self.extract_text(child, source) {
                    symbols.push(text);
                }
            }
        }
    }

    // Handle explicit list: (foo, bar, baz)
    if node.kind() == "list_expression" {
        // Parse comma-separated identifiers...
    }

    symbols
}
```

### Step 5: Register the Handler

Add your handler to the `analyze_node` dispatcher in `semantic.rs`:

```rust
fn analyze_node(&mut self, node: tree_sitter::Node, source: &str) {
    match node.kind() {
        // Phase 1 handlers (existing)
        "variable_declaration" => self.handle_variable_declaration(node, source),
        "subroutine_declaration_statement" => self.handle_subroutine_declaration(node, source),
        // ... other Phase 1 handlers ...

        // Phase 2 handlers (new)
        "use_statement" => self.handle_use_statement(node, source),
        "require_expression" => self.handle_require_expression(node, source),
        "method_call_expression" => self.handle_method_call(node, source),
        "substitution_operator" => self.handle_substitution(node, source),
        "reference_expression" => self.handle_reference(node, source),
        "dereference_expression" => self.handle_dereference(node, source),

        _ => {
            // Recurse into unhandled nodes
            self.visit_children(node, source);
        }
    }
}
```

---

## Testing Workflow

### Phase 1: Parser-Level Tests (Fast, No LSP)

**Location**: `/crates/perl-parser/tests/semantic_*_tests.rs`

**Goal**: Validate handler logic in isolation without LSP overhead.

#### Template Test Structure

```rust
// File: /crates/perl-parser/tests/semantic_use_statement_tests.rs

use perl_parser::semantic::{SemanticAnalyzer, SymbolKind};

#[test]
fn test_use_statement_imports_symbols() {
    let code = r#"
use Foo qw(bar baz);
my $x = bar();
"#;

    let mut analyzer = SemanticAnalyzer::new();
    let tree = parse_code(code); // Helper function (see existing tests)
    analyzer.analyze(tree.root_node(), code);

    // Test 1: Module itself is defined
    let module_def = analyzer.find_definition_by_name("Foo");
    assert!(module_def.is_some(), "Module 'Foo' should be defined");
    assert_eq!(module_def.unwrap().kind, SymbolKind::Module);

    // Test 2: Imported symbols are defined
    let bar_def = analyzer.find_definition_by_name("bar");
    assert!(bar_def.is_some(), "Imported symbol 'bar' should be defined");
    assert_eq!(bar_def.unwrap().kind, SymbolKind::Function);

    // Test 3: Non-imported symbols are NOT defined
    let qux_def = analyzer.find_definition_by_name("qux");
    assert!(qux_def.is_none(), "Non-imported symbol 'qux' should not be defined");
}

#[test]
fn test_use_statement_no_import_list() {
    let code = r#"
use strict;
my $x = 1;
"#;

    let mut analyzer = SemanticAnalyzer::new();
    let tree = parse_code(code);
    analyzer.analyze(tree.root_node(), code);

    // Pragma modules don't define symbols in user namespace
    let strict_def = analyzer.find_definition_by_name("strict");
    assert!(strict_def.is_some(), "Pragma module 'strict' should be tracked");
}

#[test]
fn test_use_statement_scoping() {
    let code = r#"
{
    use Foo qw(bar);
    my $x = bar();
}
my $y = bar(); # Should not resolve - 'bar' out of scope
"#;

    let mut analyzer = SemanticAnalyzer::new();
    let tree = parse_code(code);
    analyzer.analyze(tree.root_node(), code);

    // Test scoping rules (imports are lexically scoped)
    // This requires calculating positions dynamically...
    let bar_in_block = find_position_of(code, "bar()");
    let bar_outside = find_position_of(code, "bar(); #");

    let def_in_block = analyzer.find_definition(bar_in_block);
    assert!(def_in_block.is_some(), "bar() inside block should resolve");

    let def_outside = analyzer.find_definition(bar_outside);
    assert!(def_outside.is_none(), "bar() outside block should NOT resolve");
}
```

#### Running Parser Tests

```bash
# Run your specific test file
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --test semantic_use_statement_tests -- --nocapture

# Run all semantic tests (fast validation)
cargo test -p perl-parser semantic::tests -- --nocapture
```

### Phase 2: LSP Integration Tests (Slower, Real Protocol)

**Location**: `/crates/perl-lsp/tests/semantic_*.rs`

**Goal**: Validate `textDocument/definition` works end-to-end over JSON-RPC.

#### Template LSP Test Structure

```rust
// File: /crates/perl-lsp/tests/semantic_use_statement_lsp_tests.rs

use lsp_types::{Position, GotoDefinitionResponse};
use perl_lsp_test_harness::{LspTestHarness, TestResult};

#[test]
fn definition_resolves_imported_symbol() -> TestResult {
    let code = r#"
use Foo qw(bar);
my $x = bar();
"#;

    let mut harness = LspTestHarness::new()?;
    let uri = harness.open_file("test.pl", code)?;

    // Calculate position of 'bar()' call dynamically
    let bar_call_pos = find_dynamic_position(code, "bar()", 0);

    // Request definition
    let response = harness.goto_definition(&uri, bar_call_pos)?;

    // Validate response
    match response {
        Some(GotoDefinitionResponse::Scalar(location)) => {
            // Definition should point to 'use' statement line
            assert_eq!(location.uri, uri, "Definition should be in same file");
            assert_eq!(location.range.start.line, 1, "Definition on line 1 (use statement)");
        }
        _ => panic!("Expected scalar location response"),
    }

    Ok(())
}

#[test]
fn definition_handles_non_imported_symbol() -> TestResult {
    let code = r#"
use Foo qw(bar);
my $x = baz(); # 'baz' not imported
"#;

    let mut harness = LspTestHarness::new()?;
    let uri = harness.open_file("test.pl", code)?;

    let baz_call_pos = find_dynamic_position(code, "baz()", 0);
    let response = harness.goto_definition(&uri, baz_call_pos)?;

    // Should return None (symbol not defined)
    assert!(response.is_none(), "Non-imported symbol should not resolve");

    Ok(())
}
```

#### Running LSP Tests (Resource-Constrained)

**IMPORTANT**: Run LSP tests **one at a time** on memory-constrained systems to avoid OOM:

```bash
# Run individual tests sequentially
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_use_statement_lsp_tests -- --nocapture definition_resolves_imported_symbol

RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_use_statement_lsp_tests -- --nocapture definition_handles_non_imported_symbol

# Full suite (requires adequate memory)
just ci-lsp-def  # Runs all LSP semantic tests with proper constraints
```

### Phase 3: End-to-End Validation

**Location**: `/crates/perl-parser/tests/lsp_comprehensive_e2e_test.rs`

Add your node type to the comprehensive E2E test suite:

```rust
#[test]
fn test_use_statement_e2e() {
    let workspace = TestWorkspace::new();
    workspace.add_file("Foo.pm", r#"
package Foo;
sub bar { return 42; }
1;
"#);

    workspace.add_file("main.pl", r#"
use Foo qw(bar);
my $result = bar();
"#);

    let server = workspace.start_lsp_server();

    // Test 1: Definition from main.pl finds Foo.pm::bar
    let def = server.goto_definition("main.pl", position_of("bar()"));
    assert_file_and_line(def, "Foo.pm", 2);

    // Test 2: Hover on 'bar()' shows signature
    let hover = server.hover("main.pl", position_of("bar()"));
    assert_contains(hover, "sub bar");
}
```

---

## Local CI Protocol

### Overview

Validate your changes **without GitHub Actions** using local CI tools:

1. **just ci-gate**: Fast local validation matching CI requirements
2. **nix flake check**: Hermetic build validation (if Nix available)
3. **Manual Test Suite**: Comprehensive test coverage validation

### Step 1: Fast Local Validation (just ci-gate)

**Prerequisites**: Install `just` command runner (optional but recommended)

```bash
# Install just (if not already installed)
cargo install just

# Run local CI gate (matches GitHub Actions)
just ci-gate

# What it validates:
# - Compilation: cargo check --workspace
# - Formatting: cargo fmt --check
# - Linting: cargo clippy --workspace -- -D warnings
# - Parser tests: cargo test -p perl-parser
# - LSP tests: RUST_TEST_THREADS=2 cargo test -p perl-lsp
# - Documentation: cargo doc --no-deps --package perl-parser
```

**Expected Output**:
```
✓ Compilation successful
✓ Formatting correct
✓ No clippy warnings
✓ Parser tests: 245/245 passed
✓ LSP tests: 71/71 passed
✓ Documentation builds without warnings
```

### Step 2: LSP Semantic Tests (Individual Execution)

**For memory-constrained systems** (WSL, low-RAM VMs):

```bash
# Run each LSP test individually to avoid OOM
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_finds_scalar_variable_declaration

RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_finds_subroutine_declaration

RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_resolves_scoped_variables

RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_handles_package_qualified_calls
```

**CI-Ready Batch Execution** (requires adequate compute):

```bash
# Run all LSP semantic tests with proper resource constraints
just ci-lsp-def
```

### Step 3: Hermetic Build Validation (Nix Flake)

**Prerequisites**: Nix package manager with flakes enabled

```bash
# Check if Nix is available
nix --version  # Should show 2.4+ with flakes support

# Run hermetic build and test
nix flake check

# What it validates:
# - Clean build from scratch (no cargo cache pollution)
# - All tests in isolated environment
# - Cross-platform compatibility (Linux x86_64)
# - Dependency resolution correctness
```

**Expected Output**:
```
evaluating derivation 'git+file:///home/steven/code/Rust/perl-lsp#checks.x86_64-linux.default'
building '/nix/store/xxx-perl-lsp-check.drv'...
✓ Build successful
✓ Tests passed
✓ No dependency conflicts
```

### Step 4: Manual Test Coverage Validation

**Ensure comprehensive test coverage** for your handler:

```bash
# 1. Parser unit tests (required)
cargo test -p perl-parser --test semantic_YOUR_NODE_tests

# 2. LSP integration tests (required)
cargo test -p perl-lsp --test semantic_YOUR_NODE_lsp_tests

# 3. E2E workspace tests (recommended)
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- YOUR_NODE_e2e

# 4. Mutation testing (optional but valuable)
cargo test -p perl-parser --test mutation_hardening_tests
```

### Step 5: Documentation Validation

**Ensure your handler is documented**:

```bash
# Validate no missing_docs warnings
cargo doc --no-deps --package perl-parser

# Run documentation tests
cargo test --doc -p perl-parser

# Check API documentation standards
cargo test -p perl-parser --test missing_docs_ac_tests
```

### Local CI Checklist

Before opening a PR, verify:

- [ ] `just ci-gate` passes (or manual equivalent)
- [ ] All parser tests pass individually
- [ ] All LSP tests pass individually (on constrained hardware)
- [ ] `nix flake check` passes (if Nix available)
- [ ] Documentation builds without warnings
- [ ] No clippy warnings in workspace
- [ ] Code formatted with `cargo fmt`

---

## Common Pitfalls

### Pitfall 1: Hardcoded Position Calculations

**Problem**: Tests break when whitespace/formatting changes.

**Bad Example**:
```rust
#[test]
fn test_definition() {
    let code = "use Foo qw(bar);\nmy $x = bar();";
    let bar_pos = 29; // Hardcoded byte offset - FRAGILE!

    let def = analyzer.find_definition(bar_pos);
    assert!(def.is_some());
}
```

**Good Example**:
```rust
#[test]
fn test_definition() {
    let code = "use Foo qw(bar);\nmy $x = bar();";
    let bar_pos = find_dynamic_position(code, "bar()", 0); // Dynamic calculation

    let def = analyzer.find_definition(bar_pos);
    assert!(def.is_some());
}

// Helper function (add to test utilities)
fn find_dynamic_position(source: &str, pattern: &str, occurrence: usize) -> usize {
    source.match_indices(pattern)
        .nth(occurrence)
        .map(|(idx, _)| idx)
        .expect(&format!("Pattern '{}' occurrence {} not found", pattern, occurrence))
}
```

**Lesson**: Always use dynamic position calculation in tests.

### Pitfall 2: Incorrect Scope Tracking

**Problem**: Symbols defined in inner scopes leak to outer scopes.

**Bad Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    let module_name = extract_module(node, source);

    // BUG: Defines in current scope without checking block depth
    self.define_symbol(module_name, SymbolKind::Module, node.start_byte());
}
```

**Good Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    let module_name = extract_module(node, source);

    // Correct: Define in current scope (respects block depth)
    // SemanticAnalyzer already tracks scope depth internally
    self.define_symbol(module_name, SymbolKind::Module, node.start_byte());

    // When entering a block, call enter_scope()
    // When exiting a block, call exit_scope()
}
```

**Lesson**: Trust `SemanticAnalyzer`'s scope management, but verify with scoping tests.

### Pitfall 3: Missing Child Node Recursion

**Problem**: Nested definitions inside your node type are not visited.

**Bad Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    let module_name = extract_module(node, source);
    self.define_symbol(module_name, SymbolKind::Module, node.start_byte());

    // BUG: Doesn't visit child nodes - misses nested structures
}
```

**Good Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    let module_name = extract_module(node, source);
    self.define_symbol(module_name, SymbolKind::Module, node.start_byte());

    // Correct: Recurse into children (e.g., import list with expressions)
    self.visit_children(node, source);
}
```

**Lesson**: Always call `visit_children()` unless you manually handle all child nodes.

### Pitfall 4: UTF-8 vs UTF-16 Position Mismatches

**Problem**: LSP uses UTF-16 positions, parser uses UTF-8 byte offsets.

**Bad Example**:
```rust
fn lsp_position_to_byte_offset(pos: Position, source: &str) -> usize {
    // BUG: Assumes 1 character = 1 byte (fails on Unicode)
    pos.line * 80 + pos.character
}
```

**Good Example**:
```rust
fn lsp_position_to_byte_offset(pos: Position, source: &str) -> usize {
    // Correct: Use Rope for UTF-16 → UTF-8 conversion
    let rope = ropey::Rope::from_str(source);
    let line_byte_offset = rope.line_to_byte(pos.line as usize);
    let char_offset = rope.utf16_cu_to_char(pos.character as usize);
    line_byte_offset + char_offset
}
```

**Lesson**: Always use `Rope` for position conversions (already integrated in LSP layer).

### Pitfall 5: Not Handling Optional Fields

**Problem**: Perl syntax allows many optional elements.

**Bad Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    // BUG: Panics if import list is missing (e.g., "use strict;")
    let imports = node.child_by_field_name("imports").unwrap();
}
```

**Good Example**:
```rust
fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    // Correct: Handle optional import list gracefully
    if let Some(imports_node) = node.child_by_field_name("imports") {
        let symbols = self.extract_import_symbols(imports_node, source);
        for symbol in symbols {
            self.define_symbol(symbol, SymbolKind::Function, node.start_byte());
        }
    }
    // If no imports, just track the module itself
}
```

**Lesson**: Use `if let Some(...)` for all optional tree-sitter fields.

### Pitfall 6: Ignoring Test Isolation

**Problem**: Tests depend on execution order or shared state.

**Bad Example**:
```rust
// Global state shared across tests
static mut ANALYZER: Option<SemanticAnalyzer> = None;

#[test]
fn test_a() {
    unsafe { ANALYZER = Some(SemanticAnalyzer::new()); }
    // Test logic...
}

#[test]
fn test_b() {
    // BUG: Depends on test_a running first
    let analyzer = unsafe { ANALYZER.as_ref().unwrap() };
}
```

**Good Example**:
```rust
#[test]
fn test_a() {
    // Each test creates its own analyzer
    let mut analyzer = SemanticAnalyzer::new();
    // Test logic...
}

#[test]
fn test_b() {
    // Independent test with its own analyzer
    let mut analyzer = SemanticAnalyzer::new();
    // Test logic...
}
```

**Lesson**: Every test should be fully independent and self-contained.

---

## Integration Checklist

### Pre-Implementation Checklist

Before starting implementation:

- [ ] Node type selected from Phase 2 scope
- [ ] Parser tests reviewed for node structure
- [ ] Tree-sitter grammar consulted for field names
- [ ] Example Perl code collected (simple + edge cases)
- [ ] Dependencies identified (what handlers must exist first?)

### Implementation Checklist

During handler implementation:

- [ ] Handler method signature matches pattern
- [ ] All required fields extracted (with `.unwrap_or_default()`)
- [ ] All optional fields handled gracefully (`if let Some(...)`)
- [ ] Symbols defined in correct scope
- [ ] `visit_children()` called if needed
- [ ] Helper methods added for complex extraction
- [ ] Handler registered in `analyze_node()` dispatcher
- [ ] Code formatted with `cargo fmt`
- [ ] No clippy warnings introduced

### Testing Checklist

During test development:

- [ ] Parser unit tests written (3+ test cases minimum)
- [ ] Tests use dynamic position calculation (no hardcoded offsets)
- [ ] Edge cases covered (missing fields, empty lists, etc.)
- [ ] Scoping behavior validated (lexical scope tests)
- [ ] LSP integration tests written (2+ test cases minimum)
- [ ] LSP tests run individually on constrained hardware
- [ ] All tests pass in isolation
- [ ] E2E test added to comprehensive suite (if applicable)

### Documentation Checklist

Before opening PR:

- [ ] Handler method documented with `///` rustdoc comments
- [ ] Example Perl code included in documentation
- [ ] Edge cases mentioned in documentation
- [ ] Helper methods documented
- [ ] `cargo doc` builds without warnings
- [ ] No new `missing_docs` warnings introduced

### CI Validation Checklist

Before pushing to GitHub:

- [ ] `just ci-gate` passes locally
- [ ] All parser tests pass: `cargo test -p perl-parser`
- [ ] All LSP tests pass individually (memory-constrained mode)
- [ ] `nix flake check` passes (if Nix available)
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Documentation builds: `cargo doc --no-deps --package perl-parser`

---

## Example: Complete Handler Implementation

### Step-by-Step Walkthrough

**Goal**: Implement `use_statement` handler for Phase 2.

#### Step 1: Analyze Parser Behavior

```bash
# Test how parser handles use statements
echo 'use Foo qw(bar baz);' | perl-lsp --debug-ast

# Output shows:
# use_statement
#   module: qualified_name "Foo"
#   imports: qw_list
#     qw_element "bar"
#     qw_element "baz"
```

#### Step 2: Write Parser Unit Test

```rust
// File: /crates/perl-parser/tests/semantic_use_statement_tests.rs

#[test]
fn test_use_statement_basic() {
    let code = r#"
use Foo qw(bar baz);
my $x = bar();
"#;

    let mut analyzer = SemanticAnalyzer::new();
    let tree = parse_code(code);
    analyzer.analyze(tree.root_node(), code);

    // Verify module defined
    let module_def = analyzer.find_definition_by_name("Foo");
    assert!(module_def.is_some());

    // Verify symbols imported
    let bar_def = analyzer.find_definition_by_name("bar");
    assert!(bar_def.is_some());
}
```

**Run test (should fail - handler not implemented yet)**:
```bash
cargo test -p perl-parser --test semantic_use_statement_tests -- --nocapture
# Expected: test_use_statement_basic ... FAILED
```

#### Step 3: Implement Handler (TDD Red → Green)

```rust
// File: /crates/perl-parser/src/semantic.rs

fn handle_use_statement(&mut self, node: tree_sitter::Node, source: &str) {
    // Extract module name
    let module_name = node
        .child_by_field_name("module")
        .and_then(|n| self.extract_text(n, source))
        .unwrap_or_default();

    // Define module symbol
    self.define_symbol(
        module_name.clone(),
        SymbolKind::Module,
        node.start_byte(),
    );

    // Extract and define imported symbols
    if let Some(imports_node) = node.child_by_field_name("imports") {
        let symbols = self.extract_qw_symbols(imports_node, source);
        for symbol in symbols {
            self.define_symbol(symbol, SymbolKind::Function, node.start_byte());
        }
    }

    self.visit_children(node, source);
}

fn extract_qw_symbols(&self, node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for child in node.children(&mut node.walk()) {
        if child.kind() == "qw_element" {
            if let Some(text) = self.extract_text(child, source) {
                symbols.push(text);
            }
        }
    }
    symbols
}
```

**Register handler**:
```rust
fn analyze_node(&mut self, node: tree_sitter::Node, source: &str) {
    match node.kind() {
        // ... existing handlers ...
        "use_statement" => self.handle_use_statement(node, source),
        _ => self.visit_children(node, source),
    }
}
```

**Run test again (should pass now)**:
```bash
cargo test -p perl-parser --test semantic_use_statement_tests -- --nocapture
# Expected: test_use_statement_basic ... ok
```

#### Step 4: Write LSP Integration Test

```rust
// File: /crates/perl-lsp/tests/semantic_use_statement_lsp_tests.rs

#[test]
fn definition_resolves_imported_symbol() -> TestResult {
    let code = r#"
use Foo qw(bar);
my $x = bar();
"#;

    let mut harness = LspTestHarness::new()?;
    let uri = harness.open_file("test.pl", code)?;

    let bar_pos = find_dynamic_position(code, "bar()", 0);
    let response = harness.goto_definition(&uri, bar_pos)?;

    assert!(response.is_some(), "Definition should resolve");
    Ok(())
}
```

**Run LSP test**:
```bash
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_use_statement_lsp_tests -- --nocapture
```

#### Step 5: Validate Locally

```bash
# Full local CI validation
just ci-gate

# Individual validations
cargo clippy --workspace
cargo fmt --check
cargo doc --no-deps --package perl-parser
```

#### Step 6: Open PR

Once all validations pass:

1. Commit with clear message: `feat(semantic): implement use_statement handler for Phase 2`
2. Push to feature branch
3. Open PR with checklist from this guide
4. Reference Issue #188 Phase 2 in PR description

---

## Next Steps

### For Contributors

1. **Pick a handler** from Phase 2 scope (start with `use_statement` or `reference_expression`)
2. **Follow this guide** step-by-step
3. **Ask questions** if stuck (Discord, GitHub Discussions, PR comments)
4. **Share learnings** by updating this guide with new pitfalls/patterns

### For Maintainers

1. **Review PRs** against this guide's checklist
2. **Provide feedback** on handler implementation patterns
3. **Update guide** when new patterns emerge
4. **Track progress** on Issue #188 Phase 2 milestone

---

## References

- **Phase 1 Implementation**: `/crates/perl-parser/src/semantic.rs` (12 handlers)
- **Phase 1 Tests**: `/crates/perl-parser/tests/semantic_*_tests.rs`
- **LSP Integration**: `/crates/perl-lsp/tests/semantic_definition.rs`
- **Issue Tracking**: GitHub Issue #188 (Semantic Analyzer Phases)
- **Architecture**: `/docs/semantic/SEMANTIC_ANALYZER_ARCHITECTURE.md`
- **Position Tracking**: `/docs/POSITION_TRACKING_GUIDE.md`
- **Testing Framework**: `/docs/LSP_DEVELOPMENT_GUIDE.md`

---

**Last Updated**: 2025-11-21
**Maintainer**: Semantic Analyzer Working Group
**Status**: Ready for Phase 2 Contributors
