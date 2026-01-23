# Bolt's Journal

This journal tracks critical performance learnings for the `tree-sitter-perl-rs` repository.

## 2026-01-21 - [Allocation in Hot Path Analysis]
**Learning:** `format!` in recursive AST traversal (like `ScopeAnalyzer`) causes significant memory churn. Checking properties of composite values (like variable sigil + name) should be done on components before allocation.
**Action:** When filtering or checking nodes in hot paths, pass references to components (`&str`, `&str`) instead of constructing owned `String`s just for the check.

## 2024-05-22 - [Initial Setup]
**Learning:** Performance benchmarks are available in `crates/perl-parser/benches/`. The `ast_to_sexp` benchmark currently emits many parse errors, which might noise up the results.
**Action:** When running benchmarks, ensure valid input or filter out known noisy benchmarks if they obscure the target optimization.

## 2026-01-23 - [Deferred Allocation in ScopeAnalyzer]
**Learning:** Confirmed that `ScopeAnalyzer` was eagerly allocating full variable names strings even when not needed. Deferred allocation until issue reporting.
**Action:** Applied deferred allocation pattern to `VariableDeclaration` and `VariableListDeclaration`. Attempted benchmarking but faced environment stability issues ("Internal error"); relied on structural correctness (removing heap allocation in hot path) and `cargo check`.
