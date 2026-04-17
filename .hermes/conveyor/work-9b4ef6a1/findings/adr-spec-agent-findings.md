# ADR/Spec Findings — work-9b4ef6a1

## What This ADR Decides

This ADR decides to complete the production readiness of `PullDiagnosticsProvider` by unifying the workspace diagnostic path to use the orchestrator pattern already established for document diagnostics. The core architectural decision is to make workspace diagnostics use `PullDiagnosticsProvider::get_workspace_diagnostics_with_context()` and `orchestrator.collect_perlcritic_diagnostics()` instead of direct `DiagnosticsProvider` calls and `LspServer::collect_external_perlcritic_diagnostics()`.

## Key Decision

**Refactor `handle_workspace_diagnostic` to use the orchestrator pattern**: The workspace diagnostic handler should use `PullDiagnosticsProvider::get_workspace_diagnostics_with_context()` for basic/builtin/dead-code diagnostics and `orchestrator.collect_perlcritic_diagnostics()` for external perlcritic, matching the pattern used by `handle_document_diagnostic`. This unifies the two CriticAnalyzer caches into one (`PullDiagnosticsOrchestrator.critic_analyzer`).

## Alternatives Considered

1. **Keep Parallel Paths (Status Quo)**: Maintains duplicate code paths with two separate CriticAnalyzer caches. Rejected because it causes split-brain behavior after config changes and increases maintenance burden.

2. **Update `get_workspace_diagnostics_with_context` to Include Perlcritic**: Would require giving the provider access to the server, which breaks the clean separation. Rejected in favor of the two-step pattern already proven in `handle_document_diagnostic`.

3. **Remove `LspServer.critic_analyzer` Entirely**: Too large a change for this PR. Left for future work after the three gaps are verified closed.

## Consequences

**Benefits**:
- Single diagnostic collection path for both document and workspace diagnostics
- Unified perlcritic caching eliminates split-brain issue
- Reduced maintenance burden (one place to add new diagnostic sources)
- `is_fixable_diagnostic` consolidation removes duplication

**Tradeoffs/Risks**:
- Risk of behavioral change in workspace diagnostics
- Risk of performance regression if new path is less efficient
- Risk of breaking existing workspace diagnostic behavior during transition

## Acceptance Criteria

1. **AC1**: Workspace diagnostics use orchestrator pattern (not direct DiagnosticsProvider calls)
2. **AC2**: Both diagnostic paths share the same CriticAnalyzer cache
3. **AC3**: `is_fixable_diagnostic` uses shared `is_fixable_perlcritic_policy()` helper
4. **AC4**: `orchestrator.reset()` is called on perlcritic config change
5. **AC5**: Behavioral parity for workspace diagnostics (existing tests pass)
6. **AC6**: Unit tests for `PullDiagnosticsProvider` pass
