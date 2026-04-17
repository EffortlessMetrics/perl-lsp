# ADR-001: Implement source.fixAll Code Action

## Status
Proposed

## Context

The LSP protocol defines a `source.fixAll` code action kind that allows editors to apply all available quick fixes in a file with a single action. The `CodeActionKind::SourceFixAll` variant already exists in `types.rs` (line 62) and the runtime handler already maps it to the `"source.fixAll"` LSP string, but **no code actually produces a `SourceFixAll` action**.

The `perl-lsp-code-actions` crate contains 20+ quick fix implementations (PL103 UndefinedVariable, PL102 UnusedVariable, PL100 MissingStrict, etc.) that are individually returned as `QuickFix` actions. Users requesting `source.fixAll` receive no results despite fixes being available.

### Constraints
- The production LSP handler (`runtime/language/code_actions.rs`) does NOT sort edits before applying them. Edit sorting at line 279 exists only in a test helper function.
- Multiple diagnostics (PL100, PL502, TestingAndDebugging::RequireUseStrict) may generate the same pragma inserts (`use strict;`, `use warnings;`).
- Some diagnostics produce multiple fix options; `source.fixAll` should use only the preferred fix.
- Perl::Critic quick fixes are handled separately and are excluded from this scope.

## Decision

We will implement `source.fixAll` by adding a new method `get_fix_all_actions` to `CodeActionsProvider` that:

1. Iterates over **all** diagnostics (not range-filtered)
2. For each diagnostic, generates quick fixes using the same match logic as `get_code_actions`
3. Collects **only the preferred** fix per diagnostic (`is_preferred: true`)
4. Merges all edits into a single `CodeActionEdit`
5. **Sorts edits by `location.start` in descending order** (critical: production handler does not sort)
6. **Deduplicates edits** by checking if `new_text` already exists at the target position in source
7. Returns a single `CodeAction` with `kind = CodeActionKind::SourceFixAll` and all merged edits
8. Returns an empty list if no fixes are available (no empty actions)

The action will be integrated into the LSP server handler alongside other actions from `CodeActionsProvider::get_code_actions()`.

## Consequences

### Benefits
- Completes the existing LSP infrastructure (kind already defined, mapping already in place)
- Reuses existing quick fix logic rather than duplicating it
- Users can trigger "Fix All" in editors (VSCode, Neovim) and apply all fixes at once
- Consistent with LSP `source.fixAll` semantics

### Tradeoffs/Risks
- **Performance**: Diagnostics are iterated twice (once for individual quick fixes, once for `SourceFixAll`). This is acceptable as it's O(2n) and quick fixes are already O(n).
- **Edit conflicts**: Overlapping edits from different fixes could conflict. Descending sort order mitigates offset shifting, but doesn't resolve semantic conflicts.
- **Deduplication complexity**: Multiple fixes may try to add the same pragma at different positions. Deduplication must check if the text already exists at the insertion point.

### Alternatives Considered

1. **Modify `get_code_actions` to return `SourceFixAll` alongside individual fixes**: This would mix aggregated and individual actions in the same list, making kind filtering more complex and potentially returning duplicate fixes (once individually, once in the aggregate).

2. **Create a separate `FixAllProvider` struct**: This would isolate the `source.fixAll` logic but add new crate infrastructure. The simpler approach of adding a method to `CodeActionsProvider` is preferred.

3. **Include Perl::Critic fixes in `SourceFixAll`**: Perl::Critic fixes are handled separately in the LSP handler with their own quick fix infrastructure. Including them would require significant architectural changes and is out of scope.

## Technical Notes

### Edit Ordering
The production LSP handler at `runtime/language/code_actions.rs` lines 213-230 maps actions' edits directly to LSP JSON **without sorting**. The test helper `apply_action` at line 279 shows the correct pattern: `edits.sort_by(|a, b| b.location.start.cmp(&a.location.start))` (descending).

**The implementation must sort merged edits by descending offset** because:
- Applying edits from lowest to highest offset shifts subsequent offsets
- Applying from highest to lowest preserves lower positions

### Deduplication Strategy
- Check if `new_text` already exists in source at the target `location.start` position
- For pragmas at position 0, also check if the source already contains the pragma text anywhere
- Track inserted pragmas to avoid duplicates from different diagnostics (PL100, PL502)

### Preferred Fix Only
When a diagnostic produces multiple fix options (e.g., "Declare with my" vs "Declare with our"), `SourceFixAll` should only include the preferred one. This aligns with "fix all" semantics — automatic application of the preferred resolution.
