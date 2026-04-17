# Specs: source.fixAll Code Action

## Feature Description

Implement the LSP `source.fixAll` code action that collects all available diagnostic-driven quick fixes in a file and returns them as a single action that can be applied in one operation.

### Behavior

When a client requests code actions with `kind: "source.fixAll"` (or when the client filters actions to include `"source.fixAll"`), the server returns a single action that:
- Has `title: "Fix All"`
- Has `kind: "source.fixAll"`
- Contains all edits from all applicable quick fixes merged into one `CodeActionEdit`
- References the diagnostics it will resolve
- Is marked as `is_preferred: false` (it's informational, not a single recommended fix)

### What Triggers This Action

The action is available when a file contains diagnostics that have corresponding quick fixes:
- PL103 (UndefinedVariable) → Declare variable
- PL102 (UnusedVariable) → Remove/rename variable
- PL100 (MissingStrict) → Add `use strict`
- PL101 (MissingWarnings) → Add `use warnings`
- PL502 (PhaseScopedStrictPragma) → Move strict to file scope
- PL503 (PhaseScopedWarningsPragma) → Move warnings to file scope
- PL403 (AssignmentInCondition) → Add parentheses or use comparison
- PL109 (UnquotedBareword) → Quote bareword
- And 15+ other diagnostic codes

### What This Does NOT Include

- `source.organizeImports` (already implemented in `EnhancedCodeActionsProvider`)
- `source.modernize` (already implemented in `modernize.rs`)
- Refactoring actions (extract, inline, rewrite)
- Perl::Critic quick fixes (handled separately in the LSP handler)
- Cross-file fixes (single-file only)

## Acceptance Criteria

### AC1: SourceFixAll action is produced when fixes exist
**Given** a Perl file with diagnostics that have quick fixes (e.g., undefined variable, missing strict)
**When** the client requests code actions
**Then** the response includes a `SourceFixAll` action with `kind: "source.fixAll"`

### AC2: SourceFixAll action is NOT produced when no fixes exist
**Given** a Perl file with no diagnostics or diagnostics without quick fixes
**When** the client requests code actions
**Then** the response does NOT include a `SourceFixAll` action

### AC3: All preferred quick fixes are merged into SourceFixAll
**Given** a Perl file with multiple diagnostics (undefined variable, missing strict, unused variable)
**When** the `SourceFixAll` action is applied
**Then** all preferred quick fixes are applied (variable declared, `use strict` added, unused variable removed)

### AC4: Edits are sorted by descending offset
**Given** a `SourceFixAll` action with multiple edits
**When** the action's edit list is inspected
**Then** edits are ordered by `location.start` in descending order (highest offset first)
**And** applying edits in this order produces the correct result without offset shifting

### AC5: Duplicate pragmas are deduplicated
**Given** a Perl file where multiple diagnostics would add `use strict` (PL100, PL502)
**When** the `SourceFixAll` action is created
**Then** `use strict` appears exactly once in the merged edits

### AC6: Only preferred fixes are included
**Given** a diagnostic with multiple fix options (e.g., "Declare with my" and "Declare with our")
**When** the `SourceFixAll` action is created
**Then** only the preferred fix (`is_preferred: true`) is included

### AC7: Kind filtering works correctly
**Given** a client sends `context.only: ["source.fixAll"]`
**When** the server processes the code action request
**Then** only the `SourceFixAll` action is returned (individual quick fixes are filtered out)

## Non-Goals

1. **Not implementing `source.organizeImports`** — Already exists in `EnhancedCodeActionsProvider`
2. **Not implementing `source.modernize`** — Already exists in `modernize.rs`
3. **Not including Perl::Critic fixes** — These are handled separately with different infrastructure
4. **Not supporting cross-file fixes** — Only single-file diagnostics are included

## Dependencies

- `perl_lsp_code_actions::types::CodeActionKind::SourceFixAll` (already exists)
- `perl_lsp_code_actions::types::CodeActionKind::SourceFixAll` mapping to `"source.fixAll"` in LSP handler (already exists)
- `CodeActionsProvider::get_code_actions()` quick fix match logic (reused)
- Existing `TextEdit`, `SourceLocation`, `CodeActionEdit` types
- Existing `to_quick_fix_diagnostic` conversion function

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-lsp-code-actions/src/code_actions.rs` | Add `get_fix_all_actions` method |
| `crates/perl-lsp/src/runtime/language/code_actions.rs` | Call `get_fix_all_actions` and add action to response |
| `crates/perl-lsp-code-actions/tests/` | Add unit tests |
| `crates/perl-lsp/tests/` | Add integration tests |

## Test Cases

1. **Multiple diagnostics with fixes** → single SourceFixAll with all edits merged
2. **No diagnostics** → no SourceFixAll action returned
3. **Diagnostics without fixes** → no SourceFixAll action returned
4. **Duplicate pragma inserts** → deduplicated to single `use strict`/`use warnings`
5. **Edit ordering** → edits sorted by descending offset
6. **Preferred fix only** → non-preferred options excluded
7. **Kind filtering** → `context.only: ["source.fixAll"]` returns only SourceFixAll
8. **Empty result after deduplication** → no action returned
