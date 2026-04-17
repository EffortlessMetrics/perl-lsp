# Adversarial Design Findings — work-2f424f16

## Current Approach

The plan proposes to implement cross-file role composition diagnostics by:
1. Adding a new `get_diagnostics_with_workspace()` variant that accepts `WorkspaceIndex`
2. Extending `check_role_conflicts()` to accept `Option<&WorkspaceIndex>` 
3. Using on-demand parsing to extract role methods when a role isn't found in the same file
4. Caching parsed role methods for the duration of the diagnostic pass
5. Treating `-excludes` syntax support as a Phase 4 optional enhancement

This approach was chosen because it "minimizes changes to the existing architecture" and reuses the same-file logic as a fast path.

## Alternative Approaches

### Alternative 1: Build a Dedicated RoleIndex (Eager Indexing)

**Core idea:** Create a new `RoleIndex` structure that stores role method names alongside the workspace index. This index is updated incrementally when role files change, and `check_role_conflicts` queries it directly without any file parsing.

**Why it might be better:**
- O(1) conflict detection at diagnosis time — no parsing latency on the critical path
- Consistent behavior regardless of whether roles are in the same file or different files
- The RoleIndex can be shared across all diagnostic passes, not just one
- Better cache locality: index is built once, queried many times
- Solves the "parse storm" problem proactively — parsing happens in the background during idle time, not when a user opens a file

**Why it might be worse:**
- More complex to implement initially — requires changes to index update logic
- Need to handle index invalidation when role files change
- More memory usage — storing additional data in the index
- Risk of index staleness if invalidation is imperfect

**What it sacrifices:** The "simplicity" of on-demand parsing. But this simplicity is illusory — the caching layer, the parsing logic, and the WorkspaceIndex lookup pathway all add complexity that accumulates over time.

---

### Alternative 2: Move Conflict Detection to Index Time

**Core idea:** Instead of checking for conflicts when a file is opened/edited, perform conflict detection when role files are indexed. Store conflict information in the index alongside role definitions. When a consuming class is opened, the conflict is already known.

**Why it might be better:**
- No cross-file parsing needed at diagnosis time at all — conflicts are pre-computed
- Much simpler `check_role_conflicts` implementation: just look up pre-computed conflicts
- Works naturally with the existing stateless `DiagnosticsProvider` pattern
- Conflicts are available immediately when either the role or class is opened
- Easier to test: conflict detection logic can be unit tested independently of diagnostics

**Why it might be worse:**
- Conflicts are reported at the role definition location, not at the consuming class
- Can't detect conflicts for classes that consume roles but don't define them in the same file
- Index size grows with the number of potential conflicts
- Need to track which classes consume which roles for invalidation

**What it sacrifices:** The ability to report conflicts at the consuming class location. But Moose/Moo actually raise the conflict error at the consuming class — so reporting at the role location is a departure from Perl semantics.

---

### Alternative 3: Semantic Analysis Layer (Lazy but Architectural)

**Core idea:** Don't pass `WorkspaceIndex` to individual lints. Instead, create a higher-level `SemanticAnalysis` service that wraps cross-file resolution. Lints call `semantic_analysis.get_role_methods("RoleName")` and the service handles finding the role file, parsing it, and caching results. The `DiagnosticsProvider` API stays unchanged.

**Why it might be better:**
- Preserves the existing `DiagnosticsProvider` API — no signature changes needed
- Centralizes cross-file resolution logic in one place
- Can be used by multiple lints that need cross-file information (not just role conflicts)
- Easier to test: `SemanticAnalysis` can be mocked/stubbed independently
- The caching strategy is explicit and auditable in one place

**Why it might be worse:**
- Another layer of abstraction to understand and maintain
- Still requires on-demand parsing — same performance concerns
- Need to plumb `SemanticAnalysis` through the call chain to `check_role_conflicts`
- The "service" pattern may be overkill for this specific use case

**What it sacrifices:** The plan's "minimal architecture change" goal. But adding a service layer for cross-file resolution is a cleaner boundary than threading `WorkspaceIndex` through lint functions.

---

## Strongest Argument Against Current Approach

The plan's **Phase 2** claims to implement "On-Demand Role Method Extraction" but doesn't explain *how* to get role methods from `WorkspaceIndex`. The research analysis states:

> "WorkspaceIndex stores `WorkspaceSymbol` with `kind: SymbolKind::Role`... However, `WorkspaceSymbol` only contains `name`, `kind`, `uri` — no method information"

The plan says to "query `WorkspaceIndex` for the role's file URI, then parse that file to extract methods." But the `WorkspaceIndex` doesn't expose a method to look up a role symbol by name and get its URI. The index stores symbols by qualified name, but the lookup API (`find_definition`, `find_symbols`) is designed for navigation features, not for getting a symbol's file location programmatically.

More critically: **the plan treats `-excludes` syntax support as Phase 4 (optional), but the current data model cannot represent exclusions at all.** `ClassModel.roles: Vec<String>` is just a list of role names. The `try_extract_extends_with` function in `class_model.rs:855` extracts role names from `with 'Role'` calls but completely ignores the `=> { -excludes => 'method' }` hash argument. Supporting exclusions requires:

1. Changing `roles` from `Vec<String>` to `Vec<(String, Vec<String>)>` — a breaking change to `ClassModel`
2. Rewriting `try_extract_extends_with` to parse the hash argument
3. Updating all code that iterates over `roles` to handle the new structure

This is not Phase 4 work — it's foundational work that Phase 1 depends on.

## Recommended Action

**Modify the plan substantially** before implementation. The current approach has three fatal flaws:

1. **Data model flaw**: `ClassModel.roles` cannot represent exclusions. Fix this first.
2. **Index API flaw**: `WorkspaceIndex` doesn't expose the lookup API the plan assumes. Clarify what methods are available and their signatures.
3. **Performance flaw**: On-demand parsing on the diagnostic critical path will cause visible latency. Consider whether the user experience of "conflicts detected after a delay" is acceptable.

**Recommended approach**: Start with **Alternative 3** (Semantic Analysis Layer) but with eager indexing of role methods. The `SemanticAnalysis` service provides a clean API for cross-file lookups, and a `RoleIndex` (built incrementally) provides fast O(1) lookups. This gives you:
- Clean API boundary (no changes to `DiagnosticsProvider` signature)
- Fast conflict detection (no parsing on critical path)
- A foundation for other cross-file lints
- A place to implement exclusions handling without touching the core `ClassModel` immediately

The exclusion syntax can be handled by the `RoleIndex` builder: when indexing a role, also record which methods might be excluded. Then `SemanticAnalysis::get_role_methods` returns only non-excluded methods.

## Long-Term Cost Assessment

**If we do it the current way (on-demand parsing with WorkspaceIndex pass-through):**

- **6 months**: The initial implementation works for simple cases. But as users open larger codebases, diagnostic latency becomes noticeable. The "optional" exclusion support never gets implemented because Phase 4 is deprioritized. Developers start filing bugs about false positives from exclusions.

- **1 year**: The caching layer has grown ad-hoc. Multiple caching strategies exist (per-pass cache, document store cache, index cache) with unclear interaction. The `WorkspaceIndex` lookup pathway is fragile and breaks when the index is in a different state. New lints that need cross-file info replicate the same pattern, adding more technical debt.

- **2 years**: The "minimal architecture change" has become technical debt. `check_role_conflicts` has acquired multiple optional parameters for different cross-file lookups. The exclusion problem is still not solved because it would require a breaking change to `ClassModel`. Role conflict detection is considered "deprecated" and a rewrite is proposed but never happens because it's too risky.

**If we do it with a dedicated RoleIndex:**

- **6 months**: Clean implementation with fast lookups. Exclusion handling is implemented from day one because the RoleIndex structure can represent exclusions without changing `ClassModel`. Multiple lints use the same `SemanticAnalysis` service.

- **1 year**: The `SemanticAnalysis` service is a well-understood part of the codebase. When a new cross-file lint is needed, developers know to extend the service, not to thread more optional parameters. Role conflict detection is fast and reliable.

- **2 years**: The architecture scales. When someone proposes "what if we also checked for role method signature conflicts?" the RoleIndex can be extended. The initial investment pays off as new features are added on top of the same foundation.
