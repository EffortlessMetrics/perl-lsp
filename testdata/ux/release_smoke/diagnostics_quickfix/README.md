# Diagnostics Quick Fix

## Scenario

A user opens code with undefined loop-control labels. The diagnostics should
point at the label spans and code actions should offer one safe mechanical edit:
remove the undefined label from `next`, `last`, or `redo`.

## Files

- `loop_label.pl` - contains missing labels for `next`, `last`, and `redo`, plus
  a defined-label negative case.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen loop_label.pl
diagnostics receipt for PL410
textDocument/codeAction over each undefined label diagnostic
textDocument/codeAction over the defined-label line
shutdown
```

## Expected Behavior

- PL410 diagnostics are emitted for `MISSING_NEXT`, `MISSING_LAST`, and
  `MISSING_REDO`.
- Each PL410 diagnostic offers exactly one safe action titled
  `Remove undefined label`.
- The edit transforms `next MISSING_NEXT` to `next`, `last MISSING_LAST` to
  `last`, and `redo MISSING_REDO` to `redo`.
- No remove-label action is offered for the defined label.

## Non-Goals

The fixture intentionally does not claim an "add label" quick fix. Inserting a
label safely requires AST-guided placement and belongs to a separate feature.
