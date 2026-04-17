# Plan Review Findings — work-2f424f16

## Overall Assessment
**Not feasible** — The plan is built on a critical false assumption: that `WorkspaceIndex` can be used to locate Role definitions across files. Verification confirms that roles are **not indexed at all** in the workspace index (`IndexVisitor` only handles `Package`, `Subroutine`, `VariableDeclaration`, `VariableListDeclaration`, and `use constant`). Phase 2 ("query WorkspaceIndex for the role's file URI") will not work as written.

## Scope Assessment
**Mismatch — scope is larger than stated.** The issue title ("feat: Role composition diagnostics") and the plan both describe the work as primarily affecting `perl-lsp-diagnostics`. However, implementing cross-file conflict detection actually requires modifying `perl-workspace-index` to index Role symbols (adding them to `IndexVisitor`), which is a separate Tier 3 crate. This is a non-trivial infrastructure change that the plan doesn't account for.

Additionally, the plan underestimates the work by framing it as "add optional parameter" when the real work is "build new indexing infrastructure for roles."

## What Works
- **Same-file conflict detection** — The existing `check_role_conflicts()` in `role_conflicts.rs` is well-implemented and tested for same-file scenarios. It correctly uses `ClassModelBuilder`, `package_kind()`, and `provided_method_names()` to detect conflicts.
- **The `detect_dead_code` pattern** — The plan correctly identifies `detect_dead_code` as a model for a standalone function that accepts `WorkspaceIndex`. This is the right architectural pattern to follow.
- **Backward compatibility approach** — Adding an optional `WorkspaceIndex` parameter to `check_role_conflicts` preserves the same-file fast path, which is a good design choice.
- **Exclusion syntax (Phase 4)** — Correctly identified as a false-positive risk; good to defer.
- **PL303 diagnostic code** — Properly defined and wired up in `perl-diagnostics-codes`.

## What Doesn't Work

### 1. CRITICAL: WorkspaceIndex Does Not Store Role Symbols
The plan's Phase 2 states: "When a role reference can't be resolved same-file, query `WorkspaceIndex` for the role's file URI."

**This will not work.** Verification confirms `IndexVisitor` only indexes:
- `NodeKind::Package` → `SymbolKind::Package`
- `NodeKind::Subroutine` → `SymbolKind::Subroutine`
- `NodeKind::VariableDeclaration` → `SymbolKind::Variable`
- `NodeKind::VariableListDeclaration` → `SymbolKind::Variable`
- `use constant` → `SymbolKind::Subroutine`

There is **no handling for `SymbolKind::Role`**. The `find_definition("My::Role")` and `search_symbols("RoleName")` operations will NOT find roles. The plan cannot proceed until roles are added to the workspace index.

### 2. CRITICAL: Plan Does Not Account for Required Infrastructure Work
The plan treats the work as primarily a `perl-lsp-diagnostics` change with an optional parameter added. In reality:
- `perl-workspace-index` must be modified to add Role symbols to `IndexVisitor`
- This is a non-trivial change to the index update logic
- The plan has no tasks for this work (T1-T7 all assume the index already works)

### 3. Finding Role Definitions Has No Defined Strategy
The plan says "query `WorkspaceIndex` for the role's file URI" without explaining how to find the role once we don't find it in the index. Possible alternatives not explored:
- Search all files on-demand (expensive, no caching strategy defined)
- Build a separate `RoleIndex` structure (no design)
- Use module resolution infrastructure if it exists (not investigated)

### 4. No Cache Invalidation Strategy
The plan mentions "cache results for the duration of the diagnostic pass" but:
- The plan doesn't define the cache interface (what's the key? the role name? the file URI?)
- When a role file changes, how do consuming classes get re-diagnosed?
- `WorkspaceIndex` has `Invalidating` state — does the role cache need to integrate with this?

## Top Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Plan assumes indexed roles that don't exist** | HIGH | Plan is infeasible as written; entire Phase 2 is blocked | Add Role indexing to `IndexVisitor` before any other work |
| **Parse storm on role save** | MEDIUM | Many classes consuming one role → all re-diagnosed on save | Define caching strategy upfront; consider debouncing |
| **False positives without exclusion detection** | MEDIUM | Users with `with 'Role' => { -excludes => 'method' }` get spurious warnings | Add `-excludes` parsing in Phase 1, not as deferred optional enhancement |
| **Scope creep to index infrastructure** | MEDIUM | Plan underestimates by treating this as a one-crate change | Explicitly scope `perl-workspace-index` changes in tasks |
| **Cross-file resolution finds nothing** | HIGH | If role isn't in workspace (e.g., in `@INC`), nothing happens; user gets no diagnostic | Define graceful degradation behavior explicitly |

## Edge Cases

1. **Role in `@INC` outside workspace** — A class consumes a role from CPAN (`with 'Moose::Util::TypeConstraints'`). The workspace index cannot find it. What happens? Silent skip? The plan doesn't address this.

2. **Role with inheritance chain** — RoleA `with`s RoleB, and both provide the same method. A class `with`s RoleA. The plan explicitly excludes transitive composition ("intentionally ignores... transitive role composition"). Is this acceptable for the MVP?

3. **Role defined but `ClassModelBuilder` fails to parse** — If the role file has syntax errors, `ClassModelBuilder` will not extract a model. The cross-file detection will silently skip the role. Is this acceptable?

4. **Multiple classes consuming same external role** — Without caching, the same role file would be parsed once per consuming class per diagnostic pass. The plan mentions caching but doesn't define it.

5. **Role name is a short bareword** — `with 'Foo'` could mean `My::Package::Foo` or `./Foo.pm` or `%INC{'Foo.pm'}`. Module resolution is complex and the plan doesn't address it.

## Recommendations

**The plan must be revised before proceeding to DESIGNED. The following specific changes are required:**

1. **[BLOCKING]** Add Role symbols to `IndexVisitor` in `perl-workspace-index`:
   - Create a new `NodeKind::Role` variant or use existing `SymbolKind::Role` classification
   - Add handling in `IndexVisitor::visit_node` for role definitions
   - This is prerequisite work — Phase 2 cannot start without it

2. **[REQUIRED]** Define the cross-file resolution strategy explicitly:
   - Option A: Extend `WorkspaceIndex` to store role method names alongside role definitions (requires schema change)
   - Option B: On-demand file parsing (requires defining how to locate the role file first)
   - Option C: Create a separate `RoleIndex` that stores role → methods mapping

3. **[REQUIRED]** Define cache interface and invalidation:
   - What is the cache key? (role name, file URI, or both?)
   - When does it invalidate? (per diagnostic pass, per file save, per explicit invalidation)
   - How does it integrate with `WorkspaceIndex` state machine?

4. **[REQUIRED]** Define graceful degradation for roles outside workspace:
   - If a role cannot be resolved (not in workspace, not in `@INC`), should a diagnostic be emitted?
   - Should there be a different diagnostic code for "unresolved role" vs "resolved role with conflict"?

5. **[RECOMMENDED]** Move exclusion syntax detection to Phase 1:
   - False positives for users already using `-excludes` are a poor experience
   - The syntax is parseable — it's a hash with `-excludes` key

6. **[REQUIRED]** Update T1-T7 to include `perl-workspace-index` changes:
   - T1 should be "Add Role symbols to IndexVisitor" not "Add get_diagnostics_with_workspace overload"
   - The index change is the critical path; the diagnostic pipeline change is downstream

## Confidence to Proceed

**Low** — The plan cannot proceed in its current form. The fundamental assumption that `WorkspaceIndex` can locate Role definitions is incorrect. The plan needs:

1. A new Phase 0 (or renamed Phase 1) that adds Role indexing to `IndexVisitor`
2. A defined cross-file resolution strategy that accounts for the fact that roles are not currently indexed
3. Explicit handling for roles outside the workspace

The same-file MVP (which is already implemented and tested) is solid. The cross-file extension is where the plan breaks down — it assumes infrastructure that doesn't exist.

---

## Verification Corrections

The verification agent made one error worth noting: it stated "No Tests Exist for `check_role_conflicts`." This is **incorrect** — there are tests in `crates/perl-lsp-diagnostics/tests/role_conflicts_tests.rs` covering 7 scenarios:
- Same-file conflict detection with anchoring
- Class defining the method suppresses PL303
- Distinct role methods do not conflict
- `requires` does not count as a provided method
- Multiple `with` calls accumulate conflicts
- Three conflicting roles emit single PL303
- Role consuming roles does not trigger PL303

The same-file implementation is well-tested. The verification agent's other findings (particularly about workspace index not storing roles) are correct and critical.