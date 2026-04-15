# Task List — work-2097a449

## Gap 1: Workspace Root Undetected Messaging

- [ ] 1.1 Add `root_undetected_shown: Arc<AtomicBool>` field to `LspServer` struct in `perl-lsp/src/runtime/lsp_server.rs`
- [ ] 1.2 Initialize field in `LspServer::new()` as `root_undetected_shown: Arc::new(AtomicBool::new(false))`
- [ ] 1.3 Replace first `WARN_ONCE_ROOT_UNDETECTED.call_once()` at `module_resolution.rs:181-186` with atomic check-and-set pattern
- [ ] 1.4 Replace second call site at `module_resolution.rs:218-224` with same pattern
- [ ] 1.5 Replace third call site at `module_resolution.rs:349-355` with same pattern
- [ ] 1.6 Remove `WARN_ONCE_ROOT_UNDETECTED` static declaration from module level
- [ ] 1.7 Add unit test verifying `show_message` is called exactly once when root is undetected
- [ ] 1.8 Add unit test verifying `show_message` is not called again when flag is already set
- [ ] 1.9 Verify existing tests pass

## Gap 5: DAP Perl Not Found Error Message

- [ ] 2.1 In `perl-dap/src/debug_adapter/process.rs:365`, replace `Err(e) => Err(e.to_string())` with `ErrorKind::NotFound` detection
- [ ] 2.2 Return actionable message: "Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH."
- [ ] 2.3 Add unit test for `ErrorKind::NotFound` path returning actionable message
- [ ] 2.4 Add unit test verifying other error kinds return original error string
- [ ] 2.5 Verify existing tests pass

## Integration

- [ ] 3.1 Create GitHub follow-up issues for Gap 1 and Gap 5, linking to umbrella issue #4178
- [ ] 3.2 Close or update umbrella issue #4178 to reflect remaining work
- [ ] 3.3 Update CHANGELOG or docs if error message text is user-facing

## Deferred Decisions (No Implementation)

- [ ] 4.1 Missing module diagnostic severity (Warning vs Error) — deferred to team decision
- [ ] 4.2 Editor-agnostic messaging — deferred to future discussion
- [ ] 4.3 DAP PermissionDenied handling — deferred to future work
