# Bolt's Journal

This journal tracks critical performance learnings for the `tree-sitter-perl-rs` repository.

## 2024-05-22 - [Initial Setup]
**Learning:** Performance benchmarks are available in `crates/perl-parser/benches/`. The `ast_to_sexp` benchmark currently emits many parse errors, which might noise up the results.
**Action:** When running benchmarks, ensure valid input or filter out known noisy benchmarks if they obscure the target optimization.
