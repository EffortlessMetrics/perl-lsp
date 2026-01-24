# Bolt's Journal

This journal tracks critical performance learnings for the `tree-sitter-perl-rs` repository.

## 2026-01-21 - [Allocation in Hot Path Analysis]
**Learning:** `format!` in recursive AST traversal (like `ScopeAnalyzer`) causes significant memory churn. Checking properties of composite values (like variable sigil + name) should be done on components before allocation.
**Action:** When filtering or checking nodes in hot paths, pass references to components (`&str`, `&str`) instead of constructing owned `String`s just for the check.

## 2026-05-24 - [Optimizing Scope Lookups]
**Learning:** `ScopeAnalyzer` allocates a `String` for the sigil key in `HashMap<String, ...>` for every variable declaration and lookup. Since Perl has a small, fixed set of sigils (`$`, `@`, `%`, `&`, `*`), using a fixed-size array `[HashMap<...>; 6]` indexed by sigil type eliminates these allocations.
**Action:** Replace `HashMap` keyed by small enums or fixed sets with arrays or `Vec` indexed by integer representation of the key to avoid hashing and allocation.

## 2024-05-22 - [Initial Setup]
**Learning:** Performance benchmarks are available in `crates/perl-parser/benches/`. The `ast_to_sexp` benchmark currently emits many parse errors, which might noise up the results.
**Action:** When running benchmarks, ensure valid input or filter out known noisy benchmarks if they obscure the target optimization.
