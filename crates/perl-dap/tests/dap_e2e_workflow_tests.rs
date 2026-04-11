//! End-to-end DAP workflow integration tests.
//!
//! These tests drive a real `perl -d` process through a complete user-visible
//! debugging workflow: launch → set breakpoint → hit → inspect variables →
//! step/continue → next breakpoint → disconnect.
//!
//! All tests skip gracefully when `perl` is not on `PATH`, matching the pattern
//! from `dap_smoke_e2e.rs`.
//!
//! AC:3486 — End-to-end workflow: launch -> breakpoint -> inspect -> step -> continue -> exit

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use std::fs::write;
use tempfile::tempdir;

// ─── Fixture line constants ────────────────────────────────────────────────────
//
// All three test scripts share the same structure:
//
//   Line 1: use strict;
//   Line 2: use warnings;
//   Line 3: (blank)
//   Line 4: my $x = 10;        <- BP_LINE_1
//   Line 5: my $y = $x + 5;    <- BP_LINE_2
//   Line 6: my $z = $x * $y;
//   Line 7: print "$z\n";
//
const BP_LINE_1: u64 = 4; // my $x = 10
const BP_LINE_2: u64 = 5; // my $y = $x + 5

/// Minimal three-line body script.  Lines 1-3 are headers; executable code
/// starts at line 4, matching BP_LINE_1/BP_LINE_2 above.
fn workflow_script_content() -> &'static str {
    "use strict;\nuse warnings;\n\nmy $x = 10;\nmy $y = $x + 5;\nmy $z = $x * $y;\nprint \"$z\\n\";\n"
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ─── Test 1: single breakpoint → inspect → continue → exit ───────────────────

/// Validates the core debugging workflow:
/// launch with stopOnEntry=false → set one breakpoint → configurationDone →
/// wait for stopped(reason=breakpoint) → stackTrace → scopes → variables(non-empty)
/// → continue → terminated.
#[test]
fn test_e2e_single_breakpoint_hit_inspect_continue() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_single_breakpoint_hit_inspect_continue - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_e2e.pl");
    write(&script, workflow_script_content())?;

    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;

    // DAP ordering: setBreakpoints BEFORE configurationDone.
    let bp_body = session.set_breakpoints(&script_str, &[BP_LINE_1])?;
    let bp_body = bp_body.ok_or("setBreakpoints returned no body")?;
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(|v| v.as_array())
        .ok_or("setBreakpoints body missing `breakpoints` array")?;
    assert!(
        !breakpoints.is_empty(),
        "setBreakpoints response must contain at least one breakpoint entry"
    );

    session.configuration_done()?;

    // Wait for the debugger to stop at our breakpoint.
    let stopped = session.wait_stopped()?;
    assert_eq!(
        stopped.reason, "breakpoint",
        "stopped reason must be `breakpoint`, got `{}`",
        stopped.reason
    );

    let thread_id = stopped.thread_id;

    // Retrieve stack trace → top frame id and source path.
    let (frame_id, source_path) = session.stack_trace(thread_id)?;
    assert!(
        source_path.contains("workflow_e2e"),
        "stack frame source path `{source_path}` should refer to the workflow fixture"
    );

    // Retrieve locals scope reference, then variables.
    let locals_ref = session.scopes_locals_ref(frame_id)?;
    let variables = session.variables(locals_ref)?;
    assert!(
        !variables.is_empty(),
        "locals scope must contain at least one variable at breakpoint \
         (frame_id={frame_id}, locals_ref={locals_ref})"
    );

    // All variable entries must have a non-empty name.
    for var in &variables {
        let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!name.is_empty(), "variable entry must have a non-empty `name` field: {var:?}");
    }

    // Continue to script exit.
    session.continue_exec(thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

// ─── Test 2: multi-breakpoint sequence ────────────────────────────────────────

/// Validates that multiple breakpoints are hit in source order:
/// set two breakpoints → hit first (BP_LINE_1) → continue → hit second (BP_LINE_2) → exit.
#[test]
fn test_e2e_multi_breakpoint_sequence() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_multi_breakpoint_sequence - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_multi.pl");
    write(&script, workflow_script_content())?;

    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints(&script_str, &[BP_LINE_1, BP_LINE_2])?;
    session.configuration_done()?;

    // First stop — must be at BP_LINE_1.
    let first_stop = session.wait_stopped()?;
    assert_eq!(
        first_stop.reason, "breakpoint",
        "first stop reason must be `breakpoint`, got `{}`",
        first_stop.reason
    );

    // If the stopped event carries a line number, validate it.
    if let Some(line) = first_stop.line {
        assert_eq!(line, BP_LINE_1, "first breakpoint should be at line {BP_LINE_1}, got {line}");
    }

    // Continue to second breakpoint.
    session.continue_exec(first_stop.thread_id)?;
    let second_stop = session.wait_stopped()?;
    assert_eq!(
        second_stop.reason, "breakpoint",
        "second stop reason must be `breakpoint`, got `{}`",
        second_stop.reason
    );

    if let Some(line) = second_stop.line {
        assert_eq!(line, BP_LINE_2, "second breakpoint should be at line {BP_LINE_2}, got {line}");
    }

    // Continue to script exit.
    session.continue_exec(second_stop.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

// ─── Test 3: step-over changes line ───────────────────────────────────────────

/// Validates that `next` (step-over) advances execution:
/// stop at breakpoint (BP_LINE_2) → stepOver → stopped(reason=step).
///
/// # Why BP_LINE_2 and not BP_LINE_1?
///
/// `perl -d` always stops at the first executable line (line 4) before
/// processing any stdin commands.  With `stopOnEntry: false`,
/// `configurationDone` sends `c` to run to the first user breakpoint.
/// When the breakpoint is set on line 4 (the initial stop line), `c` runs
/// *past* it and continues to program termination — the Perl debugger does
/// not re-trigger a breakpoint on the line where execution is already
/// paused.  Setting the breakpoint on line 5 (BP_LINE_2) ensures `c`
/// properly runs from line 4 **to** the breakpoint at line 5, leaving the
/// stdin pipe empty so the subsequent `n` command is the first command the
/// debugger receives after the stop.
#[test]
fn test_e2e_step_over_changes_execution() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_step_over_changes_execution - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_step.pl");
    write(&script, workflow_script_content())?;

    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    // Use BP_LINE_2 (line 5) so that configurationDone's `c` runs FROM the
    // initial implicit stop at line 4 TO the breakpoint at line 5, not past it.
    session.set_breakpoints(&script_str, &[BP_LINE_2])?;
    session.configuration_done()?;

    let at_breakpoint = session.wait_stopped()?;
    assert_eq!(
        at_breakpoint.reason, "breakpoint",
        "initial stop reason must be `breakpoint`, got `{}`",
        at_breakpoint.reason
    );

    let thread_id = at_breakpoint.thread_id;

    // Step over to the next line (line 6).
    session.step_over(thread_id)?;
    let after_step = session.wait_stopped()?;

    // After stepOver, reason must be "step" (not "breakpoint").
    assert_eq!(
        after_step.reason, "step",
        "stop reason after stepOver must be `step`, got `{}`",
        after_step.reason
    );

    session.disconnect()?;

    Ok(())
}
