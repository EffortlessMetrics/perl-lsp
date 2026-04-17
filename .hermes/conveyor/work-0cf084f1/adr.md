# ADR-0194: Surface Permission-Denied File Errors During Workspace Indexing

## Status
Accepted (implemented, test fix pending)

## Context

Issue #4194 reported that when workspace indexing encounters files that cannot be read due to
permission errors (e.g., a project in a read-only directory), the errors were logged but not
surfaced to the user. Users saw missing workspace symbols with no explanation.

The issue recommended Option A + B:
- **Option A**: One-time `window/showMessage` warning on first permission error (suppressed
  subsequently in same session)
- **Option B**: Per-file `textDocument/publishDiagnostics` diagnostic for each affected file

The implementation was merged in commit `a7344b84` to `crates/perl-lsp/src/runtime/workspace.rs`.
However, a compilation error in the accompanying test file prevents the test suite from
verifying the implementation.

## Decision

The following architectural approach was implemented in `workspace.rs:1613-1670`:

1. **One-time `window/showMessage` warning**: Gated by `compare_exchange(false, true)` on an
   `Arc<AtomicBool>` (`permission_denied_shown` field on `LspServer`). Only the first background
   thread to encounter a permission error sends the warning. All subsequent errors are silently
   suppressed.

2. **Per-file `textDocument/publishDiagnostics`**: Emitted for every file that fails to read due
   to permission error. Not gated by the `permission_denied_shown` flag — users must know which
   specific files are unreadable.

3. **Cross-platform detection**: `is_permission_denied_error()` in `workspace.rs:140-150` covers
   both `ErrorKind::PermissionDenied` (portable) and Windows `ERROR_ACCESS_DENIED` (os error 5).

4. **RAII guard**: `permission_denied_shown: Arc<AtomicBool>` on `LspServer` (mod.rs:293) is cloned
   into the background indexing thread (workspace.rs:1541). The guard is thread-safe and session-
   scoped (lives for the LSP server lifetime).

## Alternatives Considered

### Alternative 1: Per-file showMessage (rejected)
Emit `window/showMessage` for every unreadable file. Rejected because it would spam the user
when entire directories are unreadable. The one-time guard was specifically requested in the
issue.

### Alternative 2: LSP `window/showDocument` (rejected)
Open a dedicated UI panel showing all unreadable files. Rejected as disproportionate — the
existing Option A + B approach is sufficient and uses standard LSP notifications that all
editors handle.

### Alternative 3: Only diagnostics, no showMessage (rejected)
Emit only per-file diagnostics without the one-time warning. Rejected because the issue
explicitly requested both, and without the top-level warning users may miss the diagnostics.

## Consequences

### Benefits
- Users are notified on first permission error and know which specific files are affected
- One-time warning avoids spam when many files are unreadable
- Cross-platform support (Unix + Windows)
- No new dependencies — uses existing `tracing`, `serde_json`, `url`, and `Arc<AtomicBool>`

### Tradeoffs / Risks
- **Root user caveat**: On Unix, root bypasses permission checks so the feature cannot be tested
  in root environments. Tests skip gracefully via `can_create_permission_denied()`.
- **Test timing**: The tests use `std::thread::sleep(Duration::from_secs(3))` before draining
  notifications. On heavily loaded CI, the background indexing may not have completed in time,
  causing flaky test failures.
- **Issue description inaccuracy**: Issue #4194 references `text_sync.rs:220-228` as the source of
  permission errors during `didOpen`. This is incorrect — `text_sync.rs` receives content from the
  LSP client and never reads from the filesystem. The actual code path is `workspace.rs` during
  background workspace indexing. The implementation correctly fixed the real problem despite the
  misleading issue description.

## Implementation Details

### Files Changed
- `crates/perl-lsp/src/runtime/workspace.rs`: Permission-denied error handling in
  `start_workspace_indexing()` (lines 1613-1670)
- `crates/perl-lsp/src/runtime/mod.rs`: `permission_denied_shown` field on `LspServer` (line 293)
- `crates/perl-lsp/src/runtime/constructors.rs`: Initialization of `permission_denied_shown` (lines 63, 172, 244)
- `crates/perl-lsp/tests/workspace_permission_denied_test.rs`: Integration tests (compilation error at lines 92, 97)

### Remaining Work
Fix test compilation error at `workspace_permission_denied_test.rs:92,97`:
```rust
// WRONG:
let _ = std::fs::set_permissions(&probe, &perms);
// RIGHT:
let _ = std::fs::set_permissions(&probe, perms);
```
