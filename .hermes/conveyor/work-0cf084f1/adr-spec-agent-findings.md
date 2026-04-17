# ADR/Spec Findings — work-0cf084f1

## What This ADR Decides
This ADR formally documents the architecture decision made for issue #4194: how to surface
permission-denied file errors during background workspace indexing. The core decision is to
emit a one-time `window/showMessage` warning (gated by `compare_exchange` on `AtomicBool`) plus
a per-file `textDocument/publishDiagnostics` diagnostic for each unreadable file.

## Key Decision
The implementation uses an `Arc<AtomicBool>` one-time guard for the `window/showMessage` warning
(so only the first thread to encounter a permission error sends the notification), while the
per-file diagnostics fire for every affected file without gating. This matches Option A + B
as recommended in the issue, and was already implemented in `workspace.rs:1613-1670` (merged
in commit `a7344b84`).

## Alternatives Considered
1. **Per-file showMessage (rejected)**: Would spam users when many files are unreadable.
2. **LSP window/showDocument (rejected)**: Disproportionate complexity for the use case.
3. **Diagnostics-only, no showMessage (rejected)**: Issue explicitly requested both.

## Consequences
- Users are notified on first permission error and know exactly which files are affected
- Root user caveat: Cannot test permission features as root on Unix
- Test timing race: Tests use fixed `sleep(3s)` before draining notifications — possible CI flakiness
- Issue description incorrectly references `text_sync.rs` instead of `workspace.rs`

## Acceptance Criteria
1. One-time `window/showMessage` fires on first permission error, suppressed thereafter
2. Per-file `textDocument/publishDiagnostics` fires for each unreadable file
3. Cross-platform: Unix `PermissionDenied` + Windows os error 5 both handled
4. Graceful degradation: readable files still indexed normally
