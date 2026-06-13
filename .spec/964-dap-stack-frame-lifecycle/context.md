# Context: DAP Stack-Frame Lifecycle Fix (#964 + #933)

## Issues

**#964**: `DAP stackTrace: session.stack_frames never cleared on resume — stale frames served after continue/step`

Every execution/resume handler clears `session.variable_cache` but never clears `session.stack_frames`, so a `stackTrace` arriving after a resume (before new frames are parsed) serves frames from the *previous* stop. Impact: a client calling `stackTrace` in the window between a resume response and the next `stopped` event (routine during stepping) sees the previous stop's line/file, and subsequent `scopes`/`variables` queries report values for the wrong program location.

**#933**: `fix(dap): degraded-transport fallback in handle_stack_trace can return stale first frame`

When `send_framed_debugger_commands` fails (degraded transport state), `handle_stack_trace` fell back to parsing the recent-output snapshot buffer directly. Because the buffer is ordered by **arrival time**, the stale pre-stop context line appeared before the current-stop line, making the first returned frame the wrong line. Low-frequency impact (only reached when transport fails), but a correctness latent.

## Validated Approach

Both issues have undergone deep review in the context of PR #1309 (which attempted to fix them but became tangled with multiple agents' work). The **fix logic is validated and correct**; this spec ensures a **clean re-creation** without inheriting the PR #1309 accretion.

### #964 Fix: Clear stack_frames on resume

**File**: `crates/perl-dap/src/debug_adapter/execution.rs`

All 6 resume handlers (`handle_continue`, `handle_next`, `handle_step_in`, `handle_step_out`, `handle_pause`, `handle_goto`) must call `session.stack_frames.clear()` alongside the existing `session.variable_cache.clear()`. This mirrors the variable cache pattern and ensures the next `stackTrace` request (before the next `stopped` event) has no stale frames to return.

**Lines to change**:
- handle_continue: line 28
- handle_next: line 74
- handle_step_in: line 112
- handle_step_out: line 151
- handle_pause: line 191
- handle_goto: line 364

Each is a one-liner: add `session.stack_frames.clear();` after `session.variable_cache.clear();`.

### #933 Fix: Degrade gracefully on transport failure

**File**: `crates/perl-dap/src/debug_adapter/frames.rs`

The degraded-transport fallback path (`else` branch at lines 66-74) currently attempts to parse the snapshot buffer. Instead, return `Vec::new()` so the caller falls through to the authoritative `session.stack_frames` (populated by the output reader). The buffer is unreliable because it contains the full session history (initial implicit stop + current stop in order of arrival), so parsing returns the stale frame first.

The comment in the code (lines 48-61) already explains why snapshot parsing is unreliable for the framed path; the degraded path now mirrors that reasoning.

## Test Helpers

Three new test helpers in `crates/perl-dap/src/debug_adapter/mod.rs` under `#[cfg(test)]` (inside the impl block):

1. **seed_session_for_test()** — creates a minimal mock session with a fake process and piped stdin. Required setup for all handler tests.
2. **inject_stack_frames_for_test(Vec<StackFrame>)** — replaces session.stack_frames with provided frames. Simulates a previous stop's frame state.
3. **stack_frames_snapshot_for_test() -> Vec<StackFrame>** — returns current session.stack_frames. Assertion helper for verifying clear.

These are marked `pub` under `#[cfg(test)]` so they're accessible only in test context but don't pollute the production API.

## Unit Tests

**In `crates/perl-dap/src/debug_adapter/mod.rs` (under `#[cfg(test)] mod tests`):**

5 handler tests, one per resume handler. Each:
1. Seeds a session (precondition)
2. Injects 2 test frames
3. Calls the handler (e.g., `handle_continue(1, 1, None)`)
4. Asserts `stack_frames_snapshot_for_test().len() == 0`

Pattern:
```rust
#[test]
fn test_handle_continue_clears_stack_frames() -> Result<(), Box<dyn std::error::Error>> {
    if !has_perl_executable() { 
        eprintln!("Skipping — perl not available"); 
        return Ok(()); 
    }
    let adapter = DebugAdapter::new();
    adapter.seed_session_for_test();
    adapter.inject_stack_frames_for_test(vec![make_test_frame(1), make_test_frame(2)]);
    assert_eq!(adapter.stack_frames_snapshot_for_test().len(), 2);
    let _response = adapter.handle_continue(1, 1, None);
    assert_eq!(adapter.stack_frames_snapshot_for_test().len(), 0);
    Ok(())
}
```

**In `crates/perl-dap/src/debug_adapter/parsing.rs`:**

1 degraded-path test:
- Mock `framed_output_lines = None` (transport failure)
- Call `handle_stack_trace` with a mock session that has stale frames
- Assert the response contains empty frames (not snapshot-parsed frames)

This verifies the `else` branch returns `Vec::new()` and falls through to the authoritative `session.stack_frames`.

## RIPR Suppression

**File**: `policy/ripr-suppressions.toml`

**ONE entry** (reuses format from existing suppressions in the file):

```toml
[[suppress]]
id = "ripr-suppress-dap-stack-frame-lifecycle"
kind = "predicate_infection_untraceable"
paths = [
  "crates/perl-dap/src/debug_adapter/execution.rs",
  "crates/perl-dap/src/debug_adapter/frames.rs",
  "crates/perl-dap/src/debug_adapter/mod.rs",
]
classification = ["reachable_unrevealed", "weakly_gripped", "call_presence"]
owner = "proof-lane"
issue = "EffortlessMetrics/ripr#1429"
reason = "..."
created = "2026-06-12"
review_after = "2026-07-12"
expires = "2026-09-30"
```

**Rationale**: The changes introduce ripr#1429-class seams:
- **execution.rs**: 6 resume handlers each call `lock_or_recover()` and `stack_frames.clear()`; the `Some` branch cannot be traced statically (Mutex availability is runtime-only)
- **frames.rs**: degraded-path `Vec::new()` is reachable (guarded by `if let Some(lines) = framed_output_lines`), but static analysis cannot confirm test coverage through the conditional
- **mod.rs**: three new test-helper methods (`seed_session_for_test`, `inject_stack_frames_for_test`, `stack_frames_snapshot_for_test`) inside `#[cfg(test)]` blocks in the production impl; ripr#1428-class false-positives where diff-scoped analysis treats newly added #[cfg(test)] methods as production seams

**Citations**:
- EffortlessMetrics/ripr#1429 (predicate-infection-untraceable): RIPR cannot statically trace activation through string-literal matches, runtime predicates, or Mutex guards
- EffortlessMetrics/ripr#1428 (test-harness false-positives): RIPR treats newly added #[cfg(test)] methods in production impl blocks as production seams

The suppression is scoped to these three files and expires 2026-09-30 (standard 3-month window).

**Parser fix #1336**: The fix on main (#1336) enables RIPR to apply suppressions via PR evidence. This entry will be validated post-merge.

## Prior Tangled Work: PR #1309

PR #1309 attempted to fix both issues but became tangled:
- Multiple agents' commits accreted (parser work, ripr evidence fixes, conformance tests)
- Includes an erroneous `apply-review-suppression` command
- Stale features (e.g., snapshot fallback logic refactoring that wasn't needed)

**This spec is a clean re-creation**, not reuse. Do not cherry-pick PR #1309 commits.

## Dup Issues to Close Post-Merge

Once this PR lands and merges, close the following as "Superseded by PR #XXXX":
- #1216 (earlier DAP stale-state attempt)
- #1325 (concurrent cleanup attempt)
- #1247 (concurrent rework)

These were parallel discovery passes that are now obsoleted by this comprehensive fix.

## Why This Matters

The fix addresses a **user-visible correctness bug**: during normal stepping, a client could see wrong file/line information and inspect variables at the wrong program location. The window is narrow (between resume response and next stop), but it's predictable during rapid stepping or breakpoint-to-breakpoint debugging. Users may have filed issues that trace to this root cause.

## Design Decisions

**Why clear stack_frames, not snapshot parsing?**
- `session.stack_frames` is the authoritative cache, updated by the output reader as frames are parsed
- The snapshot buffer is a last-resort fallback, not a reliable source of truth
- Clearing frames on resume ensures the next `stackTrace` request waits for the output reader to populate the new frames
- Mirrors the existing `variable_cache.clear()` pattern

**Why return Vec::new() in degraded path, not try parsing?**
- The snapshot buffer is ordered by arrival time, not by logical stop order
- The initial implicit-stop context line appears before the current-stop context line
- Parsing in that order returns the stale frame first
- Returning empty causes the caller to fall through to `session.stack_frames`, which the output reader has already populated correctly

**Why three test helpers?**
- `seed_session_for_test`: minimal, repeatable session setup (avoids spawning real perl in unit tests)
- `inject_stack_frames_for_test`: simulates previous stop state (the bug scenario)
- `stack_frames_snapshot_for_test`: inspection helper (enables concise assertions)

## Related Reads

- Issue #964 discovery: `execution.rs` grep confirms no stack_frames.clear() anywhere
- Issue #933 discovery: deep review of #927 (stackTrace off-by-one fix)
- #1336 (Parser fix): enables RIPR suppression application via PR evidence
- CLAUDE.md: DAP lane phase-2 roadmap includes stack/scopes/variables hardening

## Verification Gates

1. **Compile gate**: `cargo build -p perl-dap`
2. **Lint gate**: `cargo clippy -p perl-dap`
3. **Unit test gate**: `cargo test -p perl-dap` (with `RUST_TEST_THREADS=1` for DAP threading)
4. **Workspace gate**: `cargo test --workspace --lib`
5. **Format gate**: `cargo xtask fmt`
6. **RIPR gate** (post-merge): suppression entry applies, `severe_gaps` → 0

---

## Summary

This spec provides a **clean, builder-ready roadmap** to implement the validated fix from #964 and #933. The approach has undergone deep review and is correct; this ensures a straightforward build without the accretion that tangled PR #1309.
