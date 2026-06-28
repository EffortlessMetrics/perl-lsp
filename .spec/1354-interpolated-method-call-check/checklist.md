# Implementation Checklist: #1354 — Parser: Interpolated string delimiter check incorrectly flags method calls

## Change order (compiles at each step)

### Step 1: Remove method-call paren-balancing check for `->identifier(`
- **File:** `crates/perl-parser-core/src/engine/parser/expressions/primary.rs`
- **Change:** Delete the bare method call check block (lines 122–134)
- **Details:** Remove the entire block that scans for `->identifier` followed by `(` and attempts to balance parentheses. In Perl, `$obj->method()` is never interpolated in double-quoted strings, so this check is incorrect.
  ```rust
  // REMOVE THIS BLOCK (lines 122-134):
  // bare method name: $obj->method(...) — scan the name, then check for (
  if i < quote_end && Self::is_identifier_start(bytes[i]) {
      while i < quote_end && Self::is_identifier_continue(bytes[i]) {
          i += 1;
      }
      if i < quote_end && bytes[i] == b'(' {
          if !Self::consume_balanced_in_interpolated_string(
              bytes, i, b'(', b')', quote_end,
          ) {
              return Some('(');
          }
          continue;
      }
  }
  ```
- **Verify:** `cargo check -p perl-parser-core`

### Step 2: Remove direct paren-balancing check for `->`
- **File:** `crates/perl-parser-core/src/engine/parser/expressions/primary.rs`
- **Change:** Delete the direct `->( )` check block (lines 113–120)
- **Details:** Remove the block that checks for `->` followed directly by `(`. In Perl, only `->{}` and `->[]` are valid interpolation boundaries. Direct `->()` is not interpolated.
  ```rust
  // REMOVE THIS BLOCK (lines 113-120):
  if i < quote_end && bytes[i] == b'(' {
      if !Self::consume_balanced_in_interpolated_string(
          bytes, i, b'(', b')', quote_end,
      ) {
          return Some('(');
      }
      continue;
  }
  ```
- **Depends on:** Logically independent, but delete in order to avoid line number shifts.
- **Verify:** `cargo check -p perl-parser-core`

### Step 3: Add regression test
- **File:** `crates/perl-parser-core/tests/test_interpolated_method_call_1354.rs` (CREATE)
- **Change:** Add comprehensive test cases covering method calls in interpolated strings
- **Details:** Copy the test file already created during verification. This ensures:
  - Simple method calls parse cleanly: `"$obj->method()"`
  - Method calls with arguments parse cleanly: `"$obj->foo(bar, baz)"`
  - Chained method calls parse cleanly: `"$x->method1()->method2()"`
  - The specific DBI.pm line 785 case from the issue parses cleanly
  - Valid interpolation boundaries (`->[]`, `->{}`) still work correctly
- **Verify:** `cargo test -p perl-parser-core --test test_interpolated_method_call_1354`

### Step 4: Final verification
- **Verify:** 
  ```bash
  cargo test -p perl-parser-core
  cargo xtask fmt
  cargo clippy -p perl-parser-core
  ```

## Callers and consumers

- `find_unclosed_interpolation_delimiter` is called from:
  - `crates/perl-parser-core/src/engine/parser/expressions/primary.rs:230` (in `parse_primary_inner`, called when parsing double-quoted strings)
  
- No other code calls or depends on this function; it's internal to the parser.

## Scope boundary

Files IN scope:
- `crates/perl-parser-core/src/engine/parser/expressions/primary.rs` (deletion only, no additions)
- `crates/perl-parser-core/tests/test_interpolated_method_call_1354.rs` (new test file)

Files OUT of scope:
- All other parser files
- All LSP/DAP files
- All other crates
- Configuration, build system, docs

## Flags for builder

1. **Line numbers may shift slightly** after first deletion. The second deletion should account for this. Verify with `cargo check` after each step.
2. **Test file already exists** in the worktree from verification phase. Builder should keep it as-is or rebuild it to match the spec.
3. **No API changes**: This is a pure bug fix removing incorrect validation logic. No new public types, functions, or exports.
4. **No cascading changes**: The function signature and error message remain unchanged; only the logic changes. All error reporting paths are preserved.
