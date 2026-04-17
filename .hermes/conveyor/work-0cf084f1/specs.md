# Spec: Permission-Denied File Error Surfacing

## Feature Description

When the LSP server's background workspace indexer encounters files that cannot be read due to
permission errors (e.g., a project inside a read-only directory), it must surface those errors
to the user rather than silently skipping the files. This is implemented via two complementary
LSP notifications:

1. **One-time `window/showMessage` warning**: The first permission-denied error triggers a
   warning message to the user. Subsequent permission-denied errors in the same session do NOT
   re-trigger the message. This prevents spam when many files are unreadable.

2. **Per-file `textDocument/publishDiagnostics`**: Every unreadable file emits its own
   diagnostic, so users can see exactly which files are affected. The diagnostic appears at
   line 0, character 0 with a descriptive error message.

## Acceptance Criteria

### AC-1: One-time showMessage
**Given** a workspace with one or more files that cannot be read due to permission errors
**When** the background indexing thread encounters the first unreadable file
**Then** the server sends exactly one `window/showMessage` notification with `type: 2` (Warning)
and a message containing "permission denied"
**And** subsequent unreadable files in the same session do NOT emit additional
`window/showMessage` notifications.

### AC-2: Per-file diagnostics
**Given** a workspace with one or more files that cannot be read due to permission errors
**When** the background indexing thread encounters each unreadable file
**Then** the server sends a `textDocument/publishDiagnostics` notification for that specific file
**And** the diagnostic has `severity: 1` (Error), `source: "perl-lsp"`, and a message containing
"permission denied" and the file path.

### AC-3: Cross-platform detection
**Given** a permission-denied error on Unix (ErrorKind::PermissionDenied)
**When** the error is encountered during indexing
**Then** it is detected and surfaced via AC-1 and AC-2.

**Given** a permission-denied error on Windows (ERROR_ACCESS_DENIED = os error 5)
**When** the error is encountered during indexing
**Then** it is detected and surfaced via AC-1 and AC-2.

### AC-4: Graceful degradation
**Given** a workspace with a mixture of readable and unreadable files
**When** indexing encounters both
**Then** readable files are indexed normally and available for symbol lookup
**And** the unreadable files emit diagnostics per AC-2
**And** the user receives the one-time warning per AC-1.

## Non-Goals

- This does NOT handle permission errors during `textDocument/didOpen` in `text_sync.rs` — that
  code path receives file content from the LSP client (editor) and never reads from the filesystem.
- This does NOT retry permission-denied files — if a file is unreadable, it is skipped and a
  diagnostic is emitted.
- This does NOT provide a way to "unlock" or request permission for unreadable files.
- This does NOT change the behavior for non-permission I/O errors (NotFound, IsADirectory, etc.)
  which are silently skipped with a debug-level trace message.

## Dependencies

- Feature flag: `#[cfg(feature = "workspace")]` — the implementation only compiles with the
  workspace feature enabled
- Platform: Cross-platform (Unix `ErrorKind::PermissionDenied` + Windows os error 5)
- Root user: On Unix as root, permission checks are bypassed by the OS, so the feature cannot be
  tested in root environments. Tests skip gracefully via `can_create_permission_denied()`.
- No new external dependencies — uses existing infrastructure (`tracing`, `serde_json`, `url`,
  `Arc<AtomicBool>`)

## Implementation Reference

- `crates/perl-lsp/src/runtime/workspace.rs:1613-1670`: Permission-denied error handling in
  `start_workspace_indexing()`
- `crates/perl-lsp/src/runtime/workspace.rs:140-150`: `is_permission_denied_error()` helper
- `crates/perl-lsp/src/runtime/mod.rs:293`: `permission_denied_shown` field
- `crates/perl-lsp/src/runtime/constructors.rs`: Field initialization
- `crates/perl-lsp/tests/workspace_permission_denied_test.rs`: Integration tests (pending fix)

## Test Fix Required

The test file `workspace_permission_denied_test.rs` has a compilation error at lines 92 and 97:
```rust
// WRONG (current):
let _ = std::fs::set_permissions(&probe, &perms);
// RIGHT (correct):
let _ = std::fs::set_permissions(&probe, perms);
```
The `set_permissions` function takes `&Permissions` not `&&Permissions`.
