# Task List — work-afb1f466: Unreachable Code Detection in Continue Blocks

## Implementation Tasks

- [ ] 1. **Add `is_continue_block_exit()` function** to `unreachable_code.rs`
  - Mirrors `is_unconditional_exit()` but excludes `next` and `redo` from the LoopControl match arm
  - Returns `true` for: `die`, `exit`, `croak`, `Carp::croak`, `confess`, `Carp::confess`, `return`, `last`
  - Returns `false` for: `next`, `redo` (these re-run the continue block, not exit it)

- [ ] 2. **Add `check_continue_block()` function** to `unreachable_code.rs`
  - Dispatches on `NodeKind::Block { statements }` and calls `check_statement_list_with_exit_check`
  - Uses `is_continue_block_exit` as the exit predicate

- [ ] 3. **Update loop match arm in `visit_node()`** (lines 90–95 of `unreachable_code.rs`)
  - Extract `continue_block` from `NodeKind::While`, `NodeKind::For`, `NodeKind::Foreach` patterns
  - Call `check_continue_block(continue_block, diagnostics)` when `continue_block.is_some()`

- [ ] 4. **Add unit test helper `while_loop_with_continue(body, continue_block)`**
  - In `unreachable_code_tests.rs`
  - Creates a `While` node with `continue_block: Some(Box::new(continue_block))`

- [ ] 5. **Add positive test cases** (expect PL406 diagnostics):
  - T-continue-1: `die` in continue block followed by statement → 1 PL406
  - T-continue-2: `exit` in continue block followed by statement → 1 PL406
  - T-continue-3: `croak` in continue block followed by statement → 1 PL406
  - T-continue-4: `last` in continue block followed by statement → 1 PL406
  - T-continue-5: `return` in continue block followed by statement → 1 PL406
  - T-continue-6: `confess` in continue block followed by statement → 1 PL406
  - T-continue-7: Multiple unreachable statements in continue block → N PL406
  - T-continue-8: `die` in continue block with `for` loop → 1 PL406
  - T-continue-9: `die` in continue block with `foreach` loop → 1 PL406

- [ ] 6. **Add negative test cases** (expect 0 PL406 diagnostics):
  - N-continue-1: `next` in continue block followed by statement → 0 PL406
  - N-continue-2: `redo` in continue block followed by statement → 0 PL406

- [ ] 7. **Verify existing tests still pass**
  - Run `cargo test -p perl-lsp-diagnostics`
  - All existing tests must continue to pass

- [ ] 8. **Run clippy**
  - Run `cargo clippy -p perl-lsp-diagnostics`
  - No new warnings introduced

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-lsp-diagnostics/src/lints/unreachable_code.rs` | Add `is_continue_block_exit()`, `check_continue_block()`, update `visit_node()` loop arm |
| `crates/perl-lsp-diagnostics/tests/unreachable_code_tests.rs` | Add test helper and 11 new test cases |

## Verification Commands

```bash
cargo test -p perl-lsp-diagnostics
cargo clippy -p perl-lsp-diagnostics
```
