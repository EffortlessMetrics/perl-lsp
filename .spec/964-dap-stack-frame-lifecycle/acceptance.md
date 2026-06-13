# Acceptance Criteria: DAP Stack-Frame Lifecycle Fix (#964 + #933)

## Functionality

- [ ] handle_continue() clears session.stack_frames
- [ ] handle_next() clears session.stack_frames
- [ ] handle_step_in() clears session.stack_frames
- [ ] handle_step_out() clears session.stack_frames
- [ ] handle_pause() clears session.stack_frames
- [ ] handle_goto() clears session.stack_frames
- [ ] Degraded-transport fallback in handle_stack_trace returns Vec::new() (no snapshot parsing)
- [ ] stackTrace request between resume and next stopped event returns empty frames (no stale state)

## Unit Tests

- [ ] test_handle_continue_clears_stack_frames passes
- [ ] test_handle_next_clears_stack_frames passes
- [ ] test_handle_step_in_clears_stack_frames passes
- [ ] test_handle_step_out_clears_stack_frames passes
- [ ] test_handle_pause_clears_stack_frames passes
- [ ] test_stack_trace_does_not_use_snapshot_in_degraded_path passes

## Code Quality

- [ ] No unwrap(), expect(), panic!(), todo!(), unimplemented!(), dbg!() in production code
- [ ] cargo fmt passes
- [ ] cargo clippy passes on perl-dap
- [ ] cargo test passes on perl-dap
- [ ] All tests pass with RUST_TEST_THREADS=1 (DAP threading gate)

## Policy and RIPR

- [ ] ripr-suppress-dap-stack-frame-lifecycle entry exists in policy/ripr-suppressions.toml
- [ ] Suppression entry has correct format (id, kind, paths, classification, owner, issue, reason, created, review_after, expires)
- [ ] Suppression cites EffortlessMetrics/ripr#1429 and EffortlessMetrics/ripr#1428
- [ ] Expires date is 2026-09-30
- [ ] RIPR receipt (post-merge) shows suppressed_by_policy includes all three DAP files
- [ ] RIPR receipt shows severe_gaps → 0 (parser fix #1336 on main applies the suppression)

## Integration

- [ ] cargo test --workspace --lib passes
- [ ] cargo xtask fmt passes
- [ ] cargo clippy --workspace passes
- [ ] No regressions in other DAP tests
- [ ] No conflicts with concurrent DAP improvements

## Clean Separation

- [ ] Implementation is on impl/964-dap-stack-frame-lifecycle-clean branch (off origin/main)
- [ ] No commits from PR #1309 are reused (clean re-creation)
- [ ] No apply-review-suppression commands in commit history
- [ ] Spec files exist at .spec/964-dap-stack-frame-lifecycle/checklist.md, acceptance.md, context.md
