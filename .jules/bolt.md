# Bolt's Journal

This journal tracks critical performance learnings for the `tree-sitter-perl-rs` repository.

## 2026-01-21 - [Allocation in Hot Path Analysis]
**Learning:** `format!` in recursive AST traversal (like `ScopeAnalyzer`) causes significant memory churn. Checking properties of composite values (like variable sigil + name) should be done on components before allocation.
**Action:** When filtering or checking nodes in hot paths, pass references to components (`&str`, `&str`) instead of constructing owned `String`s just for the check.

## 2024-05-22 - [Initial Setup]
**Learning:** Performance benchmarks are available in `crates/perl-parser/benches/`. The `ast_to_sexp` benchmark currently emits many parse errors, which might noise up the results.
**Action:** When running benchmarks, ensure valid input or filter out known noisy benchmarks if they obscure the target optimization.

## 2026-01-23 - [Iterator Callback vs Vector Allocation]
**Learning:** Returning a temporary `Vec<_>` (e.g., `Vec<(String, usize)>`) from hot loops (like unused variable collection in `ScopeAnalyzer`) forces allocation even for items that will be immediately filtered out. Using an iterator pattern or callback (`for_each_reportable_unused_variable`) allows filtering *before* allocation.
**Action:** Prefer passing closures to inner scopes/loops instead of collecting results into temporary vectors, especially when filtering logic is available in the caller.

## 2024-05-22 - Optimizing ScopeAnalyzer Bareword Checks
**Learning:** Optimizing tree traversals vs string matches requires balancing common cases.
A simple swap to prioritize `is_known_function` (string match) over `is_in_hash_key_context` (tree traversal) slowed down benchmarks because checking the immediate parent (O(1) pointer access) for hash keys (`$hash{key}`) is faster than matching against a large list of built-ins.
**Action:** Use a hybrid approach: Check immediate parent (depth 1) first (fastest), then check `is_known_function` (fast), then check deeper ancestors (slow). This handles both common cases (hash keys and built-in functions) efficiently.

## 2026-05-24 - [Micro-optimizations in Hot Paths]
**Learning:** In hot paths like `is_builtin_global` (called for every variable), consolidating multiple string comparisons and length checks into single checks yields measurable gains (~3.5%). Also, iterating over `as_bytes()` avoids UTF-8 decoding overhead when checking for ASCII digits.
**Action:** For frequently called predicates on strings, prefer byte-level checks (`as_bytes()`) and combine logical conditions to minimize branches and length checks.

## 2026-06-01 - [Cross-Crate Closure Inlining Regression]
**Learning:** Replacing `node.children()` (allocates `Vec`) with `node.for_each_child(|child| recursive_fn(child))` caused a 45% regression in recursive AST traversal. This is likely due to the overhead of passing a large closure (capturing recursive state) to a non-inlined method in another crate. Adding `#[inline]` helped slightly but did not fully recover performance.
**Action:** Use specialized helpers like `first_child()` to avoid vector allocation for simple queries, but stick to vector iteration (which is cache-friendly and well-optimized) for full traversals unless the traversal method is guaranteed to be inlined and specialized.

## 2026-08-07 - [Regex Compilation in Loop]
**Learning:** Compiling a `Regex` inside a frequently called method (e.g., `extract_vars_from_string` for every interpolated string) is catastrophic for performance.
**Action:** Always use `std::sync::OnceLock` or `lazy_static` to compile regexes once. In this case, moving the regex to `OnceLock` improved interpolated string processing time by ~1000x (13.3s -> 12.6ms).
