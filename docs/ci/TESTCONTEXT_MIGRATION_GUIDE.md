# TestContext → LspHarness Migration Guide

> **Purpose**: Fix 688 ignored tests by migrating from broken `TestContext` to working `LspHarness`.
> **Last Updated**: 2025-12-27

## The Problem

The `TestContext` pattern (found in `lsp_comprehensive_e2e_test.rs` and similar files) has a race condition:

```rust
// BROKEN PATTERN (TestContext)
let mut ctx = TestContext::new();
ctx.initialize();                    // ❌ No barrier after init!
ctx.open_document(uri, text);        // ❌ Race with server startup
let result = ctx.send_request(...);  // ❌ Server may not be ready
```

This causes "BrokenPipe" errors because:
1. The server thread hasn't finished processing `initialize`
2. No wait for `initialized` notification to be acknowledged
3. No synchronization barrier before requests

## The Solution

The `LspHarness` (in `tests/support/lsp_harness.rs`) already solves this:

```rust
// WORKING PATTERN (LspHarness)
let mut harness = LspHarness::new_raw();
harness.initialize_with_root(root_uri, None)?;  // ✅ Includes barrier!
harness.open(uri, text)?;                        // ✅ Safe after barrier
let result = harness.request(method, params)?;   // ✅ Adaptive timeout
```

## Migration Steps

### Step 1: Replace TestContext with LspHarness

**Before:**
```rust
use crate::support::test_context::TestContext;

#[test]
#[ignore] // Flaky BrokenPipe errors...
fn test_some_feature() {
    let mut ctx = TestContext::new();
    let response = ctx.initialize();

    ctx.open_document("file:///test.pl", "my $x = 1;");

    let result = ctx.send_request("textDocument/hover", Some(json!({
        "textDocument": { "uri": "file:///test.pl" },
        "position": { "line": 0, "character": 4 }
    })));

    assert!(result.is_some());
}
```

**After:**
```rust
mod support;
use support::lsp_harness::LspHarness;

#[test]
fn test_some_feature() {  // ✅ Remove #[ignore]
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root("file:///test", None)
        .expect("initialization should succeed");

    harness.open("file:///test.pl", "my $x = 1;")
        .expect("open should succeed");

    let result = harness.request("textDocument/hover", json!({
        "textDocument": { "uri": "file:///test.pl" },
        "position": { "line": 0, "character": 4 }
    })).expect("hover request should succeed");

    // Result is already the unwrapped "result" field
    assert!(!result.is_null());
}
```

### Step 2: Use Proper Error Handling

**TestContext** returned `Option<Value>` which hid errors.
**LspHarness** returns `Result<Value, String>` which surfaces problems.

```rust
// Handle errors properly
match harness.request("textDocument/definition", params) {
    Ok(result) => {
        // Process result
    }
    Err(e) => {
        panic!("Request failed: {}", e);
    }
}

// Or use expect for simpler tests
let result = harness.request("textDocument/definition", params)
    .expect("definition request failed");
```

### Step 3: Add Explicit Barriers for Complex Workflows

For tests that do multiple operations, add barriers:

```rust
harness.open("file:///a.pl", content_a)?;
harness.open("file:///b.pl", content_b)?;
harness.barrier();  // ✅ Ensure all documents processed

// Now safe to query workspace-level features
let symbols = harness.request("workspace/symbol", json!({"query": "foo"}))?;
```

### Step 4: Use Adaptive Timeouts for Slow Operations

```rust
// Default timeout is adaptive (200-800ms based on thread count)
let result = harness.request("textDocument/completion", params)?;

// For known slow operations, use explicit timeout
let result = harness.request_with_timeout(
    "workspace/symbol",
    json!({"query": ""}),
    Duration::from_secs(5)
)?;
```

### Step 5: Use TempWorkspace for File-Based Tests

```rust
// Create workspace with real files
let (mut harness, workspace) = LspHarness::with_workspace(&[
    ("lib/Foo.pm", "package Foo;\nsub bar { 1 }\n1;"),
    ("main.pl", "use Foo;\nFoo::bar();"),
])?;

// Use workspace URIs
let uri = workspace.uri("main.pl");
let result = harness.request("textDocument/definition", json!({
    "textDocument": { "uri": uri },
    "position": { "line": 1, "character": 5 }
}))?;
```

## API Mapping

| TestContext | LspHarness | Notes |
|-------------|------------|-------|
| `TestContext::new()` | `LspHarness::new_raw()` | Raw constructor |
| `ctx.initialize()` | `harness.initialize_with_root(uri, caps)?` | Returns Result, includes barrier |
| `ctx.send_request(method, params)` | `harness.request(method, params)?` | Returns Result, adaptive timeout |
| `ctx.send_notification(method, params)` | `harness.notify(method, params)` | Same behavior |
| `ctx.open_document(uri, text)` | `harness.open(uri, text)?` | Returns Result |
| `ctx.update_document(uri, text)` | Manual via `notify("textDocument/didChange", ...)` | |
| N/A | `harness.barrier()` | **Use this!** Sync point |
| N/A | `harness.wait_for_idle(duration)` | Drain notifications |
| N/A | `harness.shutdown_gracefully()` | Called automatically on drop |

## Batch Migration Script

For files with many tests, use this sed pattern:

```bash
# Replace TestContext::new() with LspHarness::new_raw()
sed -i 's/TestContext::new()/LspHarness::new_raw()/g' file.rs

# Replace ctx.initialize() with proper pattern
sed -i 's/ctx\.initialize()/harness.initialize_with_root("file:\/\/\/test", None).expect("init")/g' file.rs

# Remove #[ignore] lines with BrokenPipe reason
sed -i '/#\[ignore\].*BrokenPipe/d' file.rs
```

## Validation

After migrating a file:

```bash
# Run the specific test file
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2

# If tests pass, commit and move to next file
# If tests fail, check for:
# 1. Missing barriers after open_document calls
# 2. Tests that need longer timeouts
# 3. Tests that have real bugs (now surfaced!)
```

## Common Pitfalls

### 1. Missing mod declaration

```rust
// Add at top of test file if not present
mod support;
use support::lsp_harness::LspHarness;
```

### 2. Wrong import path

```rust
// Some files use different paths
use crate::support::lsp_harness::LspHarness;  // If in same crate
// vs
mod support;
use support::lsp_harness::LspHarness;  // If support is sibling
```

### 3. Forgetting to handle Results

```rust
// WRONG - ignores errors
let _ = harness.request(...);

// RIGHT - handles errors
let result = harness.request(...)?;  // In function returning Result
let result = harness.request(...).expect("should work");  // In test
```

### 4. Not using barrier for workspace operations

```rust
// WRONG - race condition
harness.open("file:///a.pl", content)?;
let symbols = harness.request("workspace/symbol", ...)?;  // May miss a.pl

// RIGHT - synchronized
harness.open("file:///a.pl", content)?;
harness.barrier();  // Ensure indexing complete
let symbols = harness.request("workspace/symbol", ...)?;
```

## Progress Checklist

### Priority 1 (Phase 1)
- [ ] `lsp_comprehensive_3_17_test.rs` (59 tests)
- [ ] `lsp_comprehensive_e2e_test.rs` (33 tests)

### Priority 2 (Phase 2)
- [ ] `lsp_protocol_violations.rs` (26 tests)
- [ ] `lsp_execute_command_comprehensive_tests.rs` (25 tests)

### Priority 3 (Phase 3)
- [ ] `lsp_advanced_features_test.rs` (23 tests)
- [ ] `lsp_window_progress_test.rs` (21 tests)
- [ ] `lsp_error_recovery_behavioral_tests.rs` (21 tests)

*Continue with remaining files per IGNORED_TESTS_INDEX.md*
