# Adversarial Design Findings — work-7e2b0fe5

## Current Approach

The ADR proposes to build an export symbol table in the workspace index and integrate export table queries into `find_symbol_key_definition_location` (which already has workspace index access), rather than threading the workspace index through `symbol_at_cursor`. The rationale is that `symbol_at_cursor` has too many call sites across multiple crates, so the ADR avoids changing its signature. The solution has four phases: export symbol extraction, workspace index extension, declaration resolution update via `find_symbol_key_definition_location`, and completion enhancement.

## Alternative Approaches

### Alternative 1: Enhance `find_import_source` with Workspace-Aware Export Resolution

**Core idea:** Instead of deferring the export table query to `find_symbol_key_definition_location`, enhance `find_import_source` in `declaration.rs` to accept the workspace index (or a pre-populated export map) and use it to resolve `Module->import()` with no args.

**Why it might be better:**
- `find_import_source` already walks the AST looking for `require Module;` + `Module->import(...)` patterns. It has direct access to the importing file's AST context.
- When `find_import_source` encounters `Module->import()` with no args, it can immediately query the export table for `Module`'s `@EXPORT` and check if `symbol_name` is in there.
- This keeps the logic where it belongs: `symbol_at_cursor` calls `find_import_source` to get the correct package, so `symbol_key.pkg` is set correctly from the start.

**Why it might be worse:**
- Requires threading the workspace index through to `find_import_source` from `symbol_at_cursor`, which the ADR explicitly rejects due to "blast radius."
- However, `find_import_source` is a private helper function inside `declaration.rs`, so the blast radius is actually limited to one internal refactor.

**What it sacrifices:** The ADR's architectural preference for keeping `symbol_at_cursor` unchanged.

---

### Alternative 2: Return Import Provenance with SymbolKey

**Core idea:** Modify `symbol_at_cursor` to return `Option<(SymbolKey, Option<String>)>` where the second value is `Some(exporting_module)` when the symbol was resolved via a default `Module->import()` but the module couldn't be determined without the export table.

**Why it might be better:**
- Preserves the AST context (which modules are in scope via `use`/`require`) through the resolution chain.
- `find_symbol_key_definition_location` would receive both the symbol key AND the module that might export it, enabling targeted export table queries.
- Only changes the return type of `symbol_at_cursor`, not its signature with callers (callers can ignore the provenance initially).

**Why it might be worse:**
- The return type change is visible to all callers (tests, other crates).
- The provenance is only needed for the specific case of default import, adding complexity for a corner case.

**What it sacrifices:** Simplicity — the current approach keeps `symbol_at_cursor` as a pure `Option<SymbolKey>`.

---

### Alternative 3: Lazy Export Table Population via AST-Only Scan in `find_import_source`

**Core idea:** When `find_import_source` encounters `Module->import()` with no args, perform an on-the-fly scan of workspace files to find `Module.pm` and parse its `@EXPORT` directly (without pre-building an export table).

**Why it might be better:**
- No need to extend `FileIndex` or `WorkspaceIndex` with export tracking.
- Works with the existing architecture without adding new data structures.
- Useful for one-off resolution without polluting the index.

**Why it might be worse:**
- Performance: Parsing `Module.pm` on every resolution is expensive.
- Doesn't handle modules outside the workspace.
- Doesn't scale to large workspaces with many Exporter modules.

**What it sacrifices:** Performance and scalability — the ADR's approach builds the export table once during indexing.

---

## Strongest Argument Against Current Approach

The ADR's Phase 3 states that when local symbol resolution fails in `find_symbol_key_definition_location`, it will "query the export table: 'which module in scope exports this symbol?'" But this reveals a critical gap: **the function has no way to enumerate which modules are "in scope."**

The call sequence is:
1. `symbol_at_cursor` calls `find_import_source(ast, "load_data")`
2. `find_import_source` encounters `Module->import()` with no args and returns `false` (line 1409)
3. `symbol_at_cursor` falls back to `current_pkg` (e.g., "main") — `symbol_key.pkg = "main"`
4. `find_symbol_key_definition_location(workspace_index, &symbol_key)` is called with `symbol_key.pkg = "main"`
5. The ADR says to query the export table at this point

But in step 5, `find_symbol_key_definition_location` only knows `symbol_key.pkg = "main"` and `symbol_key.name = "load_data"`. It does NOT know that `My::Loader` is in scope. **The ADR never explains how `find_symbol_key_definition_location` would discover which modules are in scope without the AST context that `find_import_source` has.**

The existing `WorkspaceIndex` can find symbols by name across the workspace, but it cannot answer "which modules export symbol X?" without the export table being pre-built. The export table is keyed by module name (`HashMap<String, HashSet<String>>` — module → exported symbols), not by symbol name.

To answer "which module in scope exports `load_data`?", you would need:
1. The list of modules currently in scope (from the importing file's AST — `find_import_source` has this)
2. For each in-scope module, check if it exports the symbol (requires the export table)

The ADR's approach skips step 1 entirely. `find_symbol_key_definition_location` cannot enumerate in-scope modules without access to the importing file's AST or a pre-computed "modules in scope" index.

## Recommended Action

**Modify** the current approach. The Phase 3 plan is incomplete as stated.

The correct fix requires:
1. **Keep Phase 1 and Phase 2** (export extraction and workspace index extension) as proposed.
2. **Modify Phase 3**: Instead of querying the export table in `find_symbol_key_definition_location`, enhance `find_import_source` to use the export table. When `find_import_source` encounters `Module->import()` with no args, it should:
   - Look up `Module` in the workspace index's export table
   - If `symbol_name` is in `Module`'s `@EXPORT`, return `Some("Module")` instead of `false`
3. This requires threading `workspace_index` (or just the export table subset) to `find_import_source`, but this is a private function inside `declaration.rs` — the blast radius is much smaller than the ADR suggests.

The key insight: `find_import_source` is the function that already has the logic for tracing symbol provenance from imports. It SHOULD be the place where export table resolution happens. The ADR rejected this by claiming `find_import_source` returns `false` "before the package is known," but this is wrong — `find_import_source` has `expected_module` (the object of the `->import()` call) and can look it up in the export table.

## Long-Term Cost Assessment

**If we do it the current way (Phase 3 as proposed):**
- **6 months**: The implementation will stall or be incomplete because the "query the export table" step cannot be implemented without first solving "which modules are in scope?" The ADR will need revision.
- **2 years**: Either the feature remains unimplemented, or a workaround is added (like building a separate "modules in scope" index) that duplicates functionality already present in `find_import_source`. Technical debt accumulates.

**If we do it the modified way (enhance `find_import_source`):**
- **6 months**: Cleaner implementation that actually works. The feature is completed.
- **2 years**: `find_import_source` becomes the canonical place for import/export resolution, aligned with how Perl's Exporter actually works. Future features (like `use base 'Exporter'` support) are easier to add here.

The ADR's approach attempts to avoid changing `symbol_at_cursor`'s signature, but creates a harder problem: `find_symbol_key_definition_location` needs AST context it doesn't have. The ADR underestimates this gap.
