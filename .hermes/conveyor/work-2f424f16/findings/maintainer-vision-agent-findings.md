# Maintainer Vision Findings — work-2f424f16

## Alignment Assessment
**Misaligned** — The plan is built on a false premise and significantly underestimates required scope. The issue title ("feat: Role composition diagnostics") and plan framing suggest a `perl-lsp-diagnostics` change, but cross-file role conflict detection actually requires modifying `perl-workspace-index` to add Role symbols to the `IndexVisitor` — a Tier 3 infrastructure crate. This prerequisite work is entirely absent from the plan.

## Reasoning

### The Core Problem: IndexVisitor Doesn't Index Roles

The `IndexVisitor` in `perl-workspace-index/src/workspace/workspace_index.rs:2911-2961` only handles:
- `NodeKind::Package` → `SymbolKind::Package`
- `NodeKind::Subroutine` → `SymbolKind::Subroutine`
- `NodeKind::VariableDeclaration` → `SymbolKind::Variable`
- `NodeKind::VariableListDeclaration` → `SymbolKind::Variable`
- `use constant` → `SymbolKind::Subroutine`

**There is no handling for `SymbolKind::Role`.** The `SymbolKind::Role` type exists in `perl-symbol-types` (line 83), is recognized by `SymbolExtractor` in `perl-semantic-analyzer`, and is used for same-file detection in `check_role_conflicts`. But the workspace index never stores them.

The plan's Phase 2 states: "When a role reference can't be resolved same-file, query `WorkspaceIndex` for the role's file URI." This **will not work** because `find_definition("My::Role")` and `search_symbols("RoleName")` will never find role symbols.

### This Is a Tier 3 Crate Change

The roadmap (`docs/project/ROADMAP.md`) shows the codebase is in v0.12.x stability/quality hardening, heading toward v0.13.0 public alpha. The current sprint is about:
- Parser confidence (v0.12.5)
- Performance for larger workspaces (v0.12.6)
- Distribution & packaging (v0.12.7)

Expanding the workspace index to handle Role symbols is a non-trivial infrastructure change — it affects the core indexing pipeline that all cross-file features depend on. This is not a quick diagnostic-lint addition; it's a foundational change.

### The Same-File MVP Is Good

The existing `check_role_conflicts()` in `role_conflicts.rs` is well-implemented:
- Uses `ClassModelBuilder` correctly
- Distinguishes Role vs Class via `SymbolKind::Role`
- Collects methods via `provided_method_names()`
- Has 7 test scenarios covering edge cases

This is solid work that should be preserved.

### The Established Pattern Exists But Isn't Applicable Here

The `detect_dead_code()` function in `dead_code.rs` is the correct pattern for workspace-wide diagnostics:
- Standalone function taking `WorkspaceIndex` directly
- Uses `workspace_index.find_unused_symbols()` to get symbols
- Filters by document URI

However, this pattern works **only because** `WorkspaceIndex` already stores the symbols it needs (Subroutine, Variable, Package, Constant). For role conflicts, we need Role symbols in the index first. The pattern is right; the prerequisite infrastructure is missing.

### False Positives from Exclusion Syntax

The plan defers `-excludes` syntax detection to Phase 4 (optional enhancement). This creates a poor user experience: anyone already using `with 'RoleA' => { -excludes => 'method' }, 'RoleB'` will receive spurious PL303 warnings. The Moose exclusion syntax is parseable (it's a hash with `-excludes` key), and users who already use it will be punished by the diagnostic.

## Impact on Codebase Trajectory

**If we merge this as proposed:**
- Phase 2 will fail silently — cross-file role lookups return nothing, users get no diagnostics for cross-file conflicts
- We'll have to retrofit Role indexing into IndexVisitor under pressure from a shipped feature
- The "add optional parameter" approach will have created dead code paths that never execute (the workspace index path)
- Future maintenance burden: two code paths for a feature that should be unified

**6 months from now:**
- The cross-file detection will likely still be unimplemented or broken
- The same-file MVP will be stable and tested
- The team will need to circle back to add Role indexing as a separate, more disruptive change
- Or: users will have learned the feature doesn't work cross-file and will have stopped relying on it

**If we fix the plan first:**
- We add Role symbols to IndexVisitor as prerequisite infrastructure
- We use the `detect_dead_code` standalone function pattern
- We handle exclusions in Phase 1 to avoid false positives
- The codebase gains a reusable `RoleIndex` capability that other features could use

## Recommendations

### 1. [BLOCKING] Add Role Indexing as Explicit Prerequisite

Before any diagnostic pipeline changes, add Role symbols to `IndexVisitor`:

```rust
// In IndexVisitor::visit_node, add:
NodeKind::Package { name, .. } => {
    // Existing Package handling...
}

// PLUS: detect if it's a Role via framework analysis
// OR add a new NodeKind::Role variant
```

This is prerequisite work — without it, Phase 2 cannot proceed.

### 2. [REQUIRED] Define Cross-File Resolution Strategy

Three options, need to pick one:

**Option A — Extend WorkspaceIndex (recommended for consistency)**
- Add `SymbolKind::Role` to `IndexVisitor` alongside Package/Subroutine/etc.
- Role symbols stored with `name`, `kind`, `uri`, `qualified_name`
- Cross-file lookup uses `find_definition("RoleName")`

**Option B — Separate RoleIndex**
- Create new `RoleIndex` structure in `perl-workspace-index`
- Stores role → file URI + methods mapping
- More targeted, but creates parallel infrastructure

**Option C — On-Demand File Search**
- Search all workspace files for role definitions when needed
- Expensive, no caching strategy defined
- Not recommended

### 3. [REQUIRED] Define Cache Interface and Invalidation

The plan mentions "cache results for the duration of the diagnostic pass" but defines nothing:
- What is the cache key? (role name string? file URI?)
- When does it invalidate? (per pass, per file save, explicit invalidation?)
- How does it integrate with `WorkspaceIndex` state machine?

Without this, the "parse storm on role save" risk is unmitigated.

### 4. [RECOMMENDED] Move Exclusion Detection to Phase 1

False positives for `-excludes` users are a poor experience. Parse the options hash in `with 'Role' => { -excludes => 'method' }` and suppress PL303 for explicitly excluded methods. The syntax is a hash with a known key — it's parseable.

### 5. [REQUIRED] Scope `perl-workspace-index` Changes Explicitly

The plan's T1-T7 all assume `perl-lsp-diagnostics` changes. Add a T0 or rewrite T1 to include:
- Adding Role symbols to IndexVisitor
- Testing that `find_definition("My::Role")` returns the role's location
- Integration test that Role defined in file A can be found when referenced in file B

## Long-Term Impact

### Technical Debt
- Building cross-file detection on a broken foundation (no indexed roles) creates technical debt that must be repaid later
- The "optional WorkspaceIndex" parameter will become dead code that future engineers will need to understand and potentially clean up
- The same-file path will work while the cross-file path silently fails

### Architecture
- If we add Role symbols to IndexVisitor properly, we open the door for other workspace-wide role features (find all roles consuming a base role, role hierarchy analysis, etc.)
- If we create a separate RoleIndex, we add parallel infrastructure that must be maintained
- The codebase is heading toward v1.0 stability — this is exactly the wrong time to ship half-working features

### User Trust
- Shipping a feature that only works same-file when the issue explicitly requests cross-file detection will disappoint users
- False positive warnings from missing `-excludes` detection will cause users to disable or ignore PL303, undermining the diagnostic's value

## Questions the Pipeline Should Answer

1. **Is cross-file detection in scope for this issue, or should same-file-only be the accepted MVP?** The issue title says "Role composition diagnostics" and requests cross-file detection. But the scope may need to be renegotiated.

2. **Which cross-file resolution strategy should we use — extend WorkspaceIndex or create a separate RoleIndex?** This is an architectural decision that affects long-term maintainability.

3. **How should roles outside the workspace (in `@INC`, from CPAN) be handled?** Silent skip? Different diagnostic code? The plan says "graceful degradation" but doesn't define it.

4. **Is deferring `-excludes` detection acceptable?** This means users with `with 'Role' => { -excludes => 'method' }` get false positives until Phase 4 (if ever). Is this acceptable for an initial release?

5. **Who owns the `perl-workspace-index` changes?** This is a Tier 3 crate that affects all cross-file features. The plan needs a owner for the index infrastructure work.

6. **What's the expected timeline?** Adding Role indexing to IndexVisitor is non-trivial. Is this a v0.12.x change or v0.13.x?
