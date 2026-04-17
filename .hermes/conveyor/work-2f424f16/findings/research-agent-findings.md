# Research Findings — work-2f424f16

## Issue Summary
GitHub Issue #3605 requests cross-file role composition diagnostics for perl-lsp — detecting when a class consumes multiple roles (via `with 'RoleA', 'RoleB'`) that provide the same method. A same-file MVP was already merged in PR #3719; this work extends to cross-file scenarios using workspace-wide indexing.

## Relevant Codebase Areas
- `crates/perl-semantic-analyzer/src/analysis/class_model.rs` — `ClassModel.roles: Vec<String>` stores raw role names
- `crates/perl-lsp-diagnostics/src/lints/role_conflicts.rs` — Same-file only conflict detection (130 lines)
- `crates/perl-workspace-index/src/workspace/workspace_index.rs` — Workspace index with `SymbolKind::Role` but no method info
- `crates/perl-lsp-diagnostics/src/diagnostics.rs` — Diagnostic pipeline calling `check_role_conflicts()` without workspace index

## Key Findings
1. **Same-file MVP exists**: `check_role_conflicts()` already handles roles in the same file
2. **Cross-file gap**: The function doesn't receive `WorkspaceIndex` — cannot resolve roles in other files
3. **WorkspaceSymbol lacks methods**: Only stores `name`, `kind`, `uri` — no method details
4. **Two design options**: On-demand parsing (simpler) vs extended role index (more infrastructure)
5. **Open questions unresolved**: Eager vs on-demand parsing, handling roles outside workspace

## Proposed Approach
Extend `check_role_conflicts()` to accept `Option<&WorkspaceIndex>`, implement on-demand role method extraction by querying workspace index for role file URI then parsing to extract methods, and add caching for the diagnostic pass duration. Keep same-file path as fast path.

## Top Risks
1. **Parse storm on role save** — Many classes consume single role; all need re-diagnosis
2. **False positives without exclusion detection** — Moose `-excludes` syntax not currently detected
3. **Performance regression** — Cross-file parsing adds latency to diagnostic generation

## Scope
**Covers:** Cross-file Moose/Moo role conflict detection, `-excludes` syntax detection, graceful degradation when workspace index unavailable

**Does NOT cover:** Role::Tiny support, roles in `@INC` outside workspace, eager workspace-wide role indexing
