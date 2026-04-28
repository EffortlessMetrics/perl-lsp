# Specs: Wire @INC Search Paths into PullDiagnosticsProvider

## Feature/Behavior Description

The `PullDiagnosticsProvider::get_document_diagnostics` convenience method will accept an optional `include_paths` parameter. When provided, the resulting `PullDiagnosticsContext` will include those paths, enabling PL701 (ModuleNotFound) diagnostics to display the searched @INC paths in their message.

### Current Behavior
- `get_document_diagnostics(uri, content, previous_result_id)` creates a context with `include_paths: Vec::new()`
- PL701 diagnostics show "Module 'X' not found" without listing searched paths
- Tests cannot verify the @INC path inclusion because they cannot pass paths to the convenience method

### New Behavior
- `get_document_diagnostics(uri, content, previous_result_id, None)` unchanged (backward compatible)
- `get_document_diagnostics(uri, content, previous_result_id, Some(vec!["/path".to_string()]))` creates context with those paths
- PL701 diagnostics include searched paths in message: "Module 'X' not found in: /path1, /path2"

## Acceptance Criteria

### AC1: Backward Compatibility
All existing test call sites that use `get_document_diagnostics(uri, content, previous_result_id)` (4 parameters) continue to compile and pass without modification.

### AC2: PL701 Includes Search Paths
When `include_paths: Some(paths)` is provided, the PL701 diagnostic message includes the searched paths in its message body.

### AC3: New Test Coverage
A new test `pl701_pull_diagnostics_includes_inc_paths` exists in `crates/perl-lsp/tests/pull_diagnostics_tests.rs` that:
- Uses a known missing module (e.g., `Missing::Module`)
- Provides include_paths with test paths
- Verifies the PL701 diagnostic message includes the provided paths

## Non-Goals

- This fix does NOT modify production call sites — they already work correctly using `get_document_diagnostics_with_context`
- This fix does NOT address `get_workspace_diagnostics` which has the same pattern (out of scope per issue)
- This fix does NOT add perlcritic integration with include_paths for PL701 (future work)

## Dependencies

- `perl-lsp-diagnostics` crate's PL701 (missing_module lint) already accepts and uses `search_paths`
- `PullDiagnosticsProvider::collect_diagnostics_for_text_with_context` already passes `context.include_paths` to diagnostics
- No new dependencies required
