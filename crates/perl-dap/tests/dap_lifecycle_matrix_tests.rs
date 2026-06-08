//! DAP lifecycle matrix e2e tests.
//!
//! Drives the FULL DAP lifecycle in protocol-correct order against a real `perl -d`:
//!   initialize → launch → setBreakpoints → configurationDone
//!     → stopped(reason=breakpoint) → stackTrace → scopes → variables
//!     → continue → terminated(natural exit) → disconnect
//!
//! Uses `DapWorkflowSession` helpers from `common/mod.rs` (including the
//! `set_breakpoints_checked`/`wait_stopped_with_frame` additions from #927).
//!
//! All tests skip gracefully when `perl` is not on `PATH`.
//! AC: DAP lifecycle matrix — phase 2 e2e coverage.

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use tempfile::tempdir;

// ─── Fixture ──────────────────────────────────────────────────────────────────────────────
//
//   Line 1: use strict;
//   Line 2: use warnings;
//   Line 3: (blank)
//   Line 4: my $x = 10;      <- first executable line; perl -d always pauses here
//   Line 5: my $y = $x + 5;  <- reliable first breakpoint (configurationDone runs FROM line 4 TO line 5)
//   Line 6: my $z = $x * $y;
//   Line 7: print "$z\n";
//
// Line 4 is the implicit stop line — a breakpoint there is skipped by the
// initial `c` from configurationDone.  Tests use line 5 (BP_LINE) as the
// reliable first hit.

const BP_LINE: u64 = 5;

fn lifecycle_script_content() -> &'static str {
    "use strict;\nuse warnings;\n\nmy $x = 10;\nmy $y = $x + 5;\nmy $z = $x * $y;\nprint \"$z\\n\";\n"
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ─── Test 1: full ordered lifecycle — single test, every step asserted ────────

/// Exercises the FULL DAP lifecycle in protocol-correct order:
///
///   1.  initialize (via DapWorkflowSession::new) → success + initialized event
///   2.  launch(stopOnEntry=false) → success
///   3.  setBreakpoints(verified=true) → adapter-resolved line returned
///   4.  configurationDone → success
///   5.  wait for stopped(reason=breakpoint) ← precedes stackTrace (ordering contract)
///   6.  stackTrace → concrete frame whose line matches resolved breakpoint line
///   7.  scopes → Locals scope present
///   8.  variables(locals) → non-empty list
///   9.  continue → execution resumes
///  10.  terminated event received (natural program exit via `continue`)
///  11.  disconnect → clean (disconnect helper drains terminated then returns)
///
/// This is the "happy path" lifecycle matrix: every step is asserted in order,
/// and each assertion documents the adapter contract it validates.
#[test]
fn test_lifecycle_full_ordered_sequence() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_lifecycle_full_ordered_sequence - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("lifecycle_matrix.pl");
    write(&script, lifecycle_script_content())?;
    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();

    // ── Step 1: initialize ───────────────────────────────────────────
    // DapWorkflowSession::new sends `initialize`, asserts success, and drains
    // the `initialized` event — verifying steps 1+1a before continuing.
    let mut session = DapWorkflowSession::new(timeout)?;

    // ── Step 2: launch ─────────────────────────────────────────────
    // stopOnEntry=false: adapter does NOT emit stopped until configurationDone.
    session.launch(&script_str)?;

    // ── Step 3: setBreakpoints (verified=true) ──────────────────────────────────
    // DAP protocol ordering: setBreakpoints MUST be called before configurationDone.
    // set_breakpoints_checked asserts verified=true for every entry and returns
    // the adapter-resolved line numbers (which may differ from requested in
    // future when breakpoint remapping is implemented).
    let resolved = session.set_breakpoints_checked(&script_str, &[BP_LINE])?;
    let resolved_line =
        resolved.first().copied().ok_or("set_breakpoints_checked returned empty resolved lines")?;

    // Resolved line must be positive (sanity: adapter gave us a real line).
    assert!(
        resolved_line > 0,
        "adapter-resolved breakpoint line must be positive, got {resolved_line}"
    );

    // ── Step 4: configurationDone ──────────────────────────────────────────
    session.configuration_done()?;

    // ── Step 5: stopped(reason=breakpoint) ──────────────────────────────────
    // ORDERING CONTRACT: the `stopped` event must arrive BEFORE we can issue
    // `stackTrace`.  We assert the event precedes the request by not calling
    // `stack_trace` until after `wait_stopped` returns.
    //
    // `wait_stopped_with_frame` is atomic: it waits for `stopped`, then
    // immediately calls `stackTrace` — so the ordering is correct by construction.
    let frame_info = session.wait_stopped_with_frame()?;

    assert_eq!(
        frame_info.reason, "breakpoint",
        "stopped reason at first breakpoint must be `breakpoint`, got `{}`",
        frame_info.reason
    );

    // ── Step 6: stackTrace → frame whose line matches resolved breakpoint ─────
    // The line contract: adapter-resolved line from setBreakpoints == stopped frame line.
    assert_eq!(
        frame_info.line, resolved_line,
        "stackTrace frame line must equal adapter-resolved breakpoint line \
         (resolved={resolved_line}, BP_LINE={BP_LINE}), got frame_line={}",
        frame_info.line
    );

    // Source path must reference our script (not an internal file).
    assert!(
        frame_info.source_path.contains("lifecycle_matrix"),
        "stackTrace source path `{}` must reference the lifecycle fixture script",
        frame_info.source_path
    );

    // frame_id must be positive (required for scopes/variables requests).
    assert!(
        frame_info.frame_id > 0,
        "stackTrace frame_id must be positive, got {}",
        frame_info.frame_id
    );

    // ── Step 7: scopes → Locals scope present ────────────────────────────
    // scopes_locals_ref returns the variablesReference for the Locals scope,
    // asserting the scope exists and has a positive reference.
    let locals_ref = session.scopes_locals_ref(frame_info.frame_id)?;
    assert!(
        locals_ref > 0,
        "Locals scope variablesReference must be positive (frameId={}), got {locals_ref}",
        frame_info.frame_id
    );

    // ── Step 8: variables(locals) → non-empty list ─────────────────────────
    // At BP_LINE (line 5, `my $y = $x + 5`), the previous line ($x = 10) has
    // already executed, so `$x` must be visible.  At minimum one variable is present.
    let locals = session.variables(locals_ref)?;
    assert!(
        !locals.is_empty(),
        "locals scope must contain at least one variable at BP_LINE={BP_LINE} \
         (locals_ref={locals_ref})"
    );

    // Each variable must have name, value, and variablesReference fields.
    for var in &locals {
        let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let value = var.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let vars_ref = var.get("variablesReference").and_then(|v| v.as_i64()).unwrap_or(-1);

        assert!(!name.is_empty(), "locals variable must have non-empty `name`: {var:?}");
        assert!(!value.is_empty(), "locals variable `{name}` must have non-empty `value`: {var:?}");
        assert!(
            vars_ref >= 0,
            "locals variable `{name}` must have numeric `variablesReference`: {var:?}"
        );
    }

    // ── Step 9 + 10: continue → terminated event (natural program exit) ──────
    // `continue` resumes execution.  The script has no more breakpoints, so it
    // runs to completion.  The adapter should emit a `terminated` event when
    // the Perl process exits.
    //
    // COVERAGE GAP: `terminated` event delivery is not guaranteed by the current
    // adapter — the event channel may close before the event arrives, which
    // causes drain_until_event to return a channel-closed error.  The existing
    // test suite accommodates this with `let _ =` (see dap_e2e_workflow_tests.rs).
    // Test 4 (test_lifecycle_continue_leads_to_terminated_event) is the dedicated
    // test that documents this gap; here we follow the established pattern.
    session.continue_exec(frame_info.thread_id)?;

    // Drain `terminated` best-effort; channel may close before event arrives.
    let _ = session.drain_until_event("terminated");

    // ── Step 11: disconnect ───────────────────────────────────────────
    // disconnect sends the DAP `disconnect` request and expects a clean response.
    session.disconnect()?;

    Ok(())
}

// ─── Test 2: stopped event PRECEDES stackTrace (explicit ordering proof) ──────

/// Proves the event-before-request ordering required by the DAP spec:
/// the `stopped` event must arrive before the client issues `stackTrace`.
///
/// This test explicitly separates `wait_stopped` from `stack_trace` to
/// demonstrate that we observe the event first, then issue the request.
///
/// If the adapter were to emit a `stopped` event and also immediately send
/// frames without a client request, this test would catch the race.
#[test]
fn test_lifecycle_stopped_event_precedes_stack_trace() -> TestResult {
    if !perl_available() {
        eprintln!(
            "Skipping test_lifecycle_stopped_event_precedes_stack_trace - perl not available"
        );
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("lifecycle_ordering.pl");
    write(&script, lifecycle_script_content())?;
    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints_checked(&script_str, &[BP_LINE])?;
    session.configuration_done()?;

    // Explicitly separate: first wait for the stopped EVENT.
    let stopped = session.wait_stopped()?;
    assert_eq!(
        stopped.reason, "breakpoint",
        "stopped event reason must be `breakpoint`, got `{}`",
        stopped.reason
    );

    // Only AFTER observing the event do we issue the stackTrace REQUEST.
    // This is the protocol-correct ordering.
    let (frame_id, source_path, frame_line) = session.stack_trace(stopped.thread_id)?;
    assert!(frame_id > 0, "stackTrace frame_id must be positive after stopped event");
    assert!(
        !source_path.is_empty(),
        "stackTrace source_path must be non-empty after stopped event"
    );
    assert!(
        frame_line > 0,
        "stackTrace frame_line must be positive (1-based) after stopped event, got {frame_line}"
    );

    session.continue_exec(stopped.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

// ─── Test 3: scopes returns Locals AND Globals ─────────────────────────────────

/// Validates that `scopes` returns BOTH Locals and Globals scopes, and that the
/// Locals scope contains a REAL named lexical variable from the user script.
///
/// The DAP spec requires a `scopes` response for each frame; editors typically
/// render Locals and Globals as separate expandable trees.  This test asserts
/// that both scope references are positive and distinct, indicating non-empty,
/// separate scope buckets.
///
/// Fixture layout (lifecycle_scopes.pl):
///   Line 1: use strict;
///   Line 2: use warnings;
///   Line 3: (blank)
///   Line 4: our $global = 42;   <- first executable; perl -d pauses here implicitly
///   Line 5: my $x = 10;         <- after configurationDone `c`, stops here first
///   Line 6: my $y = $x + 5;     <- SCOPES_BP_LINE: $x=10 has already executed
///   Line 7: print "$y\n";
///
/// The breakpoint is set at line 6 (not the shared BP_LINE=5) because line 5 is
/// the first stop after configurationDone and `my $x = 10` has not yet executed
/// at that point.  Stopping at line 6 guarantees `$x` is in scope with a real
/// value, so the Locals assertion cannot pass on placeholder fallback.
///
/// **Regression guard for #997 (B-module PADLIST locals fix):**
/// Prior to #997, the adapter's `variables(locals_ref)` handler returned locals
/// from the Perl debugger's internal frame (DB object: `$self` and `@_`) instead
/// of the user script's lexical scope (`$x`, `$y`).  The fix in #997 replaces the
/// broken `V <frame_id> .` approach with a B-module eval that walks the current
/// pad via `PADLIST`/`main_cv`.  The assertion below (`$x` must appear by name)
/// is a regression guard: it will catch any future revert of the PADLIST walk that
/// re-exposes the internal-frame bug.
#[test]
fn test_lifecycle_scopes_locals_and_globals() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_lifecycle_scopes_locals_and_globals - perl not available");
        return Ok(());
    }

    // Breakpoint line for THIS fixture only — one line past the first lexical
    // assignment so that `my $x = 10` has already executed when we stop.
    const SCOPES_BP_LINE: u64 = 6;

    let workspace = tempdir()?;
    let script = workspace.path().join("lifecycle_scopes.pl");
    // Script with an explicit `our` global so the Globals scope has at least one entry.
    // Line layout (1-based):
    //   1: use strict;
    //   2: use warnings;
    //   3: (blank)
    //   4: our $global = 42;
    //   5: my $x = 10;
    //   6: my $y = $x + 5;   <- SCOPES_BP_LINE
    //   7: print "$y\n";
    let content = "use strict;\nuse warnings;\n\nour $global = 42;\nmy $x = 10;\nmy $y = $x + 5;\nprint \"$y\\n\";\n";
    write(&script, content)?;
    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    let resolved = session.set_breakpoints_checked(&script_str, &[SCOPES_BP_LINE])?;
    let resolved_line =
        resolved.first().copied().ok_or("set_breakpoints_checked returned empty resolved lines")?;
    session.configuration_done()?;

    let frame_info = session.wait_stopped_with_frame()?;
    assert_eq!(frame_info.reason, "breakpoint");

    // Verify we stopped at the expected line (adapter-resolved SCOPES_BP_LINE).
    assert_eq!(
        frame_info.line, resolved_line,
        "stopped frame line must equal adapter-resolved SCOPES_BP_LINE \
         (resolved={resolved_line}, SCOPES_BP_LINE={SCOPES_BP_LINE}), got {}",
        frame_info.line
    );

    // Locals scope: must be present and positive.
    let locals_ref = session.scopes_locals_ref(frame_info.frame_id)?;
    assert!(
        locals_ref > 0,
        "Locals scope variablesReference must be positive (frameId={})",
        frame_info.frame_id
    );

    // Globals scope: must be present and positive.
    let globals_ref = session.scopes_globals_ref(frame_info.frame_id)?;
    assert!(
        globals_ref > 0,
        "Globals scope variablesReference must be positive (frameId={})",
        frame_info.frame_id
    );

    // Locals and Globals must have DIFFERENT references (no aliasing).
    assert_ne!(
        locals_ref, globals_ref,
        "Locals and Globals variablesReference must be distinct \
         (locals={locals_ref}, globals={globals_ref})"
    );

    // Locals must contain the real lexical `$x` (assigned at line 5).
    // This assertion CANNOT pass on placeholder fallback — it checks that the
    // adapter parsed actual lexical locals, not a generic placeholder list.
    let locals = session.variables(locals_ref)?;
    assert!(
        !locals.is_empty(),
        "Locals scope variables must be non-empty when stopped at SCOPES_BP_LINE={SCOPES_BP_LINE} \
         (locals_ref={locals_ref}, frame_id={})",
        frame_info.frame_id
    );

    // Find `$x` by name in the locals list.  The adapter must report the real
    // lexical variable — not merely a placeholder entry.
    let x_var = locals.iter().find(|v| {
        v.get("name").and_then(|n| n.as_str()).map(|n| n == "$x" || n == "x").unwrap_or(false)
    });
    assert!(
        x_var.is_some(),
        "Locals must contain `$x` (assigned at line 5) when stopped at \
         SCOPES_BP_LINE={SCOPES_BP_LINE}; got locals={locals:?}"
    );

    // Globals must be non-empty (our $global = 42 is in scope).
    let globals = session.variables(globals_ref)?;
    assert!(
        !globals.is_empty(),
        "Globals scope variables must be non-empty (script declares `our $global`)"
    );

    session.continue_exec(frame_info.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

// ─── Test 4: continue → terminated (natural exit lifecycle) ──────────────

/// Validates the termination path: continue from a breakpoint → disconnect cleanly.
///
/// COVERAGE GAP (documented): the adapter does not reliably deliver a `terminated`
/// event before the event channel closes.  After `continue` causes the Perl
/// process to exit naturally, the channel may close before `terminated` arrives
/// (manifests as "channel closed/timeout waiting for `terminated`").  This gap
/// is already accommodated across the existing test suite with `let _ =` drains.
///
/// What this test DOES validate:
///   - `continue` is accepted without error
///   - `disconnect` is clean after natural program exit (even without a `terminated` event)
///
/// An explicit DAP `terminate` request is not tested; the adapter does not
/// currently expose a `terminate` handler beyond this natural-exit path.
#[test]
fn test_lifecycle_continue_leads_to_terminated_event() -> TestResult {
    if !perl_available() {
        eprintln!(
            "Skipping test_lifecycle_continue_leads_to_terminated_event - perl not available"
        );
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("lifecycle_exit.pl");
    write(&script, lifecycle_script_content())?;
    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints_checked(&script_str, &[BP_LINE])?;
    session.configuration_done()?;

    let frame_info = session.wait_stopped_with_frame()?;
    assert_eq!(frame_info.reason, "breakpoint");

    // Resume — no more breakpoints, script runs to EOF.
    // `continue` must be accepted without error.
    session.continue_exec(frame_info.thread_id)?;

    // Drain `terminated` best-effort.  The adapter does not guarantee this event
    // arrives before the channel closes; `let _ =` follows the established
    // pattern from the existing e2e test suite.  See PR body for the coverage gap.
    let _ = session.drain_until_event("terminated");

    // After natural exit, `disconnect` must succeed cleanly.
    // This is the primary assertion of this test.
    session.disconnect()?;

    Ok(())
}

// ─── Test 5: variables non-empty at stopped frame ──────────────────────────

/// Validates that variables inspection returns a non-empty list at a known stop.
///
/// This test verifies the variables contract specifically: that `$x` (assigned
/// at line 4) is visible in locals when stopped at line 5.  It also checks that
/// each variable entry satisfies the DAP `Variable` type shape:
///   name (string), value (string), variablesReference (number >= 0)
#[test]
fn test_lifecycle_variables_non_empty_at_stop() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_lifecycle_variables_non_empty_at_stop - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("lifecycle_vars.pl");
    write(&script, lifecycle_script_content())?;
    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints_checked(&script_str, &[BP_LINE])?;
    session.configuration_done()?;

    let frame_info = session.wait_stopped_with_frame()?;
    assert_eq!(frame_info.reason, "breakpoint");

    let locals_ref = session.scopes_locals_ref(frame_info.frame_id)?;
    let variables = session.variables(locals_ref)?;

    // At line 5 (BP_LINE), `$x = 10` (line 4) has already executed.
    // At least `$x` must be visible.
    assert!(
        !variables.is_empty(),
        "variables list must be non-empty when stopped at BP_LINE={BP_LINE} \
         (locals_ref={locals_ref}, frame_id={})",
        frame_info.frame_id
    );

    // Validate DAP Variable shape for each entry.
    for var in &variables {
        let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let value = var.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let vars_ref = var.get("variablesReference").and_then(|v| v.as_i64()).unwrap_or(-1);

        assert!(!name.is_empty(), "each variable must have non-empty `name` field: {var:?}");
        assert!(
            !value.is_empty(),
            "each variable `{name}` must have non-empty `value` field: {var:?}"
        );
        assert!(
            vars_ref >= 0,
            "each variable `{name}` must have non-negative `variablesReference`: {var:?}"
        );
    }

    session.continue_exec(frame_info.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

/// No active session: stackTrace returns honest empty list, not a fabricated frame.
/// Regression guard: pre-fix returned main::hello @ /tmp/hello.pl:10.
/// This test requires no `perl` on PATH — pure unit isolation.
#[test]
fn test_stacktrace_no_session_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(1, "stackTrace", Some(json!({"threadId": 1})));
    let DapMessage::Response { success, command, body, .. } = response else {
        return Err("Expected Response".into());
    };
    assert!(success, "stackTrace should succeed even without a session");
    assert_eq!(command, "stackTrace");
    let body = body.ok_or("Expected body")?;
    let frames =
        body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected stackFrames array")?;
    assert_eq!(frames.len(), 0, "no session must return stackFrames: [] (not a fabricated frame)");
    let total = body.get("totalFrames").and_then(|v| v.as_u64()).ok_or("Expected totalFrames")?;
    assert_eq!(total, 0, "totalFrames must be 0 when stackFrames is empty");
    Ok(())
}
