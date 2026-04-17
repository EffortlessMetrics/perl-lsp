# Plan Review Findings — work-526911ca

## Overall Assessment
**feasible with modifications** — The plan correctly identifies the single bug and proposes a straightforward fix, but the test itself may be ineffective at verifying custom config behavior.

## Scope Assessment
**Scope is narrower than issue title implies** — The issue title "remove hardcoded absolute paths in test fixtures" suggests widespread action, but research+verification correctly determined only 1 actual bug exists (out of 133 hits). The 132 others are category (c) path validation tests where the path string IS the test subject. Scope is correctly scoped to 1 bug.

## What Works
1. **Bug identification is correct** — The hardcoded `/tmp/test.perltidyrc` at line 200 and cleanup at line 235 are genuine bugs (race condition, platform assumption).
2. **Dependencies are available** — `tempfile` crate is already a dev-dependency (`tempfile.workspace = true` in Cargo.toml line 152).
3. **Pattern exists in codebase** — Other tests in `crates/perl-lsp/tests/` use `tempfile::tempdir()` extensively (16 matches found).
4. **Fix approach is sound** — Using `tempfile::tempdir()` or `std::env::temp_dir()` follows established patterns and eliminates the race condition.
5. **Verification strategy is reasonable** — Running `cargo test -p perl-lsp-rs formatting_with_custom_config` is the correct way to verify the fix.

## What Doesn't Work

### Critical Issue: The Test's Effectiveness Is Questionable

The test `formatting_with_custom_config` creates a config file at `/tmp/test.perltidyrc` but **there is no mechanism for the LSP server to use this specific config file**:

- Line 204: `let uri = "file:///custom.pl";` — This is a bare file URI with no workspace context
- Lines 210-211: Comment explicitly states "The LSP server would need to support custom config paths / This test demonstrates the structure but may need server-side support"
- The server would look for `.perltidyrc` in the workspace or home directory, NOT at `/tmp/test.perltidyrc`

**The test creates a config file that is never actually used.** The assertions (lines 223-231) only check for "some formatting" occurring, not that the custom config was applied. This means:
- The test could pass even if the config file is never read
- The test could pass even if the tempfile approach is used incorrectly
- Fixing the hardcoded path is valuable for filesystem hygiene, but doesn't improve test coverage

### Secondary Issue: Early Cleanup

If the test fails before line 235 (e.g., at the assertion on line 230), the temp file is left behind. Using `tempfile::TempDir` or `NamedTempFile` with RAII cleanup would fix this automatically.

## Top Risks

### Risk 1: Fix Doesn't Actually Improve Test Quality
- **Likelihood**: high
- **Impact**: The test will still not verify custom config behavior; it will just use a tempfile instead of a hardcoded path
- **Mitigation**: The plan should verify whether the test actually exercises custom config or add a comment explaining why the tempfile is sufficient. Alternatively, the fix could include making the test actually use the custom config (e.g., by setting up a proper workspace).

### Risk 2: Test May Be Flaky Even After Fix
- **Likelihood**: medium
- **Impact**: If `tempfile` creates the file in a location the LSP server doesn't expect, the test could behave differently than before
- **Mitigation**: Verify the tempfile location is accessible and the server's auto-discovery would find `.perltidyrc` there, OR ensure the test explicitly configures the server to use the custom config path.

### Risk 3: Missing Import Statement
- **Likelihood**: low
- **Impact**: If the fix uses `tempfile::tempdir()` but doesn't add the import, the test won't compile
- **Mitigation**: Ensure the fix includes `use tempfile::tempdir;` or uses `std::env::temp_dir()` which requires no import.

## Edge Cases

1. **perltidy not installed** — The test already handles this gracefully (lines 186-189).
2. **Filesystem permissions** — If `/tmp` has unusual permissions, `std::env::temp_dir()` might return a different path, but the same applies to any temp solution.
3. **Temp file name collision** — Using `tempdir()` creates a unique directory, so multiple test runs won't collide.
4. **Windows path handling** — The test uses forward slashes in `file:///custom.pl` which is correct for URIs but `/tmp/test.perltidyrc` would be invalid on Windows. However, this test likely only runs on Unix systems.

## Recommendations

1. **[REQUIRED] Verify test effectiveness before/after fix**: Before applying the fix, run the test and confirm it passes. After applying the fix, run the test again and confirm it still passes. Document the behavior is unchanged (tempfile still gets the same formatting results).

2. **[REQUIRED] Add import statement**: If using `tempfile::tempdir()`, add `use tempfile::tempdir;` to the imports at the top of the file.

3. **[RECOMMENDED] Use `tempfile::TempDir` for automatic cleanup**: Instead of manual cleanup at line 235, use `let temp_dir = tempfile::tempdir()?; let config_path = temp_dir.path().join("test.perltidyrc");` and let RAII handle cleanup. This eliminates the early-exit cleanup gap.

4. **[OPTIONAL] Consider improving test coverage**: The test could be improved to actually verify custom config is applied by:
   - Setting up a workspace with the temp config as `.perltidyrc`
   - Or passing the config path explicitly to the LSP server (if supported)

   However, this is out of scope for the current issue which only addresses hardcoded paths.

## Confidence to Proceed
**medium** — The fix is straightforward and the dependencies are available, but there's uncertainty about whether the test actually validates custom config behavior. The fix itself is safe (just changes a hardcoded path to a tempfile), but the benefit is limited if the test doesn't actually use the config file.

To raise confidence: Run the test before and after the fix to confirm behavior is unchanged.
