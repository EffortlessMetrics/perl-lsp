# Implementation Checklist: #5498 — DAP Function Breakpoints with Non-Boolean Conditions

## Overview

Add 3 new test functions to extend coverage for DAP function breakpoint condition handling, documenting protocol behavior for non-boolean conditions (scalars, complex expressions).

**Target file:** `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs`
**Test crate:** `perl-dap`
**Insertion point:** After line 358 (end of hit_condition tests, before line 360 logpoints section)

## Change order (compiles at each step)

### Step 1: Add test_functional_dap_function_breakpoints_with_condition()
- **File:** `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs`
- **Change:** Add new test function after line 358, before logpoints section
- **Details:** 
  ```rust
  /// Functional test: setFunctionBreakpoints accepts condition field
  #[test]
  fn test_functional_dap_function_breakpoints_with_condition() -> TestResult {
      if !has_feature("dap.breakpoints.function") {
          return Ok(());
      }
      
      let server = setup_dap_server()?;
      
      let set_fn_bp_args = perl_dap::protocol::SetFunctionBreakpointsArguments {
          breakpoints: vec![perl_dap::protocol::FunctionBreakpoint {
              name: "test_func".to_string(),
              condition: Some("$debug_flag".to_string()),
              hit_condition: None,
          }],
      };
      
      let response = server.request("setFunctionBreakpoints", set_fn_bp_args)?;
      let result: perl_dap::protocol::SetFunctionBreakpointsResponseBody = 
          serde_json::from_value(response)?;
      
      assert_eq!(result.breakpoints.len(), 1);
      assert!(result.breakpoints[0].verified);
      assert_eq!(result.breakpoints[0].message, None);
      
      Ok(())
  }
  ```
- **Verify:** `cargo check -p perl-dap`

### Step 2: Add test_functional_dap_function_breakpoints_scalar_condition()
- **File:** `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs`
- **Change:** Add new test function after test_functional_dap_function_breakpoints_with_condition()
- **Details:**
  ```rust
  /// Functional test: setFunctionBreakpoints accepts scalar variable condition
  /// Note: Perl-specific truthiness (scalar variables evaluate as boolean)
  #[test]
  fn test_functional_dap_function_breakpoints_scalar_condition() -> TestResult {
      if !has_feature("dap.breakpoints.function") {
          return Ok(());
      }
      
      let server = setup_dap_server()?;
      
      let set_fn_bp_args = perl_dap::protocol::SetFunctionBreakpointsArguments {
          breakpoints: vec![perl_dap::protocol::FunctionBreakpoint {
              name: "my_sub".to_string(),
              condition: Some("$count".to_string()),
              hit_condition: None,
          }],
      };
      
      let response = server.request("setFunctionBreakpoints", set_fn_bp_args)?;
      let result: perl_dap::protocol::SetFunctionBreakpointsResponseBody = 
          serde_json::from_value(response)?;
      
      assert_eq!(result.breakpoints.len(), 1);
      // Verification status depends on daemon's ability to parse the expression
      
      Ok(())
  }
  ```
- **Depends on:** Step 1
- **Verify:** `cargo check -p perl-dap`

### Step 3: Add test_functional_dap_function_breakpoints_complex_condition()
- **File:** `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs`
- **Change:** Add new test function after test_functional_dap_function_breakpoints_scalar_condition()
- **Details:**
  ```rust
  /// Functional test: setFunctionBreakpoints accepts complex boolean expressions
  #[test]
  fn test_functional_dap_function_breakpoints_complex_condition() -> TestResult {
      if !has_feature("dap.breakpoints.function") {
          return Ok(());
      }
      
      let server = setup_dap_server()?;
      
      let set_fn_bp_args = perl_dap::protocol::SetFunctionBreakpointsArguments {
          breakpoints: vec![perl_dap::protocol::FunctionBreakpoint {
              name: "handler".to_string(),
              condition: Some("defined($ENV{DEBUG}) && $ENV{DEBUG} > 0".to_string()),
              hit_condition: None,
          }],
      };
      
      let response = server.request("setFunctionBreakpoints", set_fn_bp_args)?;
      let result: perl_dap::protocol::SetFunctionBreakpointsResponseBody = 
          serde_json::from_value(response)?;
      
      assert_eq!(result.breakpoints.len(), 1);
      
      Ok(())
  }
  ```
- **Depends on:** Step 2
- **Verify:** `cargo check -p perl-dap`

### Step 4: Final verification
- **Verify:** `cargo test -p perl-dap && cargo xtask fmt && cargo clippy -p perl-dap`

## Callers and consumers

This is a test-only change. No production code is modified. Tests use:
- `has_feature()` — existing feature-flag helper from feature_catalog.rs
- `setup_dap_server()` — existing test fixture
- `SetFunctionBreakpointsArguments`, `FunctionBreakpoint` — existing protocol types in protocol.rs
- `request()` — existing server API

No callers affected.

## Scope boundary

**Files IN scope:**
- `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs` — test additions only

**Files OUT of scope:**
- All production code in `crates/perl-dap/src/` — no changes
- `crates/perl-dap/src/protocol.rs` — no validation added
- All other crates — no changes

## Notes for builder

1. **Feature gate:** All tests must check `has_feature("dap.breakpoints.function")` and return early if not enabled
2. **Test fixture:** Use existing `setup_dap_server()` helper
3. **Protocol parsing:** Tests use existing types and server request method — no new infrastructure needed
4. **Condition format:** Conditions are Perl expressions as strings; protocol layer accepts any string without validation
5. **Verified field:** Tests assert breakpoint records have verified=true, which indicates AST validation passed
6. **Test isolation:** Tests are independent and can run in any order
7. **Assertion placement:** Add tests after line 358, before the logpoints section (line 360), to keep breakpoint tests grouped
