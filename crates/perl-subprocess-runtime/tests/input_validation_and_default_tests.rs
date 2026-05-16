//! Focused coverage tests for `perl-subprocess-runtime`.
//!
//! Covers:
//! - `OsSubprocessRuntime::default()` (was 0 executions before this file)
//! - `validate_command_input` branch variants: empty string, tab-only,
//!   newline-only, mixed whitespace, NUL in program, NUL in args, NUL in
//!   multiple args, empty args slice with valid program (Ok path)
//!
//! # What stays uncovered and why
//!
//! **Lines 71-72 (stdin write error), 77-78 (wait_with_output error in no-timeout
//! branch), 92-93 (try_wait error in timeout loop), 97-98 (wait_with_output error
//! in timeout branch)**: These closures are only reachable if the OS returns an
//! error for operations that succeed on a process we just spawned successfully.
//! There is no portable way to inject such an error without unsafe code or OS-level
//! tricks (closing the fd out from under the child).  Triggering them reliably in CI
//! without a flake is impractical.
//!
//! **Lines 109-123 (kill-failure branch — process exits between try_wait and kill)**:
//! This is an inherent race: the child must exit in the narrow window between
//! `try_wait()` returning `None` (not yet exited) and the subsequent `child.kill()`
//! call.  Reproducing this race deterministically without `unsafe` or OS-specific
//! signals is not feasible.  The branch is documented with a comment in production
//! code explaining that it is a best-effort handler.
//!
//! **mock.rs line 7 (mutex poison path)**: Requires a thread to panic while holding
//! the lock.  The only way to trigger this in a test is to deliberately panic inside
//! a `std::thread::spawn` closure while the Mutex is locked — which requires unsafe
//! or a carefully crafted panic hook.  The code path exists solely for robustness;
//! testing it would add significant complexity for a defensive handler.

#[cfg(not(target_arch = "wasm32"))]
mod os_default {
    use perl_subprocess_runtime::OsSubprocessRuntime;
    #[cfg(not(windows))]
    use perl_subprocess_runtime::SubprocessRuntime;

    /// `OsSubprocessRuntime` implements `Default` by delegating to `new()`.
    /// This test exercises the `Default` impl (lines 43-46 of os_runtime.rs),
    /// which had zero executions in the baseline coverage run.
    #[test]
    fn os_runtime_default_impl_delegates_to_new() -> Result<(), Box<dyn std::error::Error>> {
        // Calling `default()` must not panic and must produce a functional runtime.
        let _runtime = OsSubprocessRuntime::default();
        Ok(())
    }

    /// Confirm that a runtime created via `Default` can actually run a command,
    /// exercising the full construction-to-invocation path.
    #[cfg(not(windows))]
    #[test]
    fn os_runtime_default_runs_command_successfully() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::default();
        let output = runtime.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }
}

/// Tests for `validate_command_input` — the private helper that guards
/// `OsSubprocessRuntime::run_command`.  We exercise it through the public
/// `run_command` interface so that both branches of each condition are hit.
#[cfg(not(target_arch = "wasm32"))]
mod input_validation {
    use perl_subprocess_runtime::{OsSubprocessRuntime, SubprocessError, SubprocessRuntime};

    fn run_command_error(
        program: &str,
        args: &[&str],
    ) -> Result<SubprocessError, Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        match runtime.run_command(program, args, None) {
            Ok(output) => Err(format!(
                "command must fail validation before spawning; got status {}",
                output.status_code
            )
            .into()),
            Err(err) => Ok(err),
        }
    }

    // ── empty / whitespace-only program names ────────────────────────────────

    /// Completely empty string — the simplest empty-program case.
    /// `"".trim().is_empty()` is `true`, so this must be rejected.
    #[test]
    fn rejects_empty_string_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("", &[])?;
        assert!(
            err.message.contains("must not be empty"),
            "unexpected error message: {}",
            err.message
        );
        Ok(())
    }

    /// Single space — `" ".trim().is_empty()` is `true`.
    #[test]
    fn rejects_single_space_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error(" ", &[])?;
        assert!(err.message.contains("must not be empty"), "unexpected: {}", err.message);
        Ok(())
    }

    /// Tab character — `"\t".trim().is_empty()` is `true`.
    #[test]
    fn rejects_tab_only_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("\t", &[])?;
        assert!(err.message.contains("must not be empty"), "unexpected: {}", err.message);
        Ok(())
    }

    /// Newline character — `"\n".trim().is_empty()` is `true`.
    #[test]
    fn rejects_newline_only_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("\n", &[])?;
        assert!(err.message.contains("must not be empty"), "unexpected: {}", err.message);
        Ok(())
    }

    /// Mixed whitespace — spaces, tabs, and carriage returns.
    #[test]
    fn rejects_mixed_whitespace_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("  \t \r\n  ", &[])?;
        assert!(err.message.contains("must not be empty"), "unexpected: {}", err.message);
        Ok(())
    }

    // ── NUL byte in program name ─────────────────────────────────────────────

    /// NUL byte at the start of the program name.
    #[test]
    fn rejects_nul_at_start_of_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("\0perl", &[])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    /// NUL byte at the end of the program name.
    #[test]
    fn rejects_nul_at_end_of_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("perl\0", &[])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    /// NUL byte in the middle of the program name.
    #[test]
    fn rejects_nul_in_middle_of_program_name() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("per\0l", &[])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    // ── NUL byte in arguments ─────────────────────────────────────────────────

    /// NUL byte in the only argument.
    #[test]
    fn rejects_nul_in_single_arg() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("perl", &["-e\0"])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    /// NUL byte in the second of two arguments (checks that `any()` scans all).
    #[test]
    fn rejects_nul_in_second_arg() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("perl", &["-e", "print\0"])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    /// NUL byte in the third of three arguments.
    #[test]
    fn rejects_nul_in_third_arg() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("perl", &["-I", "lib", "scri\0pt.pl"])?;
        assert!(err.message.contains("NUL"), "unexpected: {}", err.message);
        Ok(())
    }

    /// NUL byte in argument but program name is also invalid (whitespace) —
    /// validates that the program check fires first.
    #[test]
    fn whitespace_program_checked_before_nul_in_arg() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("  ", &["arg\0"])?;
        // The "empty" check must fire before the NUL-in-args check.
        assert!(
            err.message.contains("must not be empty"),
            "expected empty-program error, got: {}",
            err.message
        );
        Ok(())
    }

    /// NUL in program fires before NUL in args.
    #[test]
    fn nul_in_program_checked_before_nul_in_arg() -> Result<(), Box<dyn std::error::Error>> {
        let err = run_command_error("per\0l", &["arg\0"])?;
        // The NUL-in-program check must fire before the NUL-in-args check.
        assert!(
            err.message.contains("program name must not contain NUL"),
            "expected NUL-in-program error, got: {}",
            err.message
        );
        Ok(())
    }

    // ── Ok paths through validate_command_input ───────────────────────────────

    /// Valid program with no args — hits the `Ok(())` return on line 148.
    /// This path is already covered by many other tests but is included here
    /// for documentary completeness of the Ok branch.
    #[cfg(not(windows))]
    #[test]
    fn valid_program_empty_args_passes_validation() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        // `true` is guaranteed to exist on all POSIX systems.
        let output = runtime.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    /// Valid program with args that contain no NUL bytes — all three guards pass.
    #[cfg(not(windows))]
    #[test]
    fn valid_program_with_args_passes_validation() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["hello", "world"], None)?;
        assert!(output.success());
        assert!(output.stdout_lossy().contains("hello"));
        Ok(())
    }
}

/// Tests that expose the `with_timeout` constructor and confirm the `timeout_secs`
/// field is set correctly via observable behaviour.
#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
mod timeout_constructor {
    use perl_subprocess_runtime::{OsSubprocessRuntime, SubprocessRuntime};

    /// `with_timeout(1)` should succeed for a fast command within the deadline.
    #[test]
    fn with_timeout_one_second_succeeds_for_instant_command()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::with_timeout(1);
        let output = runtime.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    /// `with_timeout` with a generous deadline should propagate stdout correctly.
    #[test]
    fn with_timeout_propagates_output() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::with_timeout(5);
        let output = runtime.run_command("echo", &["timeout-test"], None)?;
        assert!(output.success());
        assert!(output.stdout_lossy().trim() == "timeout-test");
        Ok(())
    }

    /// `with_timeout` with stdin should work (exercises the piped-stdin +
    /// timeout-poll path together).
    #[test]
    fn with_timeout_and_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::with_timeout(5);
        let output = runtime.run_command("cat", &[], Some(b"hello"))?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), "hello");
        Ok(())
    }

    /// A command that completes quickly under a tight timeout should succeed
    /// (guards against off-by-one where deadline fires before process finishes).
    #[test]
    fn with_timeout_tight_deadline_still_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        // The echo command finishes in << 1 second, so the 1-second deadline
        // must not misfire.
        let runtime = OsSubprocessRuntime::with_timeout(1);
        let output = runtime.run_command("echo", &["quick"], None)?;
        assert!(output.success());
        Ok(())
    }
}
