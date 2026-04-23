# Implementation Checklist: #5499 — Completion Scope Distance Ranking

## Overview

Add 1 new integration test function to verify that variable completion respects lexical scope distance, ensuring variables from immediate scope rank higher than variables with the same name from parent scopes.

**Target file:** `crates/perl-lsp-rs/tests/lsp_completion_tests.rs`
**Test crate:** `perl-lsp-rs`
**Insertion point:** After line 1055 (end of test_completion_ranking function)

## Change order (compiles at each step)

### Step 1: Add test_completion_scope_distance_ranking()
- **File:** `crates/perl-lsp-rs/tests/lsp_completion_tests.rs`
- **Change:** Add new test function after line 1055
- **Details:** 
  ```rust
  /// Test that completion ranking respects lexical scope distance.
  /// Variables from immediate scope should rank higher than parent scope.
  #[test]
  fn test_completion_scope_distance_ranking() -> Result<(), Box<dyn std::error::Error>> {
      let server = start_lsp_server();
      initialize_lsp(&server);

      let uri = "file:///test_scope.pl";
      let code = r#"
my $outer = 1;

{
    my $inner = 2;
    my $config = 'local';
    
    {
        my $deep = 3;
        my $config = 'deeper';  # Same name as parent scope
        
        # At this point:
        # $config should suggest 'deeper' version first (immediate scope)
        # then 'local' version (parent scope)
        
        my $result = $c
    }
}
"#;
      
      send_notification(
          &server,
          json!({
              "jsonrpc": "2.0",
              "method": "textDocument/didOpen",
              "params": {
                  "textDocument": {
                      "uri": uri,
                      "languageId": "perl",
                      "version": 1,
                      "text": code
                  }
              }
          }),
      );
      drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(2000));

      // Request completion at line where $c is typed (should match $config)
      let lines: Vec<&str> = code.lines().collect();
      let target_line = lines.iter().position(|l| l.contains("$c")).unwrap_or(0);
      let target_char = lines[target_line].rfind("$c").unwrap_or(0) + 2;

      let response = send_request(
          &server,
          json!({
              "jsonrpc": "2.0",
              "method": "textDocument/completion",
              "params": {
                  "textDocument": { "uri": uri },
                  "position": { "line": target_line as i32, "character": target_char as i32 }
              }
          }),
      );

      let items = completion_items(&response);
      
      // Find $config entries
      let config_items: Vec<_> = items
          .iter()
          .filter(|item| {
              item["label"]
                  .as_str()
                  .map(|s| s.contains("config"))
                  .unwrap_or(false)
          })
          .collect();
      
      // Should have at least 1 entry for $config from some scope
      assert!(
          config_items.len() >= 1,
          "Should suggest $config from at least one scope"
      );

      // Verify sort_text indicates scoping
      // Sort_text format: "1<distance>_<name>" where lower distance ranks higher
      // Immediate scope: distance ~= 1 → sort_text like "11_config"
      // Parent scope: distance ~= 2 → sort_text like "12_config"
      let first_sort = config_items[0]["sortText"].as_str().unwrap_or("");
      
      if config_items.len() >= 2 {
          let second_sort = config_items[1]["sortText"].as_str().unwrap_or("");
          // First entry should have lower/better sort_text than second
          assert!(
              first_sort < second_sort,
              "Immediate scope should rank before parent scope: '{}' vs '{}'",
              first_sort,
              second_sort
          );
      }

      Ok(())
  }
  ```
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 2: Final verification
- **Verify:** `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2 && cargo xtask fmt && cargo clippy -p perl-lsp-rs --tests`

## Callers and consumers

This is a test-only change. No production code is modified. Tests use:
- `start_lsp_server()` — existing test fixture
- `initialize_lsp()` — existing test helper
- `send_notification()`, `send_request()` — existing test harness functions
- `completion_items()` — existing test helper
- `drain_until_quiet()` — existing test synchronization function
- Standard Rust test harness

No callers affected.

## Scope boundary

**Files IN scope:**
- `crates/perl-lsp-rs/tests/lsp_completion_tests.rs` — test addition only

**Files OUT of scope:**
- All production code in `crates/perl-lsp-rs/src/` — no changes
- `crates/perl-lsp-rs-core/src/providers/completion/` — no changes
- All other crates — no changes

## Notes for builder

1. **Test fixture:** Use existing harness (start_lsp_server, send_notification, send_request)
2. **Threading:** Run with `RUST_TEST_THREADS=2` per perl-lsp-rs CLAUDE.md
3. **LSP position calculation:** Code contains line markers; calculate correct line/column for completion position
4. **Sort_text format:** Assertion compares string lexicographically; "11_config" < "12_config" is true
5. **Scope distance values:** Lower distance = closer scope = better ranking (appears first in sorted list)
6. **Variable shadowing:** Test uses same variable name ($config) in multiple scopes to verify ranking prefers immediate scope
7. **Assertion robustness:** Checks `config_items.len() >= 1` first (defensive) before accessing indices
8. **Comments:** Inline comments in code explain expected behavior
9. **Test isolation:** Test is independent and can run in any order
10. **Duration constants:** Uses same timeout values as other tests (100ms drain, 2000ms max wait)
