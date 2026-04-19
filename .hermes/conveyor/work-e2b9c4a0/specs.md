# Specification: Missing Label Resolution Validation for Loop Control

## Feature Description

Validate that loop control statements (`next`, `last`, `redo`) with labels reference labels that exist in the current file. When a label is not found in the symbol table, emit a diagnostic with severity Warning and code PL410 (LoopLabelUndefined).

**Example:**
```perl
# No label "OUTER" is defined
next OUTER;  # Diagnostic: "next label 'OUTER' is not defined in this file"
last OUTER;  # Diagnostic: "last label 'OUTER' is not defined in this file"
redo OUTER;  # Diagnostic: "redo label 'OUTER' is not defined in this file"
```

## Behavior

### Valid Case
```perl
OUTER: for my $i (1..10) {
    next OUTER if $i > 5;  # Label exists → no diagnostic
}
```

### Invalid Case
```perl
next OUTER;  # No OUTER label defined anywhere in the file
```

### No-Label Case
```perl
next;  # No label → no diagnostic (valid Perl, targets nearest enclosing loop)
```

## Acceptance Criteria

1. **Diagnostic emitted for undefined loop label**: When `next LABEL`, `last LABEL`, or `redo LABEL` references a label that does not exist in the current file, a diagnostic is emitted with:
   - Severity: Warning
   - Code: PL410 (LoopLabelUndefined)
   - Message: `"<op> label '<label>' is not defined in this file"` (e.g., `"next label 'OUTER' is not defined in this file"`)

2. **No diagnostic for valid label**: When the label exists in the file (defined via `LABEL: statement`), no diagnostic is emitted.

3. **No diagnostic for label-less loop control**: `next;`, `last;`, `redo;` without a label do not trigger any diagnostic.

4. **Consistent with goto validation pattern**: The lint implementation follows the same architecture as `goto_label.rs`:
   - New lint function `check_loop_labels` in `crates/perl-lsp-diagnostics/src/lints/loop_label.rs`
   - `has_label` helper shared from `goto_label.rs` (made `pub(crate)`)
   - Called from `diagnostics.rs` alongside `check_goto_labels`

## Non-Goals

- **Loop target validation**: This spec does NOT require validating that the label points to a loop. Perl's `goto` can target any label (even non-loops), and for consistency, we only validate label existence.
- **Cross-file label resolution**: Labels are file-local in Perl; this spec does not cover cross-file references.
- **Parser changes**: Syntax parsing is already correct; no changes to the parser are needed.
- **Scope analyzer changes**: The lint approach is used specifically to avoid modifying the scope analyzer.

## Implementation Details

### Files to Create
- `crates/perl-lsp-diagnostics/src/lints/loop_label.rs` — New lint implementation

### Files to Modify
- `crates/perl-lsp-diagnostics/src/lints/goto_label.rs` — Change `has_label` from `fn` to `pub(crate) fn`
- `crates/perl-lsp-diagnostics/src/lints/mod.rs` — Add `pub mod loop_label;` and documentation
- `crates/perl-lsp-diagnostics/src/diagnostics.rs` — Add `use crate::lints::loop_label::check_loop_labels;` and call it
- `crates/perl-diagnostics/src/codes/mod.rs` — Add `LoopLabelUndefined` variant and implement all required trait methods for PL410

### Diagnostic Code PL410
- **Name**: `LoopLabelUndefined`
- **Severity**: Warning
- **Message template**: `"<op> label '<label>' is not defined in this file"`
- **URL**: `https://docs.perl-lsp.org/errors/PL410`

## Dependencies

- `perl-parser-core` — AST types (`Node`, `NodeKind::LoopControl`)
- `perl-semantic-analyzer` — Symbol table (`SymbolTable`, `SymbolKind::Label`)
- `perl-diagnostics` — Diagnostic code definitions
- `perl-lsp-diagnostics` — Lint infrastructure (walker, `has_label` helper)

## Test Scenarios

| Scenario | Input | Expected |
|----------|-------|----------|
| Valid label | `OUTER: for (...) { next OUTER; }` | No diagnostic |
| Undefined next | `next OUTER;` (no label) | Warning PL410 |
| Undefined last | `last OUTER;` (no label) | Warning PL410 |
| Undefined redo | `redo OUTER;` (no label) | Warning PL410 |
| No label | `next; last; redo;` | No diagnostic |
| Goto unaffected | `goto FOO;` (no label) | Still uses PL409 |
