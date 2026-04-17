# ADR-0042: Complete PullDiagnosticsProvider for Production Use

## Status

Proposed

## Context

GitHub Issue #4317 requests completing `PullDiagnosticsProvider` so it can be used in production code instead of just tests. The issue describes a 3-layer architecture:

```
LspServer
  └── PullDiagnosticsOrchestrator (config, workspace index, capabilities)
        └── PullDiagnosticsProvider (pure logic, testable)
              ├── DiagnosticsProvider (existing)
              └── DeadCodeDetection (optional, workspace feature)
```

Investigation reveals this architecture is **already substantially implemented** for document diagnostics (`handle_document_diagnostic` at diagnostics.rs lines 655-721). However, three gaps remain:

### Gap 1: `handle_workspace_diagnostic` Bypasses PullDiagnosticsProvider

`handle_workspace_diagnostic` (diagnostics.rs lines 932-1208) directly calls:
- `DiagnosticsProvider::new()` instead of `PullDiagnosticsProvider::get_workspace_diagnostics_with_context()`
- `LspServer::collect_external_perlcritic_diagnostics()` instead of `orchestrator.collect_perlcritic_diagnostics()`

This creates two parallel diagnostic paths with different behavior and maintenance burden.

### Gap 2: `is_fixable_diagnostic()` Has Divergent Implementations

Two implementations exist:
- `pull.rs` line 865: hardcodes 6 perlcritic policy strings inline
- `diagnostics.rs` line 1420: delegates to `is_fixable_perlcritic_policy()` which has the same policies in a separate helper

The `diagnostics.rs` version is more maintainable since policy strings live in one place.

### Gap 3: Two Separate CriticAnalyzer Caches

The codebase has two separate perlcritic caching paths:

| Component | Document Diagnostic Path | Workspace Diagnostic Path |
|-----------|------------------------|-------------------------|
| Perlcritic collection | `orchestrator.collect_perlcritic_diagnostics()` | `LspServer::collect_external_perlcritic_diagnostics()` |
| CriticAnalyzer | `PullDiagnosticsOrchestrator.critic_analyzer` | `LspServer.critic_analyzer` |
| Warning deduplication | `orchestrator.warnings_sent` | `LspServer.critic_workspace_warnings_sent` |

`handle_did_change_configuration` (workspace.rs line 747) resets `LspServer.critic_analyzer` but NOT `PullDiagnosticsOrchestrator.critic_analyzer`. This means when perlcritic config changes, document diagnostics may use stale cached analysis while workspace diagnostics get fresh analysis.

## Decision

We will complete the production readiness of `PullDiagnosticsProvider` by addressing all three gaps:

### Decision 1: Refactor `handle_workspace_diagnostic` to Use Orchestrator Pattern

The refactoring approach is:

1. **Build context via orchestrator**: `context = self.pull_diagnostics_orchestrator.build_context(self, uri_str)`

2. **Use provider for basic/builtin/dead-code diagnostics**: `provider.get_workspace_diagnostics_with_context()` (this method already exists but is currently unused)

3. **Collect external perlcritic via orchestrator**: `orchestrator.collect_perlcritic_diagnostics()` for each document

This mirrors what `handle_document_diagnostic` already does successfully. The key insight is that `get_workspace_diagnostics_with_context` already exists and is the correct abstraction — we need to wire it up, not modify it.

By using `orchestrator.collect_perlcritic_diagnostics()`, workspace diagnostics will share the same CriticAnalyzer cache as document diagnostics, eliminating the split-brain issue.

### Decision 2: Consolidate `is_fixable_diagnostic()` Implementation

The `pull.rs::is_fixable_diagnostic` should be updated to delegate to `is_fixable_perlcritic_policy()` (currently only in `diagnostics.rs`), rather than having hardcoded policy strings. This ensures both implementations use the same helper and policy list.

The `is_fixable_perlcritic_policy` helper remains in `diagnostics.rs` as a private helper, but `pull.rs` imports and calls it.

### Decision 3: Wire `orchestrator.reset()` into Configuration Change Handling

In `handle_did_change_configuration`, after resetting `LspServer.critic_analyzer`, also call `self.pull_diagnostics_orchestrator.reset()`. This ensures both caches are invalidated when perlcritic configuration changes.

## Consequences

### Positive
- Single diagnostic collection path through `PullDiagnosticsProvider` for both document and workspace diagnostics
- One canonical `is_fixable_diagnostic()` function with no duplication
- Proper cache invalidation when perlcritic configuration changes for both diagnostic types
- Reduced maintenance burden when adding new diagnostic sources
- Unified perlcritic caching eliminates split-brain issue

### Negative
- Risk of behavioral change in workspace diagnostics (different ordering, missing diagnostics, different message formatting)
- Risk of performance regression if the new path is less efficient
- Risk of temporarily breaking the existing workspace diagnostic behavior during transition

### Mitigations
- Add pre-refactoring baseline tests for workspace diagnostics
- Compare diagnostic output before/after for key scenarios
- Incremental approach: verify each phase independently before proceeding
- Keep cooperative yielding behavior preserved in the refactored path

## Alternatives Considered

### Alternative 1: Keep Parallel Paths (Status Quo)

Keep `handle_workspace_diagnostic` using direct `DiagnosticsProvider` calls. This avoids short-term risk but:
- Maintains duplicate code paths that must be kept in sync
- Bugs or features must be added in two places
- Two CriticAnalyzer caches remain split, causing inconsistent behavior after config changes

### Alternative 2: Update `get_workspace_diagnostics_with_context` to Include Perlcritic

Instead of calling `orchestrator.collect_perlcritic_diagnostics()` separately, add perlcritic collection directly to `get_workspace_diagnostics_with_context`. However, this method doesn't have access to the server, so perlcritic collection would need a different mechanism. The two-step approach (provider first, then orchestrator perlcritic) is the correct pattern already established by `handle_document_diagnostic`.

### Alternative 3: Remove `LspServer.critic_analyzer` Entirely

After refactoring, remove `LspServer.critic_analyzer` and `LspServer::collect_external_perlcritic_diagnostics()` since they would be replaced by the orchestrator path. However, this is a larger change that should be done separately after the three gaps are closed and verified stable.

## Notes

- The `get_workspace_diagnostics_with_context` method currently exists in `pull.rs` but is **never called from anywhere**. The refactoring is a wiring task — we need to use the existing method.
- The `orchestrator.reset()` method has `#[allow(dead_code)]` suggesting it was added with the expectation it would be wired later.
- WASM builds (`#[cfg(not(target_arch = "wasm32"))]`) already exclude subprocess-based perlcritic; the orchestrator's perlcritic collection is also gated the same way.
- Dead code detection requires `workspace` feature and is already feature-gated in both the existing provider method and the direct path.
