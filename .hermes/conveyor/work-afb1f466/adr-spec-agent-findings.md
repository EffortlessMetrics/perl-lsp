# ADR/Spec Findings — work-afb1f466

## What This ADR Decides

The ADR formalizes the architecture for extending PL406 unreachable code detection to cover `continue` blocks attached to `while`/`until`/`for`/`foreach` loops. It chooses a **context-aware exit predicate approach** — a separate `is_continue_block_exit()` function that excludes `next` and `redo` — over alternative approaches like adding a boolean parameter to the existing function.

## Key Decision

**Add a dedicated `check_continue_block()` function with its own `is_continue_block_exit()` predicate**, rather than:
- Parameterizing `is_unconditional_exit` with an `in_continue_block` flag
- Using the naive `visit_node(continue_block, diagnostics)` approach (which would cause false positives)

This separation is necessary because `next` and `redo` have different semantics in continue blocks vs. loop bodies:
- In loop bodies: `next`/`redo` make subsequent statements unreachable
- In continue blocks: `next` jumps to the next iteration (re-running the continue block), `redo` re-runs the continue block — so subsequent statements are reachable

## Alternatives Considered

1. **Parameterized `is_unconditional_exit(in_continue_block: bool)`**: Single function with a boolean flag. Rejected because the predicates are different enough that separate functions are cleaner and more maintainable.

2. **Naive one-line fix**: `visit_node(continue_block, diagnostics)`. Rejected by verification and plan-review agents because `is_unconditional_exit` returns `true` for `next` and `redo`, causing false positives in continue blocks.

3. **Add `in_continue_block` flag to `check_statement_list`**: Rejected because it requires threading the flag through all call sites and the exit predicates differ enough that separation is cleaner.

## Consequences

**Benefits**:
- Closes the detection gap for unreachable code in continue blocks
- Minimal, targeted change preserving the statement-slice analysis algorithm
- No false positives for `next`/`redo` in continue blocks

**Tradeoffs**:
- Duplicates some logic from `is_unconditional_exit` (but with a critical difference: excludes `next`/`redo`)
- Adds two new private functions to the unreachable_code module

## Acceptance Criteria

The specs define 10 acceptance criteria (AC-1 through AC-10) covering:
- AC-1 to AC-5: Unconditional exits in continue blocks (`die`, `exit`, `croak`, `last`, `return`) trigger PL406 on subsequent statements
- AC-6 to AC-7: `next` and `redo` in continue blocks do NOT trigger false positives
- AC-8: Multiple unreachable statements in continue blocks are all flagged
- AC-9: Loop body detection remains unchanged
- AC-10: All four loop types (while, until, for, foreach) are covered

## Friction Log

- **Underspecified fix for next/redo edge case**: The initial plan identified the false-positive risk but didn't provide concrete implementation details. The ADR resolves this by specifying the exact function signatures and exit predicates.
- **AST accessor confusion**: The verification comment suggested calling `block_statements()` on NodeKind, but that method doesn't exist — the correct approach is to pattern-match on `NodeKind::Block { statements }` inside `check_continue_block()`.
