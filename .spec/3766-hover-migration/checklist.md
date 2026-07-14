# Implementation Checklist: #3766 — Migrate hover to generation-owned analyzer/type_environment

## Summary

Migrate the hover provider from LspServer-level `(uri, content_hash)`-keyed caches to generation-owned lazy cells on `ParsedSnapshot`. This makes hover freshness-correct (generation-only, not hash-only) and constructor-efficient (one analyzer per generation, not per request). Follows the pattern established by completion in PR #3765.

## Change order (compiles at each step)

### Step 1: Update hover.rs line 220 — migrate analyzer read to snapshot method
- **File:** `crates/perl-lsp-rs/src/runtime/language/hover.rs`
- **Change:** Replace `self.get_or_build_analyzer(uri, text, ast)` with `parsed.semantic_analyzer()`
- **Details:**
  - Current code (line 220): `let analyzer = self.get_or_build_analyzer(uri, text, ast);`
  - New code: `let analyzer = parsed.semantic_analyzer();`
  - The `parsed: Arc<ParsedSnapshot>` is already available from line 79 (`doc.current_parsed()`)
  - Change is safe: both methods return `Arc<SemanticAnalyzer>` (snapshot version is wrapped in Option, but is called after AST guard on line 82)
  - Extract: Change line 220 in `extract_symbol_hover` method from calling `self.get_or_build_analyzer(uri, text, ast)` to calling `parsed.semantic_analyzer()` inside the `if let Some(ast) = parsed.as_ref().and_then(|p| p.ast())` block
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 2: Update hover.rs line 377 — migrate type engine read to snapshot method
- **File:** `crates/perl-lsp-rs/src/runtime/language/hover.rs`
- **Change:** Replace `self.get_or_build_type_engine(uri, text, ast)` with `parsed.type_environment()`
- **Details:**
  - Current code (line 377): `let type_engine = self.get_or_build_type_engine(uri, text, ast);`
  - New code: `let type_engine = parsed.type_environment();`
  - Same pattern as Step 1
  - The type_engine is already Option-wrapped (snapshot method returns Option<Arc<...>>), and the call is inside the variable-kind guard on line 375
- **Depends on:** Step 1 (logical dependency, both in same function, same guard block)
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 3: Remove get_or_build_analyzer method from document_access.rs
- **File:** `crates/perl-lsp-rs/src/runtime/document_access.rs`
- **Change:** Delete the `get_or_build_analyzer` method entirely
- **Details:**
  - Find the method definition (search for "pub fn get_or_build_analyzer")
  - Delete the entire method (approximately 20-30 lines)
  - Rationale: hover was the only consumer (verified via grep)
- **Depends on:** Step 1 (must update caller first)
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 4: Remove get_or_build_type_engine method from document_access.rs
- **File:** `crates/perl-lsp-rs/src/runtime/document_access.rs`
- **Change:** Delete the `get_or_build_type_engine` method entirely
- **Details:**
  - Find the method definition (search for "pub fn get_or_build_type_engine")
  - Delete the entire method
  - Rationale: hover was the only consumer
- **Depends on:** Step 2
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 5: Remove semantic_analyzer_cache field from LspServer
- **File:** `crates/perl-lsp-rs/src/runtime/mod.rs`
- **Change:** Delete the `semantic_analyzer_cache` field from the LspServer struct
- **Details:**
  - Find the field definition (search for "semantic_analyzer_cache" in the LspServer struct)
  - Delete the entire field declaration
  - Type: `DashMap<(String, u64), Arc<SemanticAnalyzer>>` or similar
  - Also remove any related initialization in `LspServer::new()` or other constructors
- **Depends on:** Steps 3-4
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 6: Remove type_inference_engine_cache field from LspServer
- **File:** `crates/perl-lsp-rs/src/runtime/mod.rs`
- **Change:** Delete the `type_inference_engine_cache` field from the LspServer struct
- **Details:**
  - Find the field definition (search for "type_inference_engine_cache" in the LspServer struct)
  - Delete the entire field declaration
  - Type: `DashMap<(String, u64), Arc<TypeInferenceEngine>>` or similar
  - Also remove any related initialization
- **Depends on:** Steps 3-4
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 7: Remove cache entries from MemoryStateSnapshot
- **File:** `crates/perl-lsp-rs/src/runtime/mod.rs`
- **Change:** Delete cache-related fields from any `MemoryStateSnapshot` struct or similar metrics structure
- **Details:**
  - Search for "semantic_analyzer_cache_size" or similar metrics fields
  - These are accounting/observability fields used for memory profiling
  - Delete the field and any corresponding population code
- **Depends on:** Steps 5-6
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 8: Remove didChange cache invalidation from text_sync.rs
- **File:** `crates/perl-lsp-rs/src/runtime/text_sync.rs`
- **Change:** Remove the eviction/invalidation blocks that clear cache entries on document change
- **Details:**
  - Search for cache invalidation in `handle_did_change` or related methods
  - Look for patterns like `self.evict_open_document_session_state(uri)` or `semantic_analyzer_cache.remove(...)`
  - Delete these invalidation blocks entirely
  - The new snapshot-based approach needs no explicit invalidation (old snapshots naturally age out as generation bumps)
- **Depends on:** Steps 5-6
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 9: Remove cache-related tests from text_sync/tests.rs
- **File:** `crates/perl-lsp-rs/src/runtime/text_sync/tests.rs` (if exists) or `crates/perl-lsp-rs/tests/`
- **Change:** Delete tests that specifically verify cache invalidation or cache behavior
- **Details:**
  - Search for tests that mention "cache" or "semantic_analyzer_cache" or "type_inference_engine_cache"
  - Delete these tests entirely (they test an implementation detail that no longer exists)
  - Keep tests that verify hover behavior itself (those should still pass)
- **Depends on:** Steps 7-8
- **Verify:** `cargo check -p perl-lsp-rs`

### Step 10: Add generation round-trip test
- **File:** `crates/perl-lsp-rs/tests/hover_generation_roundtrip.rs` (CREATE if new)
- **Change:** Add a test that proves hover reflects a NEW generation's facts after editing
- **Details:**
  - Test: Open document `my $x = 42;` → hover on `$x` → see type or docs
  - Edit to `my $x = "hello";` (bumps generation)
  - Hover again on `$x` → see NEW type, not old cached type
  - Assert that results differ between generations
  - Use insta snapshots or manual assertion to prove the change is observed
  - This directly tests the "generation round-trip" acceptance criterion
- **Depends on:** Steps 1-9
- **Verify:** `cargo test -p perl-lsp-rs --test hover_generation_roundtrip`

### Step 11: Add pending-parse-gap test
- **File:** `crates/perl-lsp-rs/tests/hover_pending_parse.rs` (CREATE if new)
- **Change:** Add a test that verifies hover during a pending parse returns honest (not stale-wrong) results
- **Details:**
  - Arm a cancellation flag that fires during parse (difficult to time precisely)
  - Request hover while generation N+1 parse is in flight (not yet published)
  - Assert: hover returns EITHER results from generation N (last-published) OR degraded/pending placeholder, NEVER stale-wrong from a prior generation
  - Alternatively: mock the parse flow to delay publication and verify hover doesn't return stale lies
  - This tests the "pending-parse-gap honesty" acceptance criterion
- **Depends on:** Steps 1-9
- **Verify:** `cargo test -p perl-lsp-rs --test hover_pending_parse`

### Step 12: Add fidelity preservation test
- **File:** `crates/perl-lsp-rs/tests/hover_fidelity.rs` (CREATE if new)
- **Change:** Add a test that verifies hover uses real source text (not empty-source overload)
- **Details:**
  - Test case: Hover on a variable that has a POD comment above it
  - Assert: Hover includes the documentation from the POD (proves source is present, not empty)
  - Test case: Hover on a symbol in a narrow text range
  - Assert: Hover's text range is precise, not a broad fallback (proves real source enables precise lookup, not empty-source guess)
  - This tests the "fidelity preservation" acceptance criterion
  - Note: Don't regress to empty-source `SemanticAnalyzer::analyze(ast)` overload; always use `snapshot.semantic_analyzer()` which carries real source
- **Depends on:** Steps 1-9
- **Verify:** `cargo test -p perl-lsp-rs --test hover_fidelity`

### Step 13: Add construction-count test using snapshot's test-only methods
- **File:** `crates/perl-lsp-rs/tests/hover_construction_count.rs` (CREATE if new)
- **Change:** Add a test that verifies hover constructs analyzer/type-engine exactly once per generation
- **Details:**
  - Requires feature `expose_lsp_test_api` (see perl-lsp-rs/CLAUDE.md)
  - Test: Hover on document generation N at 3 different offsets
  - Assert: `parsed.semantic_analyzer_initialized()` is true (lazy cell fired once)
  - Assert: `parsed.semantic_analyzer_build_count()` == 1 (constructed exactly once, not 3 times)
  - Repeat for `type_environment` via `type_environment_initialized()` and `type_environment_build_count()`
  - Edit document (generation N+1) and repeat
  - Assert: new snapshot has its own 1x construction, doesn't reuse generation N's
  - This tests the "construction-count" acceptance criterion (mirrors #3765's pattern)
  - Test must be `#[cfg(any(test, feature = "expose_lsp_test_api"))]` to compile only with feature
- **Depends on:** Steps 1-9
- **Verify:** `cargo test -p perl-lsp-rs --test hover_construction_count --features expose_lsp_test_api`

### Step 14: Final verification
- **Verify all:**
  - `cargo test -p perl-lsp-rs --lib` (all unit tests pass)
  - `cargo clippy -p perl-lsp-rs --locked -- -D warnings -A missing_docs` (no clippy warnings)
  - `cargo xtask fmt` (formatted)
  - `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` (full test suite with threading constraints per CLAUDE.md)

## Consumers and call sites

**Consumers of old cache methods (verified via grep):**
- `get_or_build_analyzer`: Called only from `hover.rs:220` (in `extract_symbol_hover` method)
- `get_or_build_type_engine`: Called only from `hover.rs:377` (in `extract_symbol_hover` method)

**Result:** Hover is the ONLY consumer. Safe to delete caches entirely after migration.

## Scope boundary

**Files IN scope:**
- `crates/perl-lsp-rs/src/runtime/language/hover.rs` (2 line changes: 220, 377)
- `crates/perl-lsp-rs/src/runtime/document_access.rs` (remove 2 methods)
- `crates/perl-lsp-rs/src/runtime/mod.rs` (remove 2 cache fields + initializations + metrics)
- `crates/perl-lsp-rs/src/runtime/text_sync.rs` (remove cache invalidation blocks)
- `crates/perl-lsp-rs/src/runtime/text_sync/tests.rs` (remove cache-specific tests)
- `crates/perl-lsp-rs/tests/` (add 4 new integration test files)

**Files OUT of scope (DO NOT CHANGE):**
- References/rename providers (separate follow-up slices per task framing)
- Completion provider (already migrated in #3765)
- Parser, AST, semantic analyzer crates (no changes needed)
- Other LSP providers (diagnostics, code_actions, etc.)

## Flags for builder

1. **Exact line numbers**: grep for `get_or_build_analyzer` to find line 220, `get_or_build_type_engine` for line 377. Lines may have drifted since issue filed.
2. **Optional call site**: The snapshot is available as `parsed` (type `Arc<ParsedSnapshot>`), but it's Option-wrapped. Callers after line 82's `if let Some(ast) = parsed.as_ref()...` can safely call `.semantic_analyzer()` and `.type_environment()` (both return Option; handle the None case gracefully).
3. **Cache retirement condition**: VERIFY no other caller of the two cache methods exists before deleting them. Use `cargo grep` or manual review to double-check.
4. **No new public API**: The change is purely internal. `ParsedSnapshot.semantic_analyzer()` and `type_environment()` already exist from #3765 — this just consumes them.
5. **Test feature gate**: Construction-count test (Step 13) requires `expose_lsp_test_api` feature. See `crates/perl-lsp-rs/CLAUDE.md` for threading constraints.
6. **Documentation in hover.rs**: The doc comment on line 197 says "Uses `get_or_build_analyzer` so repeated hovers on the same document version share a single cached...". Update this comment to explain the snapshot-based approach instead.
