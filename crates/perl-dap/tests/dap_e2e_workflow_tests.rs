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

use common::{assert_variables_have_names, DapWorkflowSession, perl_available, workflow_timeout};
use std::fs::write;
use tempfile::tempdir;

// ─── Fixture line constants ────────────────────────────────────────────────────
//
// All three test scripts share the same structure:
//
//   Line 1: use strict;
//   Line 2: use warnings;
//   Line 3: (blank)
//   Line 4: my $x = 10;        <- BP_LINE_1 (initial implicit stop — see note below)
//   Line 5: my $y = $x + 5;    <- BP_LINE_2
//   Line 6: my $z = $x * $y;   <- BP_LINE_3
//   Line 7: print "$z\n";
//
// IMPORTANT: BP_LINE_1 (line 4) is the first executable line where `perl -d`
// always pauses implicitly before processing any stdin commands.  With
// `stopOnEntry: false`, `configurationDone` sends `c` which runs FROM that
// implicit stop.  The Perl debugger does NOT re-trigger a breakpoint set on
// the line where execution is already paused, so a breakpoint at BP_LINE_1
// will be skipped by the initial `c`.  Tests that need a reliably-hit first
// breakpoint should use BP_LINE_2 or later.
const BP_LINE_1: u64 = 4; // my $x = 10 — initial implicit stop (skipped by configurationDone)
const BP_LINE_2: u64 = 5; // my $y = $x + 5
const BP_LINE_3: u64 = 6; // my $z = $x * $y

/// Minimal three-line body script.  Lines 1-3 are headers; executable code
/// starts at line 4, matching BP_LINE_1/BP_LINE_2 above.
fn workflow_script_content() -> &'static str {
    "use strict;\nuse warnings;\n\nmy $x = 10;\nmy $y = $x + 5;\nmy $z = $x * $y;\nprint \"$z\\n\";\n"
}

/// Shorthand for a test that returns `Ok(())` on success or propagates a boxed error.
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

    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

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

    // Retrieve stack trace → top frame id, source path, and line.
    let (frame_id, source_path, frame_line) = session.stack_trace(thread_id)?;
    assert!(
        source_path.contains("workflow_e2e"),
        "stack frame source path `{source_path}` should refer to the workflow fixture"
    );
    assert_eq!(
        frame_line, BP_LINE_1 as i64,
        "stack frame line must be {BP_LINE_1} (BP_LINE_1), got {frame_line}"
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
    assert_variables_have_names(&variables, "locals scope");

    // Continue to script exit.
    session.continue_exec(thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}

// ─── Test 2: multi-breakpoint sequence ────────────────────────────────────────

/// Validates that multiple breakpoints are hit in source order.
///
/// Uses BP_LINE_2 and BP_LINE_3 (not BP_LINE_1) because BP_LINE_1 is the
/// initial implicit stop line: `perl -d` pauses there before processing any
/// stdin, and the initial `c` from `configurationDone` runs past it without
/// re-triggering.  Breakpoints at BP_LINE_2 and BP_LINE_3 are reliably hit
/// in sequence.
#[test]
fn test_e2e_multi_breakpoint_sequence() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_multi_breakpoint_sequence - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_multi.pl");
    write(&script, workflow_script_content())?;

    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints(&script_str, &[BP_LINE_2, BP_LINE_3])?;
    session.configuration_done()?;

    // First stop — must be at BP_LINE_2.
    let first_stop = session.wait_stopped()?;
    assert_eq!(
        first_stop.reason, "breakpoint",
        "first stop reason must be `breakpoint`, got `{}`",
        first_stop.reason
    );

    // Verify the stack frame line — the stopped event doesn't carry a line
    // number, but stackTrace always does.
    let (_, _, first_line) = session.stack_trace(first_stop.thread_id)?;
    assert_eq!(
        first_line, BP_LINE_2 as i64,
        "first breakpoint must be at line {BP_LINE_2}, stack frame reports {first_line}"
    );

    // Continue to second breakpoint.
    session.continue_exec(first_stop.thread_id)?;
    let second_stop = session.wait_stopped()?;
    assert_eq!(
        second_stop.reason, "breakpoint",
        "second stop reason must be `breakpoint`, got `{}`",
        second_stop.reason
    );

    let (_, _, second_line) = session.stack_trace(second_stop.thread_id)?;
    assert_eq!(
        second_line, BP_LINE_3 as i64,
        "second breakpoint must be at line {BP_LINE_3}, stack frame reports {second_line}"
    );

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

    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

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

// ─── Test 4: attach workflow with stopOnEntry=false ────────────────────────────

/// Validates the attach workflow:
/// initialize → attach(pid, stopOnEntry=false) → wait for stopped(reason=attach) →
/// set breakpoints → disconnect.
#[test]
fn test_e2e_attach_workflow_stopped_event() -> TestResult {
    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    // Use an arbitrary non-zero PID (the adapter doesn't validate it exists)
    let test_pid = 12345u32;

    // Attach without stopOnEntry — should emit stopped(reason=attach)
    session.attach(test_pid, false)?;

    // Wait for the attach stopped event
    let attached = session.wait_stopped()?;
    assert_eq!(
        attached.reason, "attach",
        "stopped reason after attach must be `attach`, got `{}`",
        attached.reason
    );

    let _thread_id = attached.thread_id;

    // After attach, we can set breakpoints (the adapter accepts them)
    let workspace = tempdir()?;
    let script = workspace.path().join("dummy.pl");
    write(&script, workflow_script_content())?;
    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

    let bp_response = session.set_breakpoints(&script_str, &[BP_LINE_2])?;
    assert!(
        bp_response.is_some(),
        "setBreakpoints should succeed after attach"
    );

    session.disconnect()?;

    Ok(())
}

// ─── Test 5: attach workflow with stopOnEntry=true ──────────────────────────────

/// Validates attach with stopOnEntry=true:
/// attach(pid, stopOnEntry=true) should emit both "attach" and "entry" stopped events.
#[test]
fn test_e2e_attach_workflow_stop_on_entry() -> TestResult {
    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    let test_pid = 12346u32;

    // Attach with stopOnEntry=true
    session.attach(test_pid, true)?;

    // Should receive "attach" stopped event first
    let first_stop = session.wait_stopped()?;
    assert_eq!(
        first_stop.reason, "attach",
        "first stopped event after attach(stopOnEntry=true) must be reason=attach"
    );

    // Then should receive "entry" stopped event
    let entry_stop = session.wait_stopped()?;
    assert_eq!(
        entry_stop.reason, "entry",
        "second stopped event after attach(stopOnEntry=true) must be reason=entry"
    );

    session.disconnect()?;

    Ok(())
}

// ─── Test 6: step-into advances execution ────────────────────────────────────

/// Validates that `stepIn` (the DAP `stepIn` command) advances execution.
///
/// Uses the same proven fixture as the step-over test.  Since the fixture has
/// no sub calls on BP_LINE_2, `stepIn` degrades to a single-line step like
/// `next`, but the protocol behaviour is identical: the adapter sends `s` to
/// `perl -d` and must receive a `stopped(reason=step)` event.
///
/// A dedicated subroutine-stepping test would require a fixture where the
/// initial `c` runs reliably past the sub definition; that can be added in a
/// follow-up.  This test validates the DAP protocol round-trip.
#[test]
fn test_e2e_step_into_subroutine() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_step_into_subroutine - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_stepinto.pl");
    write(&script, workflow_script_content())?;

    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    // BP_LINE_2 (line 5): same rationale as step-over test — configurationDone's `c`
    // runs from the initial implicit stop at line 4 to the breakpoint at line 5.
    session.set_breakpoints(&script_str, &[BP_LINE_2])?;
    session.configuration_done()?;

    let at_breakpoint = session.wait_stopped()?;
    assert_eq!(at_breakpoint.reason, "breakpoint");

    let thread_id = at_breakpoint.thread_id;

    // stepIn on a non-sub-call line degrades to a single step; the adapter
    // still sends `s` to perl -d and must receive stopped(reason=step).
    session.step_into(thread_id)?;
    let after_step_in = session.wait_stopped()?;

    // After stepIn, reason must be "step"
    assert_eq!(
        after_step_in.reason, "step",
        "stop reason after stepIn must be `step`, got `{}`",
        after_step_in.reason
    );

    session.disconnect()?;

    Ok(())
}

// ─── Test 7: inspect global variables at a stopped frame ──────────────────────

/// Validates that global variables can be inspected:
/// stop at breakpoint → stackTrace → scopes → scopes_globals_ref → variables → inspect $variable
#[test]
fn test_e2e_globals_scope_inspection() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_globals_scope_inspection - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_globals.pl");

    // Script with a global variable
    let content = "use strict;\nuse warnings;\n\nour $global_var = 999;\nmy $x = 10;\nmy $y = $x + 5;\nprint \"$y\\n\";\n";
    write(&script, content)?;

    let script_str = script
        .to_str()
        .ok_or("script path is not valid UTF-8")?
        .to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;
    session.set_breakpoints(&script_str, &[BP_LINE_2])?;
    session.configuration_done()?;

    let stopped = session.wait_stopped()?;
    assert_eq!(stopped.reason, "breakpoint");

    let thread_id = stopped.thread_id;

    // Retrieve stack trace and get frame
    let (frame_id, _, _) = session.stack_trace(thread_id)?;

    // Retrieve globals scope reference
    let globals_ref = session.scopes_globals_ref(frame_id)?;
    assert!(
        globals_ref > 0,
        "globals scope variablesReference must be positive"
    );

    // Retrieve global variables
    let globals = session.variables(globals_ref)?;
    assert!(
        !globals.is_empty(),
        "globals scope must contain at least one variable"
    );

    // Verify variable entries have non-empty names
    assert_variables_have_names(&globals, "globals scope");

    session.disconnect()?;

    Ok(())
}

// ─── Test 8: hover evaluation for variable inspection ──────────────────────────

/// Validates DAP hover evaluation (context="hover") for variable inspection.
///
/// This test closes issue #3481 which reported that DAP hover evaluation was untested.
/// It performs an end-to-end test with a real `perl -d` session:
///
/// 1. Launch perl -d with a script declaring $scalar, @array, %hash, $ref
/// 2. Set breakpoint after variable declarations
/// 3. configurationDone → continue → wait_stopped
/// 4. For each variable, send evaluate request with context="hover"
/// 5. Assert success, non-empty result, and variables_reference==0
///
/// Key behavioral facts verified:
/// - variables_reference is ALWAYS 0 for evaluate responses (hardcoded in evaluation.rs:171)
/// - frame_id is accepted but unused by handle_evaluate()
/// - context is accepted but unused by handle_evaluate()
#[test]
fn test_e2e_hover_evaluation() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_e2e_hover_evaluation - perl not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_hover_eval.pl");

    // Minimal script for hover evaluation testing.
    // All variables are declared before the breakpoint line.
    let content = "use strict;\nuse warnings;\n\nmy $scalar = 42;\nmy @array = (1,2,3);\nmy %hash = (a=>1,b=>2);\nmy $ref = \\$scalar;\nprint \"done\n\";\n";
    write(&script, content)?;

    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    session.launch(&script_str)?;

    // Set breakpoint at line 7 (print statement, after all variables declared).
    const EVAL_LINE: u64 = 7; // print "done\n";
    session.set_breakpoints(&script_str, &[EVAL_LINE])?;
    session.configuration_done()?;

    let stopped = session.wait_stopped()?;
    assert_eq!(
        stopped.reason, "breakpoint",
        "stopped reason must be `breakpoint`, got `{}`",
        stopped.reason
    );

    let thread_id = stopped.thread_id;

    // Get frame_id (required by DAP even though handle_evaluate() currently ignores it)
    let (frame_id, _, _) = session.stack_trace(thread_id)?;

    // ── Hover evaluate $scalar ────────────────────────────────────────────────
    let scalar_body = session.evaluate("$scalar", "hover", Some(frame_id))?;
    {
        let result = scalar_body
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or("evaluate response missing `result` field")?;
        assert!(
            !result.is_empty(),
            "scalar evaluation result must be non-empty, got empty string"
        );
        assert!(
            result.contains("42"),
            "scalar evaluation result should contain '42', got `{}`",
            result
        );

        let variables_reference = scalar_body
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .ok_or("evaluate response missing `variablesReference` field")?;
        assert_eq!(
            variables_reference, 0,
            "evaluate response variablesReference must be 0 (hardcoded at evaluation.rs:171), got {}",
            variables_reference
        );
    }

    // ── Hover evaluate @array ─────────────────────────────────────────────────
    let array_body = session.evaluate("@array", "hover", Some(frame_id))?;
    {
        let result = array_body
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or("evaluate response missing `result` field")?;
        assert!(
            !result.is_empty(),
            "array evaluation result must be non-empty, got empty string"
        );
        assert!(
            result.chars().any(|c| c.is_numeric()),
            "array evaluation result should contain numeric values, got `{}`",
            result
        );

        let variables_reference = array_body
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .ok_or("evaluate response missing `variablesReference` field")?;
        assert_eq!(
            variables_reference, 0,
            "evaluate response variablesReference must be 0, got {}",
            variables_reference
        );
    }

    // ── Hover evaluate %hash ──────────────────────────────────────────────────
    let hash_body = session.evaluate("%hash", "hover", Some(frame_id))?;
    {
        let result = hash_body
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or("evaluate response missing `result` field")?;
        assert!(
            !result.is_empty(),
            "hash evaluation result must be non-empty, got empty string"
        );

        let variables_reference = hash_body
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .ok_or("evaluate response missing `variablesReference` field")?;
        assert_eq!(
            variables_reference, 0,
            "evaluate response variablesReference must be 0, got {}",
            variables_reference
        );
    }

    // ── Hover evaluate $ref (scalar reference) ────────────────────────────────
    let ref_body = session.evaluate("$ref", "hover", Some(frame_id))?;
    {
        let result = ref_body
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or("evaluate response missing `result` field")?;
        assert!(
            !result.is_empty(),
            "ref evaluation result must be non-empty, got empty string"
        );

        let variables_reference = ref_body
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .ok_or("evaluate response missing `variablesReference` field")?;
        assert_eq!(
            variables_reference, 0,
            "evaluate response variablesReference must be 0, got {}",
            variables_reference
        );
    }

    session.disconnect()?;

    Ok(())
}
