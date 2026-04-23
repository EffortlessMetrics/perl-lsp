# Implementation Checklist: #5496 — Parser Unclosed Delimiter Recovery

## Overview

Add 4 new error recovery tests to verify that the parser gracefully handles unclosed string delimiters (qw, q, qq) without panicking and produces valid AST with error records.

**Target file:** `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs`
**Test crate:** `perl-parser-core`
**Insertion point:** After line 493 (end of file)

## Change order (compiles at each step)

### Step 1: Add test_recovery_unclosed_qw()
- **File:** `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs`
- **Change:** Add new test function after line 493
- **Details:** 
  ```rust
  #[test]
  fn test_recovery_unclosed_qw() {
      let code = "my @items = qw(one two three print 1;";
      let mut parser = Parser::new(code);
      let result = parser.parse();
      
      assert!(result.is_ok(), "Parser should recover from unclosed qw()");
      let ast = must(result);
      
      if let NodeKind::Program { statements } = &ast.kind {
          assert!(statements.len() >= 1, "Should have recovered statements after unclosed qw");
      }
      
      assert!(!parser.errors().is_empty(), "Should record unclosed delimiter error");
  }
  ```
- **Verify:** `cargo check -p perl-parser-core`

### Step 2: Add test_recovery_unclosed_q_brace()
- **File:** `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs`
- **Change:** Add new test function after test_recovery_unclosed_qw()
- **Details:**
  ```rust
  #[test]
  fn test_recovery_unclosed_q_brace() {
      let code = "my $str = q{ hello world print 1;";
      let mut parser = Parser::new(code);
      let result = parser.parse();
      
      assert!(result.is_ok(), "Parser should recover from unclosed q{}");
      let ast = must(result);
      
      if let NodeKind::Program { statements } = &ast.kind {
          assert!(statements.len() >= 1, "Should have recovered statements");
      }
      
      assert!(!parser.errors().is_empty(), "Should record unclosed brace error");
  }
  ```
- **Depends on:** Step 1
- **Verify:** `cargo check -p perl-parser-core`

### Step 3: Add test_recovery_unclosed_qq()
- **File:** `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs`
- **Change:** Add new test function after test_recovery_unclosed_q_brace()
- **Details:**
  ```rust
  #[test]
  fn test_recovery_unclosed_qq() {
      let code = "my $name = \"unknown; print 1;";
      let mut parser = Parser::new(code);
      let result = parser.parse();
      
      assert!(result.is_ok(), "Parser should recover from unclosed qq string");
      assert!(!parser.errors().is_empty(), "Should record unclosed quote error");
  }
  ```
- **Depends on:** Step 2
- **Verify:** `cargo check -p perl-parser-core`

### Step 4: Add test_recovery_nested_qw_paren_mismatch()
- **File:** `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs`
- **Change:** Add new test function after test_recovery_unclosed_qq()
- **Details:**
  ```rust
  #[test]
  fn test_recovery_nested_qw_paren_mismatch() {
      let code = "my @list = qw(one (two three) print 1;";
      let mut parser = Parser::new(code);
      let result = parser.parse();
      
      assert!(result.is_ok(), "Parser should recover from nested paren in qw");
      assert!(!parser.errors().is_empty(), "Should record delimiter mismatch error");
  }
  ```
- **Depends on:** Step 3
- **Verify:** `cargo check -p perl-parser-core`

### Step 5: Final verification
- **Verify:** `cargo test -p perl-parser-core && cargo xtask fmt && cargo clippy -p perl-parser-core`

## Callers and consumers

This is a test-only change. No production code is modified. Tests use:
- `Parser::new()` — existing public API
- `parser.parse()` — existing public API
- `parser.errors()` — existing public API
- `must()` from `perl_tdd_support` — existing test helper
- `NodeKind::Program` — existing enum variant

No callers affected.

## Scope boundary

**Files IN scope:**
- `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs` — test additions only

**Files OUT of scope:**
- All production code in `crates/perl-parser-core/src/` — no changes
- All other crates — no changes
- `crates/perl-parser-core/tests/` — separate test directory (not modified)

## Notes for builder

1. **Test isolation:** Each test is independent and can run in any order
2. **Parser API stability:** All APIs used (Parser::new, parse, errors) are public and stable
3. **Error collection:** Parser must record at least one error for each unclosed delimiter case — verify with `parser.errors()` assertion
4. **AST structure:** Tests follow existing pattern from test_recovery_missing_expression — use pattern matching on NodeKind::Program
5. **No mocking:** Tests use real Parser, no mocks or fixtures needed
6. **Escape sequences:** Code strings use raw strings (r#"..."#) where needed to avoid escaping issues; single quotes for simple cases
7. **Assertion messages:** Each assertion includes a descriptive message for debugging
