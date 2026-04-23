# Acceptance Criteria: #5498

- [ ] Three test functions added to `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs`
  - [ ] `test_functional_dap_function_breakpoints_with_condition()`
  - [ ] `test_functional_dap_function_breakpoints_scalar_condition()`
  - [ ] `test_functional_dap_function_breakpoints_complex_condition()`
- [ ] Each test function includes feature gate check (`has_feature("dap.breakpoints.function")`)
- [ ] Each test creates FunctionBreakpoint with condition field
- [ ] Each test calls `server.request("setFunctionBreakpoints", ...)` and gets response
- [ ] Each test asserts response has 1 breakpoint record
- [ ] Tests execute without panic or error
- [ ] All DAP tests pass: `cargo test -p perl-dap`
- [ ] No clippy warnings on new code: `cargo clippy -p perl-dap --tests`
- [ ] Code formatted correctly: `cargo xtask fmt`
- [ ] Tests positioned after line 358 and before logpoints section (line 360)
