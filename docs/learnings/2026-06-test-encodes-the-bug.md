---
tags: [test-quality, dap, debugger, test-encodes-bug, stack-frames]
repos: [perl-lsp-swarm]
related: ["#964", "#933", "#1337"]
portable: false
article_asset: true
search_terms: [test_stack_trace_uses_recent_output_when_available, test_stack_trace_returns_empty_without_live_session, stack_frames_stale_resume_tests, frames.len, snapshot_buffer, handle_stack_trace, frames.rs, test-encodes-bug]
---

# Pre-existing test asserted the stale-frames defect as expected behavior

**Date**: 2026-06
**Hazard class**: test-encodes-bug
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 5)

## What happened

PR #1337 fixed the stale-stack-frames bug (#964): stack frames were never cleared on
resume, so stackTrace requests between resume and the next stopped event served the
previous stop frames verbatim. A pre-existing test
test_stack_trace_uses_recent_output_when_available asserted frames.len() >= 2 --
directly testing the now-removed snapshot-buffer parsing behavior that was the bug.
The fix changed frames.len() to 0 in the degraded path, causing the old test to fail
with correct code in place.

## Why

The test was written to characterize existing behavior at the time it was authored.
At that time the snapshot-buffer path was the only path; "returns 2 frames" was the
observable output. The assertion captured what happened, not what should happen.
When the bug is a plausible-looking output (2 frames rather than 0), the test looks
reasonable until the fix is applied.

## Fix

Update the test to assert the correct post-fix behavior: frames.len() == 0. Rename to
test_stack_trace_returns_empty_without_live_session. Mark in the commit message: "was
testing the bug." Add a new test test_stack_trace_does_not_use_snapshot_in_degraded_path
that seeds parseable snapshot text and asserts no frames are returned.

## Spec impact

Motivated Class 5 (Test Encodes the Bug) in docs/concepts/hazard-class-invariants.md.
Added checklist item to docs/agents/SPEC_UPDATE_CHECKLIST.md section 8.

## Portable lesson

Tests that characterize existing behavior capture the bug when the existing behavior IS
the bug. The signal: fixing the bug causes a pre-existing test to fail. The correct
response is to update the test (asserting correct behavior) and mark it clearly.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 5 -- Test Encodes the Bug
- **Generalization**: When a bug fix breaks a test, read the old assertion -- the test may be asserting the defect.

## Related PRs

- [#1337](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1337) -- fix: clear stack_frames on resume
- [#964](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/964) -- issue: stack_frames never cleared on resume
- [#933](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/933) -- issue: degraded-transport fallback returned stale first frame
