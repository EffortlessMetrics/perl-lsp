# Specs: Cross-File Role Composition Diagnostics

## Feature Description

Detect when a Perl class consumes multiple roles (via `with 'RoleA', 'RoleB'`) that provide the same method, causing a conflict in Moose/Moo/Mouse role composition. The diagnostic should work across files — when a role is defined in file A and consumed by a class in file B.

**Already Implemented (Same-File)**: `check_role_conflicts()` in `role_conflicts.rs` detects conflicts when the class and all roles are defined in the same file.

**This Spec Covers**: Cross-file role conflict detection and `-excludes` syntax support.

## Non-Goals

- **Role::Tiny support**: Different syntax (`role { ... }` blocks, `apply_roles_to_package`) — out of scope
- **Eager workspace-wide role indexing**: On-demand parsing is acceptable for MVP
- **Transitive role composition**: When RoleA `with`s RoleB and both provide the same method, intentionally ignores transitive chains
- **Roles in `@INC` outside workspace**: Would require module resolution infrastructure — not handled

## Dependencies

### Required Infrastructure Changes

1. **`perl-workspace-index`** — `IndexVisitor` must index `SymbolKind::Role` symbols:
   - When `NodeKind::Package` is visited, check if `SymbolTable` classifies it as a Role
   - Store as `SymbolKind::Role` alongside `SymbolKind::Package`
   - Dual-index under both qualified and bare names for cross-file lookup

2. **`perl-lsp-diagnostics`** — Extend `check_role_conflicts()`:
   - Add `Option<&WorkspaceIndex>` parameter
   - Use standalone function pattern (like `detect_dead_code`)
   - Query workspace index when role not found same-file
   - On-demand parse role file to extract methods
   - Cache parsed role methods per diagnostic pass

3. **`perl-semantic-analyzer`** — No changes required:
   - `SymbolExtractor` already detects roles via `FrameworkKind::MooRole | FrameworkKind::MooseRole`
   - `ClassModelBuilder` already extracts role methods via `provided_method_names()`

### Existing Components Used

| Component | Location | Purpose |
|-----------|----------|---------|
| `check_role_conflicts()` | `lints/role_conflicts.rs` | Same-file conflict detection (already works) |
| `ClassModelBuilder` | `class_model.rs` | Extracts methods from role models |
| `SymbolExtractor` | `symbol.rs` | Detects roles via framework flags |
| `SymbolKind::Role` | `perl-symbol-types` | Already exists |
| `PL303` (RoleConflict) | `perl-diagnostics-codes` | Already defined |

## Acceptance Criteria

### AC1: Same-File Role Conflicts Still Work
**Given** a file with a class consuming two roles that provide the same method  
**When** diagnostics are generated  
**Then** PL303 (RoleConflict) is emitted at the `with` statement location  

**Test**: Existing tests in `role_conflicts_tests.rs` continue to pass.

### AC2: Cross-File Role Conflicts Are Detected
**Given** RoleA.pm defines `method foo` and is consumed by MyClass.pm in a different file  
**And** MyClass also consumes RoleB.pm that also defines `method foo`  
**When** diagnostics are generated for MyClass.pm  
**Then** PL303 (RoleConflict) is emitted for the `with 'RoleB'` statement  

**Test**: New integration test with RoleA.pm and MyClass.pm in different files.

### AC3: Roles Not Found In Workspace Are Skipped Gracefully
**Given** MyClass consumes a role that is not in the workspace index  
**When** diagnostics are generated  
**Then** No "role not found" diagnostic is emitted  
**And** No crash or error occurs  
**And** Other diagnostics are still generated  

**Test**: Class consuming an external role (e.g., `with 'Some::External::Role'`) produces no error.

### AC4: Excluded Methods Do Not Trigger False Positives
**Given** MyClass consumes RoleA and RoleB that both provide `method foo`  
**And** MyClass explicitly excludes `foo` via `with 'RoleA' => { -excludes => 'foo' }, 'RoleB'`  
**When** diagnostics are generated  
**Then** No PL303 warning is emitted for `method foo`  

**Test**: Role conflict with exclusion syntax does not emit PL303.

### AC5: Diagnostic Message Includes Suggestion
**Given** a role conflict is detected  
**When** PL303 is emitted  
**Then** the diagnostic message includes:
- Names of the conflicting roles
- Name of the conflicting method
- Suggestion to use `-excludes` or define the method in the class  

**Test**: Diagnostic message matches expected format.

### AC6: Works When WorkspaceIndex Is Unavailable
**Given** `check_role_conflicts()` is called without a workspace index  
**When** a same-file conflict exists  
**Then** the conflict is still detected (same-file path is preserved)  

**Test**: Same-file conflict detection works when `None` is passed for workspace index.

## Implementation Notes

### On-Demand Role Parsing Strategy
1. When `check_role_conflicts` encounters a role reference not found in same-file `role_models`:
2. Query `workspace_index.find_definition(role_name)`
3. If found, parse the role file to extract `ClassModel`
4. Extract methods via `provided_method_names()`
5. Cache result in a `HashMap<String, ClassModel>` for the diagnostic pass

### Cache Interface
- **Key**: Role name (e.g., `"My::Role"` or bare `"Role"`)
- **Value**: `ClassModel` for the role
- **TTL**: Per diagnostic pass (cleared after each `get_diagnostics()` call)
- **Invalidation**: No explicit invalidation needed for per-pass caching

### IndexVisitor Change Details
```
In IndexVisitor::visit_node for NodeKind::Package:
  1. Check current_package in symbol_table — is it classified as Role?
     (Requires access to SymbolTable or passing classification info)
  2. If Role: store as SymbolKind::Role
     If Class/Package: store as SymbolKind::Package/SymbolKind::Class
```

**Challenge**: `IndexVisitor` doesn't currently have access to `SymbolTable` which contains the framework classification. This may require:
- Passing `SymbolTable` to `IndexVisitor` (significant change)
- Adding a new `NodeKind::Role` variant to the parser (simpler but requires parser change)
- Storing role classification metadata alongside the AST (alternative approach)

This is an open design question to resolve in implementation.

## Test Scenarios

| Scenario | Input | Expected |
|----------|-------|----------|
| Same-file conflict | Class + RoleA + RoleB all in one file | PL303 emitted |
| Cross-file conflict | RoleA.pm, RoleB.pm, MyClass.pm | PL303 emitted for MyClass |
| No conflict | RoleA provides `foo`, RoleB provides `bar` | No PL303 |
| Class defines method | RoleA and RoleB both have `foo`, Class defines `foo` | No PL303 |
| Exclusion syntax | `with 'RoleA' => { -excludes => 'foo' }, 'RoleB'` | No PL303 for `foo` |
| External role | Class with `with 'External::Role'` | No error, no PL303 |
| Role not indexed | Role exists but indexer hasn't processed it | Silent skip |
| Multiple classes | RoleA consumed by ClassB and ClassC | Both get PL303 |