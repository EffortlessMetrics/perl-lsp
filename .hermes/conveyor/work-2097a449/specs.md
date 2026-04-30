# Specs: First-Run Error Classification and Messaging

## Feature Description

Improve user-facing error messaging for two first-run failure scenarios:

1. **Gap 1 (Perl Not Installed/Workspace Root Undetected)**: When perl-lsp cannot detect a workspace root, it currently logs a warning to the server log only. Users opening a single file without a workspace see nothing. The fix surfaces a `window/showMessage` notification to the user with actionable guidance.

2. **Gap 5 (DAP Perl Not on PATH)**: When the debugger cannot find the `perl` interpreter, it currently returns a raw I/O error `"No such file or directory: 'perl'"`. The fix detects this specific failure and returns an actionable error message: `"Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH."`

## Non-Goals

- This spec does NOT address the missing module diagnostic severity question (Warning vs Error) — deferred to team decision
- This spec does NOT implement a centralized error message service
- This spec does NOT add editor-specific messaging beyond what is already in the codebase
- This spec does NOT address DAP PermissionDenied errors (when perl is found but not executable)
- This spec does NOT re-file already-addressed error scenarios (parse error hints, missing module @INC context, workspace building notification, permission denied, LSP crash classification)

## Gap 1: Workspace Root Undetected Messaging

### Changes

1. **Add `root_undetected_shown` field to `LspServer`**:
   - Type: `Arc<AtomicBool>`
   - Location: In the `LspServer` struct definition alongside other `Arc<AtomicBool>` fields
   - Initialization: `root_undetected_shown: Arc::new(AtomicBool::new(false))`

2. **Update `resolve_module_path` call site** (3 locations in `module_resolution.rs`):
   - Lines 181-186, 218-224, 349-355
   - Replace `WARN_ONCE_ROOT_UNDETECTED.call_once(|| { tracing::warn!(...) })` with:
     ```rust
     if self.root_undetected_shown.fetch_or(true, Ordering::SeqCst) == false {
         let _ = self.show_message(MessageType::Warning, 
             "perl-lsp: workspace root not detected — module resolution disabled. \
              To enable: open the project folder in your editor (File > Open Folder) \
              rather than individual files. This warning appears once per server session.");
     }
     ```

3. **Remove `WARN_ONCE_ROOT_UNDETECTED` static** after migration is complete

### Message Text
> "perl-lsp: workspace root not detected — module resolution disabled. To enable: open the project folder in your editor (File > Open Folder) rather than individual files. This warning appears once per server session."

### Behavior
- Message appears once per `LspServer` instance (not once per process)
- Subsequent module resolution calls within the same session do not re-display the message
- Message is non-blocking (uses `let _ =` to ignore result)
- If `show_message` fails (channel not initialized), the error is silently dropped

## Gap 5: DAP Perl Not Found Error Message

### Changes

1. **In `perl-dap/src/debug_adapter/process.rs:365`**:
   - Before:
     ```rust
     Err(e) => Err(e.to_string())
     ```
   - After:
     ```rust
     Err(e) => {
         if e.kind() == std::io::ErrorKind::NotFound {
             Err("Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH.".to_string())
         } else {
             Err(e.to_string())
         }
     }
     ```

### Message Text
> "Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH."

### Behavior
- Only triggers when `perl` is not found (`ErrorKind::NotFound`)
- Other spawn errors return the original I/O error string (unchanged behavior)
- Error is returned to the DAP client as a structured error message

## Acceptance Criteria

### Gap 1: Workspace Root Undetected

1. **AC1**: When a user opens a single Perl file (no workspace root) and triggers module resolution, a `window/showMessage` notification with `MessageType::Warning` is sent exactly once for that server session.

2. **AC2**: The warning message text includes: (a) the problem description ("workspace root not detected"), (b) remediation guidance ("open the project folder in your editor"), and (c) a note that it appears once per session.

3. **AC3**: Multiple rapid module resolution calls before the first call completes do not result in multiple user-visible notifications.

4. **AC4**: The `LspServer` struct has a `root_undetected_shown: Arc<AtomicBool>` field.

5. **AC5**: Existing tests pass; a new test verifies that `show_message` is called once when root is undetected.

### Gap 5: DAP Perl Not Found

6. **AC6**: When `perl` is not on PATH and the user launches the debugger, the error message displayed is: "Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH." — NOT "No such file or directory: 'perl'".

7. **AC7**: Other spawn errors (e.g., permission denied when perl exists but is not executable) continue to return their original I/O error string.

8. **AC8**: A test exists for the `ErrorKind::NotFound` detection path in `process.rs`.

## Dependencies

- **Rust crates**: `perl-lsp`, `perl-dap`
- **LSP infrastructure**: `LspServer::show_message()` method (already exists)
- **Testing infrastructure**: Existing Rust test patterns in `perl-dap`

## Testing Strategy

### Gap 1
- Unit test: Verify `show_message` is called exactly once when `root_undetected_shown` is initially `false` and module resolution is triggered
- Unit test: Verify `show_message` is not called again when `root_undetected_shown` is already `true`

### Gap 5
- Unit test: Verify `ErrorKind::NotFound` returns the actionable error message
- Unit test: Verify other error kinds return the original error string

## Files to Modify

1. `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs` — Replace `WARN_ONCE_ROOT_UNDETECTED` pattern with `Arc<AtomicBool>` check
2. `crates/perl-lsp/src/runtime/lsp_server.rs` — Add `root_undetected_shown: Arc<AtomicBool>` field to `LspServer` struct
3. `crates/perl-dap/src/debug_adapter/process.rs:365` — Add `ErrorKind::NotFound` detection
4. Test files: Add tests for both behaviors

## Open Questions (Deferred)

1. Should the missing module diagnostic be `Warning` or `Error` severity? (Deferred to team)
2. Is the VS Code-specific "File > Open Folder" message appropriate for all editor contexts? (Deferred)
3. Should DAP also handle `PermissionDenied` when perl is found but not executable? (Future work)
