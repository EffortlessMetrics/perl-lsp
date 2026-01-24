# Bolt's Journal

This journal tracks critical performance learnings for the `tree-sitter-perl-rs` repository.

## 2026-01-21 - [Allocation in Hot Path Analysis]
**Learning:** `format!` in recursive AST traversal (like `ScopeAnalyzer`) causes significant memory churn. Checking properties of composite values (like variable sigil + name) should be done on components before allocation.
**Action:** When filtering or checking nodes in hot paths, pass references to components (`&str`, `&str`) instead of constructing owned `String`s just for the check.

## 2024-05-22 - [Initial Setup]
**Learning:** Performance benchmarks are available in `crates/perl-parser/benches/`. The `ast_to_sexp` benchmark currently emits many parse errors, which might noise up the results.
**Action:** When running benchmarks, ensure valid input or filter out known noisy benchmarks if they obscure the target optimization.

## 2025-05-23 - [Optimization of Scope Data Structure]
**Learning:** `ScopeAnalyzer` performance was hindered by repeated `String` allocations and HashMap lookups for variable sigils (which are a small, fixed set).
**Action:** Replaced `HashMap<String, HashMap<...>>` with `[HashMap<...>; 6]` indexed by a helper `sigil_to_index`. This eliminates allocation for the outer key and reduces lookup overhead to O(1) array access. Note: Benchmarking in this environment is unstable (OOM/timeout), so optimization was verified by static analysis and compilation checks.
