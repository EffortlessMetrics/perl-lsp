## 2025-02-12 - Unused Security Controls and Test Isolation
**Vulnerability:** The `validate_path` function in `security.rs` was implemented and tested but *never used* in the `DebugAdapter` launch logic, allowing path traversal if `program` was absolute.
**Learning:** Security controls (like validators) must be explicitly hooked into the application logic. Grepping for usage of security functions is a good audit step. Also, existing tests (`dap_security_validation_tests`) were flaky because they used a hardcoded directory name (`test_workspace`) causing race conditions when running in parallel.
**Prevention:** Ensure all security utility functions have at least one usage in the main application code (dead code analysis can help). Use `tempfile` for filesystem tests to ensure isolation.
