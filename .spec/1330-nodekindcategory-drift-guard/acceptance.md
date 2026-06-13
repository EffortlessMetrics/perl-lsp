# Acceptance Criteria: NodeKindCategory Drift-Guard for Document Symbols

## Behavior Preservation

| Case | Input | Expected Output | Test |
|------|-------|-----------------|------|
| **Subroutine symbol** | `NodeKind::Subroutine { name: Some("foo"), .. }` | Emit `LspWorkspaceSymbol { kind: 12 (Function), name: "foo", .. }` | `extract_symbols_subroutine_emits_function` (existing passes) |
| **Package symbol** | `NodeKind::Package { name: "Foo", .. }` | Emit `LspWorkspaceSymbol { kind: 2 (Module), name: "Foo", .. }` | `extract_symbols_package_emits_module` (new or existing) |
| **Class symbol** | `NodeKind::Class { name: "MyClass", .. }` | Emit `LspWorkspaceSymbol { kind: 5 (Class), name: "MyClass", .. }` | `extract_symbols_class_emits_class` (new or existing) |
| **Method symbol** | `NodeKind::Method { name: "my_method", .. }` | Emit `LspWorkspaceSymbol { kind: 6 (Method), name: "my_method", .. }` | `extract_symbols_method_emits_method` (new or existing) |
| **Our variable symbol** | `NodeKind::VariableDeclaration { declarator: "our", variable: Variable { sigil: "$", name: "x" }, .. }` | Emit `LspWorkspaceSymbol { kind: 13 (Variable), name: "$x", .. }` | `extract_symbols_our_var_emits_variable_kind` (existing passes) |
| **My variable NOT indexed** | `NodeKind::VariableDeclaration { declarator: "my", .. }` | Do NOT emit symbol | `extract_symbols_my_var_not_indexed` (existing passes) |
| **Has attribute symbol** | `NodeKind::FunctionCall { name: "has", args: [String { value: "attr_name" }, ..] }` | Emit `LspWorkspaceSymbol { kind: 7 (Property), name: "attr_name", .. }` | `extract_symbols_has_attr_emits_property_kind` (existing passes) |
| **Program node recurse** | `NodeKind::Program { statements: [Subroutine, ..] }` | Recurse and extract nested symbols | existing tests pass |
| **Block node recurse** | `NodeKind::Block { statements: [Subroutine, ..] }` | Recurse and extract nested symbols | existing tests pass |
| **ExpressionStatement recurse** | `NodeKind::ExpressionStatement { expression: FunctionCall { name: "has", .. } }` | Recurse into expression | existing tests pass |

---

## Drift-Guard Enforcement

- [ ] **Compile-time protection active**: Category guard `kind.category() == NodeKindCategory::Declaration` is placed before the 6 symbol-emitting arms
- [ ] **Inner match exhaustive over Declaration variants**: The inner `match &node.kind { ... }` inside the guard has explicit arms for Subroutine, Package, Class, Method, VariableDeclaration, FunctionCall, and a catch-all `_` for non-indexed Declaration variants (Use, No, PhaseBlock, DataSection, Format, etc.)
- [ ] **Hypothetical new Declaration variant fails to compile**: If a developer adds a new `NodeKind::Role { .. }` with `category() == Declaration` and forgets to handle it in `extract_symbols_recursive`, the inner match will produce a compiler error: "pattern `NodeKind::Role { .. }` not covered in this match". This prevents silent drops of new Declaration types.
- [ ] **Mechanism documented**: Test comment or PR description explains the guard mechanism and why it matters (prevents accidental symbol drops on future NodeKind expansion)

---

## Code Quality

| Requirement | Acceptance |
|-------------|-----------|
| **No banned patterns** | `symbol_extraction.rs` contains no `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in production code (tests may use `perl_tdd_support::must`) |
| **Format compliance** | `cargo fmt --all` produces no changes |
| **Clippy clean** | `cargo clippy -p perl-lsp-rs --lib` produces no new warnings |
| **Import correct** | `use perl_parser::ast::{NodeKind, NodeKindCategory}` is added (or equivalent merged import) |
| **Symbol-kind mapping unchanged** | All 6 per-variant arms emit their original symbol kinds (Function=12, Module=2, Class=5, Method=6, Variable=13, Property=7) |

---

## Test Suite

### Existing Tests (must all pass unchanged)

- [ ] `extract_symbols_our_var_emits_variable_kind` — "our $VERSION" produces kind 13 (Variable)
- [ ] `extract_symbols_my_var_not_indexed` — "my $local" produces no symbol
- [ ] `extract_symbols_has_attr_emits_property_kind` — "has 'attr'" produces kind 7 (Property)
- [ ] Any other symbol extraction tests in the crate

### New Test

- [ ] `extract_symbols_category_guard_enforces_declarations` — Documents the guard and verifies all 6 declaration types still emit correct symbols

---

## CI Verification

The following CI checks must pass (3 gates):

1. **Perl LSP Rust Small Result** — `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib -- --test-threads=2` includes symbol extraction tests ✓
2. **ripr+ New Gap Gate** — Workspace-wide `cargo xtask fmt` and `cargo clippy --workspace --lib` produce no new issues ✓
3. **Codecov / Patch 95%** — New test coverage meets 95% threshold (new test exercises the category guard path) ✓

---

## Diff Audit

- [ ] Exactly 1 file modified: `crates/perl-lsp-rs/src/runtime/symbol_extraction.rs`
- [ ] Exactly 2 logical changes: (1) import NodeKindCategory, (2) wrap 6 arms in category guard + inner match
- [ ] No unintended diffs (no formatting of unrelated code, no whitespace-only changes outside the target function)
- [ ] Commit message: `fix(document-symbols):<description>` (builder responsibility)

---

## Functional Edge Cases

- [ ] **Declaration that doesn't emit**: A Declaration variant (e.g., `NodeKind::Format { .. }`) that doesn't emit a symbol is silently ignored by the inner `_ => {}` arm. This is correct — it's a Declaration, but we choose not to index it.
- [ ] **Non-Declaration that's mistakenly emitted as symbol**: Behavior unchanged (impossible with this refactoring; all 6 symbol-emitting arms are Declaration variants).
- [ ] **Recursion**: Program, Block, and ExpressionStatement are not Declaration variants; they skip the outer guard and recurse directly. This is correct.

---

## Historical Correctness

Before this refactoring, the function would silently drop any new Declaration variant added to NodeKind (because the outer `_ => {}` catch-all would match it and do nothing). After this refactoring, a new Declaration variant **cannot** be added without triggering a compiler error, forcing the developer to explicitly choose: emit a symbol, or add a `_ => {}` wildcard inside the category guard. This is the **drift-guard promise**.

---

## Sign-Off

Red-TDD builder: Verify test expectations above and commit as `test(document-symbols): add category guard tests for #1330`.

Builder: Implement per checklist, run all tests, ensure all above criteria pass, commit as `fix(document-symbols): adopt NodeKindCategory drift-guard for #1330`.

Reviewer: Confirm no symbol-kind mapping changes, no unintended regressions, category guard logic is sound.
