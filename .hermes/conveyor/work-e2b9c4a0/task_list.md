# Task List: Missing Label Resolution Validation for Loop Control

## Overview
Implement label resolution validation for `next`/`last`/`redo` loop control statements as a new lint in `perl-lsp-diagnostics`, following the established pattern from `goto_label.rs`.

## Task 1: Add LoopLabelUndefined (PL410) diagnostic code
- **Description**: Add the new `LoopLabelUndefined` variant to the `DiagnosticCode` enum in `perl-diagnostics/src/codes/mod.rs`, implementing all required trait methods (`as_str`, `documentation_url`, `severity`, `tags`, `context_hint`).
- **Inputs**: 
  - `crates/perl-diagnostics/src/codes/mod.rs` (existing PL409 and other codes as reference)
  - `specs.md` (PL410 specification)
- **Outputs**: 
  - Modified `crates/perl-diagnostics/src/codes/mod.rs` with `LoopLabelUndefined` variant
- **Depends on**: None
- **Complexity**: Small

## Task 2: Make has_label helper pub(crate) in goto_label.rs
- **Description**: Change the visibility of `has_label` function from private (`fn`) to `pub(crate)` in `crates/perl-lsp-diagnostics/src/lints/goto_label.rs` so it can be reused by the new `loop_label.rs` lint.
- **Inputs**: 
  - `crates/perl-lsp-diagnostics/src/lints/goto_label.rs`
- **Outputs**: 
  - Modified `goto_label.rs` with `pub(crate) fn has_label`
- **Depends on**: None
- **Complexity**: Small

## Task 3: Create loop_label.rs lint implementation
- **Description**: Create the new lint file `crates/perl-lsp-diagnostics/src/lints/loop_label.rs` implementing `check_loop_labels` function. The lint walks the AST, finds `NodeKind::LoopControl` nodes with labels, checks if the label exists in the symbol table using the shared `has_label` helper, and emits a PL410 diagnostic if not found.
- **Inputs**: 
  - `crates/perl-lsp-diagnostics/src/lints/goto_label.rs` (pattern to follow)
  - `crates/perl-ast/src/ast.rs` (NodeKind::LoopControl structure: `op: String`, `label: Option<String>`)
  - `crates/perl-semantic-analyzer/src/analysis/symbol.rs` (SymbolKind::Label, SymbolTable usage)
- **Outputs**: 
  - New file: `crates/perl-lsp-diagnostics/src/lints/loop_label.rs`
- **Depends on**: Task 2 (must have `has_label` visibility change)
- **Complexity**: Medium

## Task 4: Register loop_label module in mod.rs
- **Description**: Add `pub mod loop_label;` to `crates/perl-lsp-diagnostics/src/lints/mod.rs` with appropriate documentation, following the pattern of other lint modules.
- **Inputs**: 
  - `crates/perl-lsp-diagnostics/src/lints/mod.rs`
  - `specs.md` (documentation requirements)
- **Outputs**: 
  - Modified `crates/perl-lsp-diagnostics/src/lints/mod.rs`
- **Depends on**: Task 3
- **Complexity**: Small

## Task 5: Integrate check_loop_labels into diagnostics pipeline
- **Description**: Add `use crate::lints::loop_label::check_loop_labels;` import and call `check_loop_labels(ast, &symbol_table, &mut diagnostics);` in `crates/perl-lsp-diagnostics/src/diagnostics.rs`, alongside the existing `check_goto_labels` call around line 167.
- **Inputs**: 
  - `crates/perl-lsp-diagnostics/src/diagnostics.rs`
- **Outputs**: 
  - Modified `diagnostics.rs`
- **Depends on**: Task 4
- **Complexity**: Small

## Implementation Order Rationale

1. **Task 1 first**: The diagnostic code PL410 must exist before the lint can emit diagnostics with that code.
2. **Task 2 second**: The `has_label` helper must be made `pub(crate)` before the new lint can use it.
3. **Task 3 (create lint) third**: The core implementation depends on the visibility change from Task 2.
4. **Task 4 (mod.rs) fourth**: The module must be created and registered before it can be imported.
5. **Task 5 (integration) last**: The lint must be registered in `mod.rs` before it can be imported and called in `diagnostics.rs`.

## Task Complexity Summary
- **Small (3 tasks)**: Task 1, 2, 4, 5
- **Medium (1 task)**: Task 3
- **Total**: 5 tasks

## Key Risks
1. **Visibility change side effects**: Making `has_label` `pub(crate)` is a minor API expansion — verify no other internal uses are affected.
2. **AST node location**: `LoopControl` node's `location` field should cover the full statement including the label. Verify with a test case.
3. **Symbol table timing**: The `symbol_table` is built via `SymbolExtractor::new_with_source(source).extract(ast)` which must run before the lint check. This is already the case in `diagnostics.rs` (line 156 vs 167), so no ordering risk.
