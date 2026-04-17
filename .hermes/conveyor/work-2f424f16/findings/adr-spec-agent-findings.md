# ADR/Spec Findings — work-2f424f16

## What This ADR Decides

The ADR decides **how to implement cross-file role composition diagnostics** given that the original plan was built on a false premise: `WorkspaceIndex` does NOT store Role symbols. The ADR chooses to **extend the existing `IndexVisitor` infrastructure** to index roles rather than creating a separate `RoleIndex` or using on-demand file search.

## Key Decision

**Extend `WorkspaceIndex` via `IndexVisitor`** to index `SymbolKind::Role` symbols alongside existing Package/Subroutine/Variable handling, rather than:
- Creating a separate `RoleIndex` structure (rejected: parallel infrastructure)
- Using on-demand file search (rejected: no caching, O(n) per lookup)

**Secondary decision**: Include `-excludes` syntax parsing in Phase 1 rather than deferring to Phase 4, because false positives harm user trust more than false negatives.

## Alternatives Considered

1. **Separate RoleIndex** — Rejected because it creates parallel infrastructure that must be kept in sync with the main index
2. **On-demand file search** — Rejected because no caching, O(n) per lookup, no integration with index invalidation
3. **Defer exclusion syntax** — Rejected because users with exclusion syntax get spurious warnings from day one

## Consequences

**Benefits**:
- Cross-file lookups via `WorkspaceIndex::find_definition("RoleName")` work
- Consistent architecture with existing cross-file features (dead code detection pattern)
- Role indexing benefits other potential features (role hierarchy analysis, find consuming classes)

**Tradeoffs**:
- Infrastructure change to Tier 3 crate (`perl-workspace-index`)
- Parse storm risk when role file changes (mitigated by per-pass caching)
- IndexVisitor needs access to role classification info (open design question)

**Risks**:
| Risk | Mitigation |
|------|------------|
| IndexVisitor modification breaks existing indexing | Add tests |
| Parse storm on role save | Per-pass caching, consider debouncing |
| Role in `@INC` triggers noise | Silent skip is intentional |
| Memory bloat from caching | Bounded by role count, per-pass TTL |

## Acceptance Criteria

1. **Same-File Still Works**: Existing tests pass, same-file conflicts detected
2. **Cross-File Detected**: Role in file A, class in file B — PL303 emitted
3. **Graceful Degradation**: Role not found → silent skip, no error
4. **Exclusion Syntax**: `with 'RoleA' => { -excludes => 'foo' }, 'RoleB'` → no PL303 for `foo`
5. **Helpful Message**: Diagnostic includes role names, method name, suggestion
6. **No Workspace Index**: Same-file path preserved when workspace index unavailable