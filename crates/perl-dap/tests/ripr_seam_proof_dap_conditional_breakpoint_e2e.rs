//! Live DAP seam proof for conditional line breakpoints.
//!
//! The adapter should persist and forward conditions to `perl -d`; it should
//! not simulate Perl condition semantics in Rust.

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use serde_json::{Value, json};
use std::fs::write;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CONDITIONAL_BP_LINE: u64 = 5;

fn conditional_breakpoint_script_content() -> &'static str {
    "use strict;\nuse warnings;\n\nfor my $x (1..3) {\n    my $observed = $x;\n    print \"$observed\\n\";\n}\n"
}

fn set_conditional_breakpoint_checked(
    session: &mut DapWorkflowSession,
    source_path: &str,
    line: u64,
    condition: &str,
) -> Result<i64, String> {
    let response = session.request(
        "setBreakpoints",
        Some(json!({
            "source": { "path": source_path },
            "breakpoints": [{
                "line": line,
                "condition": condition
            }]
        })),
    );
    let body = session
        .expect_success(&response, "setBreakpoints")?
        .ok_or("setBreakpoints returned no body")?;
    let breakpoints = body
        .get("breakpoints")
        .and_then(Value::as_array)
        .ok_or("setBreakpoints response missing `breakpoints` array")?;
    let breakpoint =
        breakpoints.first().ok_or("setBreakpoints response returned no breakpoints")?;
    let verified = breakpoint.get("verified").and_then(Value::as_bool).unwrap_or(false);
    if !verified {
        let message = breakpoint.get("message").and_then(Value::as_str).unwrap_or("<no message>");
        return Err(format!("conditional breakpoint at line {line} was not verified: {message}"));
    }
    let resolved_line = breakpoint.get("line").and_then(Value::as_i64).unwrap_or(line as i64);
    Ok(resolved_line)
}

#[test]
fn conditional_breakpoint_stops_when_perl_condition_is_true() -> TestResult {
    if !perl_available() {
        return Ok(());
    }

    let workspace = tempdir()?;
    let script = workspace.path().join("workflow_conditional_breakpoint.pl");
    write(&script, conditional_breakpoint_script_content())?;

    let script_str = script.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    let mut session = DapWorkflowSession::new(workflow_timeout())?;
    session.launch(&script_str)?;
    let resolved_line = set_conditional_breakpoint_checked(
        &mut session,
        &script_str,
        CONDITIONAL_BP_LINE,
        "$x > 2",
    )?;
    session.configuration_done()?;

    let stopped = session.wait_stopped_with_frame()?;
    assert_eq!(
        stopped.reason, "breakpoint",
        "conditional breakpoint stop reason must be `breakpoint`, got `{}`",
        stopped.reason
    );
    assert_eq!(
        stopped.line, resolved_line,
        "conditional breakpoint stopped at line {}, expected adapter-resolved line {}",
        stopped.line, resolved_line
    );

    let (x_value, _) = session.evaluate_expression("$x", stopped.frame_id)?;
    assert!(
        x_value.contains('3'),
        "conditional breakpoint `$x > 2` should stop on the third loop iteration, got `$x`={x_value:?}"
    );

    session.continue_exec(stopped.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}
