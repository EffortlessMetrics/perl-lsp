# ADR: Wire @INC Search Paths into PullDiagnosticsProvider

**Status:** Proposed

## Context

Issue #4259 is a follow-up to PR #4249 which enhanced PL701 (ModuleNotFound) diagnostics to include `@INC` search paths in the diagnostic message. The issue identifies that `PullDiagnosticsProvider::get_document_diagnostics` creates an empty `PullDiagnosticsContext` (with `include_paths: Vec::new()`), meaning PL701 diagnostics from the pull-diagnostics code path never include @INC context in tests.

### Key Code Facts

1. **`PullDiagnosticsProvider::get_document_diagnostics`** (lines 128-136) is a convenience method that creates `PullDiagnosticsContext::new()` with empty `include_paths`.

2. **Production code is already correct** — `runtime/diagnostics.rs` calls `get_document_diagnostics_with_context` directly with a properly-built context containing `include_paths` (computed via `server.include_paths_for_doc(uri)`).

3. **Tests use the convenience method** — `crates/perl-lsp/tests/pull_diagnostics_tests.rs` uses `provider.get_document_diagnostics(&uri, content, None)` which cannot pass include_paths.

4. **`PullDiagnosticsProvider` is stateless** — It's just `Self` by design, intended to be free of LspServer dependencies for testability.

## Decision

Add an **optional** `include_paths: Option<Vec<String>>` parameter to `PullDiagnosticsProvider::get_document_diagnostics`:

```rust
pub fn get_document_diagnostics(
    &self,
    uri: &Uri,
    content: &str,
    previous_result_id: Option<String>,
    include_paths: Option<Vec<String>>,  // NEW: optional parameter
) -> DocumentDiagnosticReport
```

When `Some(paths)` is provided, build a context with those paths; when `None`, use empty paths (backward compatible).

## Consequences

### Benefits
- Tests can now verify PL701 diagnostic messages include @INC paths
- Backward compatible with ~18 existing test call sites
- Production code unchanged (already correct)
- Minimal, focused change

### Tradeoffs
- The convenience method still defaults to empty paths when not provided (acceptable since production doesn't use this method)
- `get_workspace_diagnostics` has the same pattern but is out of scope per the issue

## Alternatives Considered

1. **Thread include_paths through the provider interface** — Would violate `PullDiagnosticsProvider`'s stateless design. Production already handles this correctly at the call site.

2. **Make include_paths required** — Would break all existing test call sites (compilation failures). The plan review correctly flagged this as HIGH risk.

3. **Add new method `get_document_diagnostics_with_inc_paths`** — Unnecessary API surface when optional parameter achieves the same goal more simply.

4. **Resolve paths inside convenience method** — Would require server state dependencies in `PullDiagnosticsProvider`, violating separation of concerns.
