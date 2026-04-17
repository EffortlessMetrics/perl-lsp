# Verification Findings — work-2f424f16

## Confidence Assessment

**Low-Medium** — While I verified the core claims about the same-file implementation being correct, the key premise about cross-file detection is fundamentally flawed. The research agent made an incorrect assumption about the workspace index that invalidates the proposed approach.

## Confirmed Findings

### 1. Same-File MVP Exists and Works
The `check_role_conflicts()` function in `role_conflicts.rs` (lines 18-84) correctly implements same-file role conflict detection:
- Uses `ClassModelBuilder` to extract `ClassModel` for each package in the file
- Uses `package_kind()` (lines 86-91) to distinguish `SymbolKind::Role` from `SymbolKind::Class` via the `SymbolTable`
- Collects methods from role models via `provided_method_names()` (lines 93-95)
- Reports conflicts with diagnostic code `PL303` (RoleConflict) at the `with` statement location

Evidence: `crates/perl-lsp-diagnostics/src/lints/role_conflicts.rs:18-84`

### 2. `PL303` (RoleConflict) Is Properly Defined
The diagnostic code `PL303` is correctly defined in `perl-diagnostics-codes/src/lib.rs`:
- Line 139: `RoleConflict,` enum variant
- Line 259: `DiagnosticCode::RoleConflict => "PL303",` mapping
- Line 329: documentation URL

Evidence: `crates/perl-diagnostics-codes/src/lib.rs:139,259,329`

### 3. `ClassModel.roles: Vec<String>` Is Accurate
The research agent correctly identified that `ClassModel` stores role names as raw strings at line 195 of `class_model.rs`:
```rust
pub roles: Vec<String>,
```

Evidence: `crates/perl-semantic-analyzer/src/analysis/class_model.rs:195`

### 4. `check_role_conflicts` Does Not Receive `WorkspaceIndex`
The function signature is confirmed:
```rust
pub fn check_role_conflicts(
    node: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
)
```
And it's called at `diagnostics.rs:157`:
```rust
check_role_conflicts(ast, &symbol_table, &mut diagnostics);
```
No `WorkspaceIndex` is passed.

Evidence: `crates/perl-lsp-diagnostics/src/lints/role_conflicts.rs:18-22` and `crates/perl-lsp-diagnostics/src/diagnostics.rs:155-157`

### 5. Role Detection via Framework Flags
The `SymbolExtractor` correctly detects roles via framework flags:
```rust
FrameworkKind::MooRole | FrameworkKind::MooseRole => SymbolKind::Role,
```
And `with 'Role'` statements are tracked as `SymbolKind::Role` references (lines 1333 and 1381 in `symbol.rs`).

Evidence: `crates/perl-semantic-analyzer/src/analysis/symbol.rs:461,1333,1381`

### 6. `detect_dead_code` Uses Standalone Pattern
`perl-lsp-diagnostics` already has a pattern for workspace-index-aware diagnostics: `detect_dead_code()` is a standalone function (not part of `DiagnosticsProvider`) that takes `WorkspaceIndex` directly.

Evidence: `crates/perl-lsp-diagnostics/src/dead_code.rs:23-28`

## Corrected Findings

### CRITICAL: WorkspaceIndex Does NOT Store Role Symbols

**Research Agent Claim**: "WorkspaceIndex has Role symbols But Not Methods" and "`WorkspaceIndex` stores `WorkspaceSymbol` with `kind: SymbolKind::Role`"

**This is INCORRECT.**

The `IndexVisitor` (workspace_index.rs:2641) that populates `WorkspaceIndex` ONLY handles:
- `NodeKind::Package` → `SymbolKind::Package` (line 2762-2778)
- `NodeKind::Subroutine` → `SymbolKind::Subroutine` (line 2781-2824)
- `NodeKind::VariableDeclaration` → `SymbolKind::Variable` (line 2826-2856)
- `NodeKind::VariableListDeclaration` → `SymbolKind::Variable` (line 2858-2889)
- `use constant` → `SymbolKind::Subroutine` (line 2953-2975)

**There is NO handling for `SymbolKind::Role` in the `IndexVisitor`.**

This means:
1. `find_definition("My::Role")` will NOT find role definitions in the workspace
2. `search_symbols("RoleName")` will NOT return role symbols
3. Roles are simply not indexed in the workspace index at all

The research agent assumed roles were indexed but missing method info. In reality, roles are not indexed at all.

**Impact**: The plan's Phase 2 approach — "query `WorkspaceIndex` for the role's file URI" — will NOT work because the workspace index doesn't store roles. The plan needs to either:
1. Add Role symbols to the workspace index (modify `IndexVisitor`)
2. Create a separate Role index
3. Search all files on-demand to find role definitions

## New Findings

### 1. No Tests Exist for `check_role_conflicts`
Grepping for `role_conflict` or `RoleConflict` in test files returned no matches. There are no unit or integration tests for the role conflict detection feature. This is a gap the plan should address.

### 2. Cross-File Detection Requires New Infrastructure
To implement cross-file role conflict detection, you would need:
- A way to find role definitions across files (currently nonexistent in workspace index)
- On-demand parsing of role files to extract `ClassModel` (possible via `ClassModelBuilder`)
- A caching mechanism to avoid re-parsing the same role file multiple times per diagnostic pass

### 3. Exclusion Syntax Not Detected in Same-File MVP
The `check_role_conflicts` function does not parse the `-excludes` hash in `with 'Role' => { -excludes => 'method' }`. The plan correctly identifies this as a risk for false positives, but it's marked as "Phase 4 (Optional Enhancement)" — meaning the base implementation would have false positives.

### 4. Role Inheritance Chains Not Handled
If RoleA `with`s RoleB, and both provide the same method, the consuming class that `with`s RoleA would get a conflict with itself through the chain. The same-file MVP doesn't handle this (it only looks at direct role methods).

## Scope Assessment

**Issue title**: "feat: Role composition diagnostics"

**Actual scope requires**:
1. Adding Role symbols to workspace index (infrastructure change)
2. Extending `check_role_conflicts` to accept `Option<&WorkspaceIndex>` (or creating standalone function)
3. On-demand role file parsing to extract methods
4. Caching layer for parsed role methods
5. Tests for both same-file and cross-file scenarios

**The issue only mentions one crate affected** (`perl-lsp-diagnostics`), but implementing cross-file detection actually requires changes to:
- `perl-workspace-index` (add Role symbols to `IndexVisitor`)
- `perl-lsp-diagnostics` (extend `check_role_conflicts`)
- Possibly `perl-semantic-analyzer` (if role method extraction needs changes)

## Verification Methodology

1. **Read source files directly**: Verified the exact implementation of `check_role_conflicts` and `IndexVisitor`
2. **Grep for patterns**: Searched for `SymbolKind::Role` in workspace-index and found NO matches (proving roles aren't indexed)
3. **Read call chains**: Verified how `detect_dead_code` pattern works (standalone function with `WorkspaceIndex` parameter)
4. **Checked diagnostic codes**: Verified `PL303` exists and is properly defined

## Summary

The research agent correctly identified the same-file MVP implementation and the need to extend the diagnostic pipeline for cross-file detection. However, the agent made a **fundamental error** in assuming that `WorkspaceIndex` stores Role symbols — it does not. Any implementation plan that relies on using `WorkspaceIndex` to find roles is based on a false premise and will fail. The plan needs to be revised to either add Role indexing to the workspace index or use an alternative approach to locate role definitions across files.
