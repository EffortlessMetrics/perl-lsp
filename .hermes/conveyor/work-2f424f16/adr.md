# ADR-2f424f16: Cross-File Role Composition Diagnostics

## Status
**Proposed**

## Context

GitHub Issue #3605 requests role composition diagnostics for perl-lsp, specifically detecting when a class consumes multiple roles that provide the same method. A same-file MVP was merged in PR #3719 implementing `check_role_conflicts()` in `crates/perl-lsp-diagnostics/src/lints/role_conflicts.rs`. The remaining work is **cross-file** role conflict detection.

**Critical Finding (Verification)**: The `WorkspaceIndex` does NOT store Role symbols. The `IndexVisitor` (workspace_index.rs:2911-2961) only handles:
- `NodeKind::Package` → `SymbolKind::Package`
- `NodeKind::Subroutine` → `SymbolKind::Subroutine`  
- `NodeKind::VariableDeclaration` → `SymbolKind::Variable`
- `NodeKind::VariableListDeclaration` → `SymbolKind::Variable`
- `use constant` → `SymbolKind::Subroutine`

There is **no handling for `SymbolKind::Role`**. Any plan that assumes `WorkspaceIndex::find_definition("My::Role")` will work is built on a false premise.

## Decision

**We will extend `WorkspaceIndex` to index Role symbols via the existing `IndexVisitor` infrastructure**, rather than creating a separate `RoleIndex` or using on-demand file search.

### Sub-Decisions

#### 1. Index Role Symbols in IndexVisitor

Add `SymbolKind::Role` handling to `IndexVisitor::visit_node` alongside existing Package/Subroutine/Variable handling. When a `NodeKind::Package` is visited, check if the `SymbolTable` indicates it's a Role (via `FrameworkKind::MooRole | FrameworkKind::MooseRole` analysis already performed by `SymbolExtractor`). If so, store as `SymbolKind::Role` instead of `SymbolKind::Package`.

**Why not create a separate `RoleIndex`?**
- A separate index creates parallel infrastructure that must be maintained
- The `detect_dead_code` pattern shows the established approach: pass `WorkspaceIndex` to standalone diagnostic functions
- Roles are namespaces that benefit from the same dual-indexing (qualified + bare name) that packages use
- Other workspace-wide features would benefit from role indexing (find all roles consuming a base role, etc.)

**Why not on-demand file search?**
- Searching all files for role definitions is O(n) per lookup with no caching
- No integration with the existing index invalidation state machine
- Parse storm risk is higher without shared caching

#### 2. Exclusion Syntax Handling in Phase 1

Parse `with 'Role' => { -excludes => 'method' }` options hash in Phase 1 (not deferred to Phase 4).

**Why include in Phase 1?**
- Users who already use exclusion syntax will receive spurious PL303 warnings
- The Moose exclusion syntax is parseable (hash with `-excludes` key)
- Deferring it creates a poor initial user experience that undermines the diagnostic's credibility
- False positives are more harmful than false negatives for a first release

#### 3. Graceful Degradation for External Roles

When a role cannot be resolved (not in workspace index, not in `@INC`):
- **Same-file roles**: Continue using existing same-file detection
- **Cross-file roles not found in index**: Skip diagnostic for that role (silent skip, no diagnostic emitted)
- **External roles (CPAN, @INC)**: Silent skip (different from "unresolved role" warning — we don't know it's a role)

**Why silent skip over emitting a diagnostic?**
- Emitting "role not found" warnings for legitimate external roles creates noise
- Users consuming roles from CPAN (e.g., `with 'Moose::Util::TypeConstraints'`) expect these to "just work"
- The diagnostic should focus on **conflicts** when roles ARE resolved, not force resolution of all roles

## Alternatives Considered

### Alternative 1: Separate RoleIndex Structure
Create a new `RoleIndex` in `perl-workspace-index` that stores role → methods mapping alongside the workspace index.

**Rejected because**: Creates parallel indexing infrastructure that must be kept in sync with the main index. The same-file MVP already shows that `ClassModelBuilder` can extract role methods — extending the index to store role locations (not full method signatures) is less invasive and benefits all cross-file features.

### Alternative 2: On-Demand File Search
When checking `with 'RoleA'`, search all workspace files for role definitions on-demand.

**Rejected because**: No caching, O(n) per lookup, no integration with index invalidation. Parse storm risk is unmitigated. The `detect_dead_code` pattern shows that workspace-wide operations benefit from shared indexing.

### Alternative 3: Extend WorkspaceIndex But Defer Exclusion Syntax
Add Role indexing but defer `-excludes` parsing to Phase 4 (optional enhancement).

**Rejected because**: Users with exclusion syntax get false positives from day one. The issue explicitly mentions `-excludes` as a risk factor. Deferring a parseable syntax when it directly causes false positives is poor UX.

## Consequences

### Benefits
- Cross-file role conflict detection works via `WorkspaceIndex::find_definition("RoleName")`
- Consistent architecture with existing cross-file features (dead code detection)
- Role indexing benefits other potential features (role hierarchy analysis, find consuming classes)
- Graceful degradation avoids noise for external role users

### Tradeoffs
- **Infrastructure change to Tier 3 crate**: `perl-workspace-index` is shared infrastructure; changes require careful review
- **Parse storm risk**: Many classes consuming one role will all need re-diagnosis when role file changes. Mitigation: caching layer with per-pass TTL.
- **Initial implementation complexity**: Role symbols need to be distinguished from packages during indexing, requiring cross-referencing with `SymbolTable` or adding a new `NodeKind::Role` variant

### Risks
| Risk | Mitigation |
|------|------------|
| IndexVisitor modification breaks existing indexing | Add tests verifying Package and Role are both indexed correctly |
| Parse storm on role save | Cache parsed role methods per diagnostic pass; consider debouncing |
| Role in `@INC` triggers "not found" behavior | Silent skip is intentional; no diagnostic emitted |
| Memory bloat from caching | Cache invalidates after diagnostic pass; bounded by role count |

## Implementation Approach

### Phase 1: Extend WorkspaceIndex to Index Roles
1. In `IndexVisitor::visit_node`, when `NodeKind::Package` is visited, check if it's a Role
2. Store as `SymbolKind::Role` in `file_index.symbols` when detected
3. Add dual-indexing (qualified + bare name) for Role symbols
4. Add tests verifying `find_definition("My::Role")` returns the role's location

### Phase 2: Extend check_role_conflicts for Cross-File
1. Add `Option<&WorkspaceIndex>` parameter to `check_role_conflicts()`
2. Use `detect_dead_code` pattern: standalone function taking workspace index directly
3. When role not found same-file, query `WorkspaceIndex::find_definition`
4. On-demand parse of role file to extract `ClassModel` and methods
5. Cache results per diagnostic pass

### Phase 3: Conflict Detection with Exclusion Support
1. Parse `-excludes` hash in `with 'Role' => { -excludes => 'method' }`
2. Suppress PL303 for explicitly excluded methods
3. Emit conflict diagnostics for remaining method intersections

### Phase 4: Integration Testing
1. Role in file A, class in file B — verify diagnostic emitted
2. Role in file A, two classes B and C consuming it — verify both get diagnostic
3. Role with exclusion — verify no diagnostic for excluded method