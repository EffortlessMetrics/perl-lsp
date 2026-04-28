# ADR-406: Unreachable Code Detection in Continue Blocks

## Status
Proposed

## Context

The unreachable code detector (PL406) in `perl-lsp-diagnostics` has a gap: it does not analyze `continue` blocks attached to `while`/`until`/`for`/`foreach` loops. In Perl, a `continue` block executes after each iteration of the loop body (unless the loop was exited via `last`), but code inside the continue block after an unconditional exit (`die`, `return`, `exit`, `croak`, `confess`, `last`) is itself unreachable and should be flagged.

**Example of missing detection:**
```perl
while (1) {
    next if $condition;
} continue {
    die "error";  # exit - this is reachable
    print "unreachable";  # NOT detected — this SHOULD be PL406
}
```

The root cause is in `unreachable_code.rs` lines 90–95: the loop match arms use `body, ..` which silently drops the `continue_block` field.

## Decision

Implement a targeted fix that:

1. **Adds continue block recursion to `visit_node`** in `unreachable_code.rs` for `NodeKind::While`, `NodeKind::For`, and `NodeKind::Foreach`.

2. **Uses a context-aware exit predicate** specifically for continue blocks. The existing `is_unconditional_exit()` returns `true` for `next` and `redo`, which is correct for loop bodies but **incorrect for continue blocks**:
   - In a **loop body**: `next` and `redo` make subsequent statements unreachable
   - In a **continue block**: `next` jumps to the next iteration (which re-runs the continue block), and `redo` re-runs the continue block — so subsequent statements are reachable

3. **Introduces two new private functions**:
   - `check_continue_block(node: &Node, diagnostics: &mut Vec<Diagnostic>)` — dispatches on the continue block's `NodeKind::Block { statements }` and calls `check_statement_list_with_exit_check` with the continue-block-specific predicate
   - `is_continue_block_exit(node: &Node) -> bool` — returns `true` for `die`, `exit`, `croak`, `Carp::croak`, `confess`, `Carp::confess`, `return`, and `last` but **NOT** `next` or `redo`

4. **Updates the loop match arm** to also recurse into `continue_block` when present:
   ```rust
   NodeKind::While { body, continue_block, .. }
   | NodeKind::For { body, continue_block, .. }
   | NodeKind::Foreach { body, continue_block, .. } => {
       visit_node(body, diagnostics);
       if let Some(cont) = continue_block {
           check_continue_block(cont, diagnostics);
       }
   }
   ```

## Consequences

### Tradeoffs

**Benefits:**
- Closes the detection gap for unreachable code in continue blocks
- Minimal, targeted change that preserves the existing statement-slice analysis algorithm
- Correctly handles the `next`/`redo` edge case (no false positives)

**Risks:**
- Requires adding a second exit-predicate function with overlapping logic with `is_unconditional_exit()`
- Could introduce subtle differences in how `eval { }` inside continue blocks is handled (intentionally not recursed into, same as elsewhere)

### Alternatives Considered

**Alternative 1: Parameterized `is_unconditional_exit` with `in_continue_block: bool`**
- Pro: Single function, no duplication
- Con: More complex API; the predicates are different enough (one includes `next`/`redo`, the other excludes them) that separate functions are cleaner

**Alternative 2: Naive one-line fix (`visit_node(continue_block, diagnostics)`)**
- Pro: Minimal code change
- Con: Causes false positives for `next`/`redo` in continue blocks, as `is_unconditional_exit()` treats them as unconditional exits. This was the plan's initial proposal and was rejected by the verification and plan-review agents.

**Alternative 3: Add `in_continue_block` flag to `check_statement_list`**
- Pro: Single function
- Con: More complex API; requires threading the flag through all call sites; the exit predicates differ enough that separation is cleaner

## Technical Notes

- `continue_block` is `Option<Box<Node>>` where the inner `Node`'s `kind` is `NodeKind::Block { statements }`
- Both `while` and `until` loops produce `NodeKind::While` nodes (the condition is negated for `until`); the fix covers both automatically
- `last` in a continue block IS an unconditional exit (it exits the entire loop, so the continue block won't be re-entered)
- `next` and `redo` in a continue block are NOT exits from the continue block's statement list (they jump to the next iteration which re-runs the continue block)
- Existing tests must continue to pass; `while_loop()` test helper creates `continue_block: None`, so no existing tests exercise continue blocks
