//! ripr call-observation seam proof for the `cwd_override` conditional paths in
//! `process.rs` (`launch_debugger` lines 375-382, `check_syntax` lines 467-473).
//!
//! # What this proves
//!
//! The `cwd_override` parameter was introduced by PR #1769 so that a `cwd` field
//! in the DAP `launch` request is actually honoured: the debugged Perl process
//! must start in the user-specified directory, not in the script's parent directory.
//!
//! This test drives the changed code through the **real production caller** —
//! `DebugAdapter::handle_request("launch", ...)` — and asserts that the spawned
//! Perl process reports a working directory equal to the user-supplied `cwd`,
//! not to the directory that contains the `.pl` file.
//!
//! # Non-vacuity
//!
//! If the fix were reverted (i.e. `cwd_override` removed from `launch_debugger`
//! and `check_syntax`, so the process always runs in the script's parent), the
//! Perl variable `$proc_cwd` would hold the *scripts* directory path, not the
//! *user_cwd* directory path, and the `assert!` below would fail.

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use std::fs;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Line numbers in the fixture script below.
///
/// ```text
/// 1: use strict;
/// 2: use warnings;
/// 3: use Cwd;
/// 4: my $proc_cwd = Cwd::getcwd();   # captures actual process cwd at startup
/// 5: my $sentinel = 1;               # breakpoint here; $proc_cwd is already set
/// ```
const SCRIPT_BP_LINE: u64 = 5;

fn cwd_probe_script() -> &'static str {
    "use strict;\nuse warnings;\nuse Cwd;\nmy $proc_cwd = Cwd::getcwd();\nmy $sentinel = 1;\n"
}

/// Canonical seam proof: DAP launch with an explicit `cwd` field results in the
/// spawned Perl process running in the user-specified directory.
///
/// The script is placed in `scripts/` and the user-specified cwd is a separate
/// `userland/` directory.  After stopping at the breakpoint the test evaluates
/// `$proc_cwd` (captured by the Perl process itself at startup via `Cwd::getcwd()`)
/// and asserts it resolves to the `userland/` path, NOT to `scripts/`.
///
/// Failure mode without the fix: `$proc_cwd` would equal the `scripts/` directory
/// because `launch_debugger` would default to `Path::new(program).parent()`.
#[test]
fn cwd_override_process_runs_in_user_specified_directory() -> TestResult {
    if !perl_available() {
        eprintln!(
            "Skipping cwd_override_process_runs_in_user_specified_directory — perl not available"
        );
        return Ok(());
    }

    // Create two separate directories: one for the script, one for user-specified cwd.
    let workspace = tempdir()?;
    let scripts_dir = workspace.path().join("scripts");
    let userland_dir = workspace.path().join("userland");
    fs::create_dir(&scripts_dir)?;
    fs::create_dir(&userland_dir)?;

    // Write the probe script to scripts_dir (not userland_dir).
    let script_path = scripts_dir.join("probe_cwd.pl");
    fs::write(&script_path, cwd_probe_script())?;

    let script_str = script_path.to_str().ok_or("script path not valid UTF-8")?;
    let userland_str = userland_dir.to_str().ok_or("userland path not valid UTF-8")?;

    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    // Drive through the real DAP launch path with cwd = userland_dir.
    // This exercises handle_launch -> launch_debugger (cwd_override = Some(userland_dir))
    // and check_syntax (cwd_override = Some(userland_dir)).
    session.launch_with_cwd(script_str, userland_str)?;

    // Set a breakpoint at line 5 ($sentinel = 1) so that line 4 (Cwd::getcwd()) has
    // already been executed by the time we inspect $proc_cwd.
    session.set_breakpoints(script_str, &[SCRIPT_BP_LINE])?;
    session.configuration_done()?;

    let stopped = session.wait_stopped()?;
    assert_eq!(
        stopped.reason, "breakpoint",
        "expected stop reason 'breakpoint', got '{}'",
        stopped.reason
    );

    // Retrieve the top stack frame so we can evaluate in its context.
    let (frame_id, _source, _line) = session.stack_trace(stopped.thread_id)?;

    // Evaluate $proc_cwd — the cwd captured by Cwd::getcwd() at startup.
    // $proc_cwd is a plain scalar variable; it passes the safe-evaluator policy.
    let (actual_cwd, _ty) = session.evaluate_expression("$proc_cwd", frame_id)?;

    // The perl debugger's `x` command (used internally for evaluate) formats
    // string scalars as:  0  'value'
    // Strip everything up to and including the first opening quote, then the
    // trailing quote.  Fall back to the raw string if quote stripping fails.
    let actual_path = {
        let stripped = if let Some(after_open) = actual_cwd.find('\'') {
            let inner = &actual_cwd[after_open + 1..];
            if let Some(close) = inner.rfind('\'') { &inner[..close] } else { inner }
        } else {
            actual_cwd.trim()
        };
        stripped.replace('\\', "/")
    };

    // The two sibling directories are distinguished only by their last component
    // ("userland" vs "scripts"), both under the same unique tempdir root.
    // Checking for the last component is robust regardless of platform path format
    // (Windows C:\..., msys /c/..., UNC \\?\..., etc.).
    assert!(
        actual_path.ends_with("/userland") || actual_path == "userland",
        "Perl process cwd should end with '/userland' (the user-specified cwd).\n\
         Got: {actual_path}\n\
         (If it ends with '/scripts' the cwd_override branch was NOT taken — \
         the fix has been reverted or is broken.)"
    );

    // Negative: must NOT be the scripts directory.
    assert!(
        !actual_path.ends_with("/scripts"),
        "Perl process cwd must NOT be the scripts directory.\n\
         Got: {actual_path}\n\
         This means cwd_override was ignored and the old default (script parent dir) was used."
    );

    session.continue_exec(stopped.thread_id)?;
    let _ = session.drain_until_event("terminated");
    session.disconnect()?;

    Ok(())
}
