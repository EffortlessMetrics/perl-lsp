# Initial Plan — work-2097a449

## Issue
[UX] first-run error classification and messaging — https://github.com/EffortlessMetrics/perl-lsp/issues/4178

## Approach

This umbrella issue describes 7 error scenarios. My research found that **4 of the 7 are already addressed** in the current codebase (likely fixed since the issue was filed), because I examined the actual code at each cited location and found that the issue descriptions did not match the current state. The genuinely remaining gaps are:

1. **Gap 1 (HIGH): Perl not installed — use window/showMessage instead of tracing::warn!()**
   - Site: `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs:181-226`
   - Currently: `WARN_ONCE_ROOT_UNDETECTED.call_once(|| { tracing::warn!(...) })` — goes to server log only
   - Fix: Replace `tracing::warn!()` with `self.show_message(MessageType::Warning, ...)` using the same `LspServer` receiver pattern
   - Why: `tracing::warn!()` writes to the server's debug log which users never see unless they explicitly enable verbose logging. Using `window/showMessage` ensures the user sees the message directly in VS Code. The `WARN_ONCE_ROOT_UNDETECTED` static is kept to avoid duplicate notifications because we want to show the message exactly once per session, not on every module resolution attempt.

2. **Gap 2 (MEDIUM): DAP — explicit "perl not on PATH" remediation in check_syntax skip path**
   - Site: `crates/perl-dap/src/debug_adapter/process.rs:393-398`
   - Currently: When `perl -c` can't be spawned, silently returns `Ok(())` and lets the subsequent `perl -d` launch produce whatever error comes out
   - Fix: When `perl` cannot be found (not on PATH), detect this specifically and return a targeted error like `"Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH."`
   - Why: The current approach technically works — `perl -d` will fail and produce an error — but that error message is less actionable because it surfaces as an I/O error rather than a clear "perl not found" message. Detecting `io::ErrorKind::NotFound` explicitly allows us to provide a targeted remediation message upfront. This matters because the DAP launch flow is a key onboarding scenario for new users.

3. **Gap 3 (LOW): Missing module diagnostic severity**
   - Site: `crates/perl-lsp-diagnostics/src/lints/missing_module.rs:245-256`
   - Currently: `DiagnosticSeverity::Warning`
   - Consider: Whether a missing required module should be `Error` severity instead of `Warning`
   - Why: A Warning severity lets the file open and show other diagnostics, but a required module that's truly missing might be a critical import that will cause runtime failures. This is a design decision — the current Warning is conservative (non-blocking), but Error would be more consistent with how other languages handle missing imports. We defer this decision to the team.

## What NOT To Do

The issue recommends filing 7 follow-up issues, but my research shows only **2-3 genuine gaps** remain in the current codebase. The following were found to already be addressed:
- Parse errors with hints (already have `build_parse_error_hint`)
- Missing module @INC context (already shows searched paths)
- Workspace building notification (already sends `logMessage`)
- Permission denied (already sends `showMessage`; wrong file cited in issue)
- LSP crash classification (already wired in `startupDiagnosis.ts`)

## Task Breakdown

### Phase 1: Fix Perl Not Installed (Highest Priority)
1. In `module_resolution.rs`, refactor `WARN_ONCE_ROOT_UNDETECTED` to also call `self.show_message(...)` instead of only `tracing::warn!(...)`
   - The `LspServer` has access to `show_message()` — need to verify it's callable from this context (the module resolution functions are methods on `LspServer`)
   - If not directly callable, may need to pass a messenger/shell handle
2. Add a test that verifies the window/showMessage is sent (not just traced)
3. Verify with manual testing that the message appears in VSCode

### Phase 2: Fix DAP Perl Not On PATH
1. In `process.rs`, modify the `check_syntax` error path at line 393-398
2. Detect specifically when `e.kind() == ErrorKind::NotFound` (io::Error)
3. Return explicit "Perl interpreter not found on PATH" message with remediation hint
4. Add test for this specific error path

### Phase 3: Validate/Review Missing Module Severity
1. Check with team/product on whether `Warning` vs `Error` severity is the right default
2. If Error is desired, change `DiagnosticSeverity::Warning` to `DiagnosticSeverity::Error` in `missing_module.rs`

## Risks

1. **Risk: show_message requires LspServer context** — The `resolve_module_path` and `resolve_module_path_with_uri` functions ARE methods on `LspServer` (line: `impl LspServer`), so `self.show_message(...)` should be callable. However, need to verify the channel is initialized at the point these warnings fire (they fire during early module resolution).

2. **Risk: Testability** — Adding tests for window/showMessage may require mocking the LspServer's message channel. Need to verify existing test infrastructure can handle this.

3. **Risk: Stale issue description** — The issue was filed based on a scan of the codebase at a point in time. Some gaps may have already been fixed by other work. The plan above reflects the current (not historical) state.

4. **Risk: Umbrella issue scope creep** — If we file only 3 follow-up issues instead of 7, we may need to update the umbrella issue to reflect what's actually left.

## Scope

- **In scope**: Fix the 2-3 genuine gaps identified in research; file updated follow-up issues; link them to the umbrella
- **Out of scope**: Centralized error message service (Option A from issue — rejected as too high-cost); re-filing already-addressed gaps as new issues
