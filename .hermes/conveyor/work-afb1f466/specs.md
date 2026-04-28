# Spec: Unreachable Code Detection in Continue Blocks

## Feature Description

Extend the PL406 unreachable code detector to analyze `continue` blocks attached to `while`/`until`/`for`/`foreach` loops. When an unconditional exit (`die`, `exit`, `croak`, `Carp::croak`, `confess`, `Carp::confess`, `return`, `last`) appears inside a continue block, any statements following it in that same continue block are unreachable and should be flagged.

## Background

In Perl, `continue` blocks are attached to loops and execute after each iteration:

```perl
while (1) {
    # loop body
} continue {
    # continue block — runs after each iteration
}
```

Code inside a continue block after an unconditional exit is unreachable:
- `die "msg"; print "unreachable";` → `print` is unreachable
- `exit(0); print "unreachable";` → `print` is unreachable
- `croak "msg"; print "unreachable";` → `print` is unreachable
- `return; print "unreachable";` → `print` is unreachable (if continue block were part of a sub)
- `last; print "unreachable";` → `print` is unreachable (last exits the entire loop)

However, `next` and `redo` in continue blocks are **NOT** unconditional exits from the continue block's statement list:
- `next; print "reachable";` → `print` is reachable (next jumps to the next iteration, which re-runs the continue block)
- `redo; print "reachable";` → `print` is reachable (redo re-runs the continue block)

## Acceptance Criteria

### Functional Criteria

1. **Continue block with `die` followed by statement**: `while (1) { } continue { die "err"; print "dead"; }`
   - Expect: exactly 1 PL406 diagnostic on the `print` statement
   - Criterion: AC-1

2. **Continue block with `exit` followed by statement**: `while (1) { } continue { exit(0); print "dead"; }`
   - Expect: exactly 1 PL406 diagnostic on the `print` statement
   - Criterion: AC-2

3. **Continue block with `croak` followed by statement**: `while (1) { } continue { croak "err"; print "dead"; }`
   - Expect: exactly 1 PL406 diagnostic on the `print` statement
   - Criterion: AC-3

4. **Continue block with `last` followed by statement**: `while (1) { } continue { last; print "dead"; }`
   - Expect: exactly 1 PL406 diagnostic on the `print` statement (last exits the loop entirely)
   - Criterion: AC-4

5. **Continue block with `return` followed by statement** (when continue block is in a sub):
   - Expect: exactly 1 PL406 diagnostic on the `print` statement
   - Criterion: AC-5

6. **`next` in continue block followed by statement — NO false positive**: `while (1) { } continue { next; print "reachable"; }`
   - Expect: 0 PL406 diagnostics (next jumps to the next iteration, continue block re-runs)
   - Criterion: AC-6

7. **`redo` in continue block followed by statement — NO false positive**: `while (1) { } continue { redo; print "reachable"; }`
   - Expect: 0 PL406 diagnostics (redo re-runs the continue block)
   - Criterion: AC-7

8. **Multiple unreachable statements in continue block**: `while (1) { } continue { die "err"; my $x = 1; my $y = 2; print "dead"; }`
   - Expect: 3 PL406 diagnostics (one each for `$x`, `$y`, and `print`)
   - Criterion: AC-8

9. **Loop body unreachable detection unchanged**: `while (1) { next if $cond; die "err"; print "dead"; }`
   - Expect: 1 PL406 diagnostic on `print` in the loop body
   - Criterion: AC-9

10. **All four loop types covered**: `while`, `until`, `for`, and `foreach` with continue blocks
    - Criterion: AC-10

### Non-Goals

- This fix does NOT add detection for `goto` labels (documented pre-existing false negative, N7 test)
- This fix does NOT add detection for `die` inside `or` expression (right operand of Binary, not a direct statement)
- This fix does NOT add detection for conditional exits via `StatementModifier` (e.g., `return if $cond`)
- This fix does NOT change how `eval { }` blocks are handled (intentionally not recursed into)

### Dependencies

- `perl-ast` crate: `NodeKind::While`, `NodeKind::For`, `NodeKind::Foreach` all have `continue_block: Option<Box<Node>>`
- `perl-parser-core`: Parser correctly builds `continue_block` nodes for all four loop types
- No new dependencies required

## Implementation Notes

### File to modify
`/crates/perl-lsp-diagnostics/src/lints/unreachable_code.rs`

### Changes required

1. **Add `check_continue_block` function** (lines after `check_statement_list`):
   - Dispatches on `NodeKind::Block { statements }` and calls `check_statement_list_with_exit_check(statements, is_continue_block_exit, diagnostics)`

2. **Add `is_continue_block_exit` function** (after `is_unconditional_exit`):
   - Same as `is_unconditional_exit` but excludes `next` and `redo` from the LoopControl match arm

3. **Update loop match arm** (lines 90–95):
   - Extract `continue_block` from the pattern
   - Call `check_continue_block(continue_block, diagnostics)` when `continue_block.is_some()`

4. **Add unit tests** in `perl-lsp-diagnostics/tests/unreachable_code_tests.rs`:
   - Helper `while_loop_with_continue(body, continue_block)` that creates a `While` node with `continue_block: Some(Box::new(continue_block))`
   - Tests T-continue-1 through T-continue-8 covering all acceptance criteria
   - Negative test N-continue-1 for `next` in continue block (no false positive)
   - Negative test N-continue-2 for `redo` in continue block (no false positive)

### Verification

- All existing tests must continue to pass (`cargo test -p perl-lsp-diagnostics`)
- All new tests must pass
- `cargo clippy -p perl-lsp-diagnostics` must report no new warnings
