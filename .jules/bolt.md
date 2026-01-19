## 2026-01-19 - String Allocation in AST Traversal
**Learning:** `find_current_package` allocated `String`s via `clone()` inside hot declaration search loops, causing excessive heap pressure.
**Action:** Prefer `Option<&str>` return types for AST traversal methods where the AST lifetime allows, avoiding unnecessary allocations.
