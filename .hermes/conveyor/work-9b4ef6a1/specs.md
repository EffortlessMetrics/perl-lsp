# Specification: Complete PullDiagnosticsProvider for Production Use

## Feature/Behavior Description

This change completes the production readiness of `PullDiagnosticsProvider` by unifying the workspace diagnostic path to use the orchestrator pattern already established for document diagnostics. The three specific gaps being addressed are:

1. **`handle_workspace_diagnostic` refactoring**: The workspace diagnostic handler currently bypasses `PullDiagnosticsProvider` and uses direct `DiagnosticsProvider` calls. After the change, it will use `PullDiagnosticsProvider::get_workspace_diagnostics_with_context()` for basic/builtin/dead-code diagnostics and `orchestrator.collect_perlcritic_diagnostics()` for external perlcritic, matching the pattern used by `handle_document_diagnostic`.

2. **`is_fixable_diagnostic` consolidation**: The `pull.rs` implementation currently has hardcoded perlcritic policy strings. After the change, it will delegate to `is_fixable_perlcritic_policy()` (imported from diagnostics module), matching the more maintainable pattern used in `diagnostics.rs`.

3. **Orchestrator reset wiring**: The `PullDiagnosticsOrchestrator.reset()` method exists but is not called. After the change, it will be called from `handle_did_change_configuration` when perlcritic configuration changes, ensuring the orchestrator's CriticAnalyzer cache is properly invalidated.

## Acceptance Criteria

### AC1: Workspace Diagnostics Use Orchestrator Pattern

**Given** a workspace with Perl files
**When** a workspace diagnostic request is made (LSP `textDocument/diagnostic` with full result)
**Then** the diagnostics are collected via `PullDiagnosticsProvider::get_workspace_diagnostics_with_context()` and `orchestrator.collect_perlcritic_diagnostics()`
**And** NOT via `DiagnosticsProvider::new()` directly
**And** NOT via `LspServer::collect_external_perlcritic_diagnostics()`

**Verification**: Source inspection confirms `handle_workspace_diagnostic` no longer calls `DiagnosticsProvider::new()` or `collect_external_perlcritic_diagnostics()` directly.

### AC2: Single CriticAnalyzer Cache for Both Diagnostic Types

**Given** document diagnostics have been collected using `handle_document_diagnostic`
**And** workspace diagnostics are collected using `handle_workspace_diagnostic`
**When** perlcritic configuration changes (e.g., `perlcritic_enabled`, `perlcritic_severity`, `perlcritic_profile`)
**Then** both diagnostic paths use the same `PullDiagnosticsOrchestrator.critic_analyzer` cache
**And** the cache is properly invalidated when configuration changes

**Verification**: Source inspection confirms `handle_workspace_diagnostic` calls `orchestrator.collect_perlcritic_diagnostics()` which uses `PullDiagnosticsOrchestrator.critic_analyzer`.

### AC3: `is_fixable_diagnostic` Uses Shared Helper

**Given** the `is_fixable_diagnostic` function
**When** checking if a diagnostic code is fixable
**Then** the implementation in `pull.rs` delegates to `is_fixable_perlcritic_policy()` helper
**And** does NOT have hardcoded policy strings inline

**Verification**: `pull.rs::is_fixable_diagnostic` imports and calls `is_fixable_perlcritic_policy()` from the diagnostics module. The `is_fixable_perlcritic_policy` helper has no hardcoded duplicates elsewhere.

### AC4: Orchestrator Reset Called on Config Change

**Given** `handle_did_change_configuration` is invoked with changed perlcritic settings
**When** the configuration change is processed
**Then** `self.pull_diagnostics_orchestrator.reset()` is called
**And** `orchestrator.critic_analyzer` is set to `None`
**And** `orchestrator.warnings_sent` is cleared

**Verification**: Source inspection confirms `handle_did_change_configuration` calls `self.pull_diagnostics_orchestrator.reset()`.

### AC5: Behavioral Parity for Workspace Diagnostics

**Given** the existing workspace diagnostic tests
**When** tests are run before and after the refactoring
**Then** the test expectations are unchanged
**And** no new test failures are introduced

**Verification**: `cargo test -p perl-lsp-rs lsp_workspace_diagnostic` passes with no changes to test expectations.

### AC6: Unit Tests for `PullDiagnosticsProvider` Pass

**Given** `PullDiagnosticsProvider` unit tests
**When** tests are run
**Then** all existing tests pass

**Verification**: `cargo test -p perl-lsp-rs pull_diagnostics` passes.

## Non-Goals

- This change does NOT remove `LspServer.critic_analyzer` or `LspServer::collect_external_perlcritic_diagnostics()` — those are left for backward compatibility and may be removed in a future change.
- This change does NOT modify the diagnostic output format or ordering in a way that would break existing tests.
- This change does NOT add new diagnostic sources — it only unifies the collection path for existing sources.
- This change does NOT address `handle_workspace_diagnostic` performance optimization — the cooperative yielding behavior is preserved as-is.

## Dependencies

- **Feature flag `workspace`**: Required for dead code detection in `get_workspace_diagnostics_with_context`. The feature gate is already in place.
- **Feature flag `not(target_arch = "wasm32")`**: Required for external perlcritic subprocess execution. Both the orchestrator path and direct path use the same gating.
- **Existing orchestrator infrastructure**: `build_context()`, `collect_perlcritic_diagnostics()`, and `reset()` methods must exist and be functional. Verification confirms they are.
- **Existing `get_workspace_diagnostics_with_context` method**: Must exist and handle all required diagnostic sources. Verification confirms it exists but is currently unused.

## Files to Modify

| File | Changes | Gap |
|------|---------|-----|
| `crates/perl-lsp/src/features/diagnostics/pull.rs` | Update `is_fixable_diagnostic` to delegate to `is_fixable_perlcritic_policy()` | Gap 2 |
| `crates/perl-lsp/src/runtime/diagnostics.rs` | Refactor `handle_workspace_diagnostic` to use orchestrator pattern; add import for `is_fixable_perlcritic_policy` helper | Gap 1, Gap 2 |
| `crates/perl-lsp/src/runtime/lifecycle/workspace.rs` | Call `pull_diagnostics_orchestrator.reset()` in `handle_did_change_configuration` | Gap 3 |

## Test Files to Verify

| File | Purpose |
|------|---------|
| `crates/perl-lsp/tests/pull_diagnostics_tests.rs` | Unit tests for `PullDiagnosticsProvider` |
| `crates/perl-lsp/tests/lsp_pull_diagnostics_test.rs` | Integration tests for LSP pull diagnostics |
| `crates/perl-lsp/tests/lsp_perlcritic_diagnostics_tests.rs` | Perlcritic integration tests (may need cache invalidation test) |

## Verification Commands

```bash
# Unit tests for PullDiagnosticsProvider
cargo test -p perl-lsp-rs pull_diagnostics

# Integration tests for LSP pull diagnostics
cargo test -p perl-lsp-rs lsp_pull_diagnostics

# Perlcritic tests
cargo test -p perl-lsp-rs perlcritic

# Full test suite
cargo test -p perl-lsp-rs

# Clippy
cargo clippy -p perl-lsp-rs --tests

# Format
cargo fmt --all
```
