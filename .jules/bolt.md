## 2026-01-21 - [Allocation in Hot Path Analysis]
**Learning:** `format!` in recursive AST traversal (like `ScopeAnalyzer`) causes significant memory churn. Checking properties of composite values (like variable sigil + name) should be done on components before allocation.
**Action:** When filtering or checking nodes in hot paths, pass references to components (`&str`, `&str`) instead of constructing owned `String`s just for the check.
