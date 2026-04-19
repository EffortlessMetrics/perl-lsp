//! Comprehensive unit tests for perl-subprocess-runtime crate.

use perl_subprocess_runtime::mock::{CommandInvocation, MockResponse, MockSubprocessRuntime};
use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

// ──────────────────────────── SubprocessOutput ────────────────────────────

#[test]
fn output_success_with_zero_status() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: 0,
    };
    assert!(output.success());
    Ok(())
}

#[test]
fn output_failure_with_nonzero_status() -> Result<(), Box<dyn std::error::Error>> {
    for code in [1, 2, -1, 127, 255] {
        let output = SubprocessOutput {
            stdout: vec![],
            stderr: vec![],
            status_code: code,
        };
        assert!(
            !output.success(),
            "status_code {code} should not be success"
        );
    }
    Ok(())
}

#[test]
fn output_stdout_lossy_valid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: b"hello world".to_vec(),
        stderr: vec![],
        status_code: 0,
    };
    assert_eq!(output.stdout_lossy(), "hello world");
    Ok(())
}

#[test]
fn output_stderr_lossy_valid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: b"some error".to_vec(),
        status_code: 1,
    };
    assert_eq!(output.stderr_lossy(), "some error");
    Ok(())
}

#[test]
fn output_stdout_lossy_with_invalid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![0xFF, 0xFE, b'a', b'b'],
        stderr: vec![],
        status_code: 0,
    };
    let lossy = output.stdout_lossy();
    assert!(lossy.contains("ab"));
    assert!(lossy.contains('\u{FFFD}'));
    Ok(())
}

#[test]
fn output_stderr_lossy_with_invalid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![b'e', 0x80, 0x81, b'r'],
        status_code: 1,
    };
    let lossy = output.stderr_lossy();
    assert!(lossy.contains('\u{FFFD}'));
    Ok(())
}

#[test]
fn output_empty_stdout_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: 0,
    };
    assert_eq!(output.stdout_lossy(), "");
    assert_eq!(output.stderr_lossy(), "");
    Ok(())
}

#[test]
fn output_clone() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: b"out".to_vec(),
        stderr: b"err".to_vec(),
        status_code: 42,
    };
    let cloned = output.clone();
    assert_eq!(cloned.stdout, output.stdout);
    assert_eq!(cloned.stderr, output.stderr);
    assert_eq!(cloned.status_code, output.status_code);
    Ok(())
}

#[test]
fn output_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![1],
        stderr: vec![2],
        status_code: 0,
    };
    let debug = format!("{output:?}");
    assert!(debug.contains("SubprocessOutput"));
    Ok(())
}

// ──────────────────────────── SubprocessError ─────────────────────────────

#[test]
fn error_new_from_str() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("something failed");
    assert_eq!(err.message, "something failed");
    Ok(())
}

#[test]
fn error_new_from_string() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new(String::from("owned message"));
    assert_eq!(err.message, "owned message");
    Ok(())
}

#[test]
fn error_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("display test");
    assert_eq!(format!("{err}"), "display test");
    Ok(())
}

#[test]
fn error_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("debug test");
    let debug = format!("{err:?}");
    assert!(debug.contains("SubprocessError"));
    assert!(debug.contains("debug test"));
    Ok(())
}

#[test]
fn error_implements_std_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("trait check");
    let _boxed: Box<dyn std::error::Error> = Box::new(err);
    Ok(())
}

#[test]
fn error_clone() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("clone me");
    let cloned = err.clone();
    assert_eq!(cloned.message, err.message);
    Ok(())
}

#[test]
fn error_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("");
    assert_eq!(format!("{err}"), "");
    Ok(())
}

// ──────────────────────────── MockResponse ────────────────────────────────

#[test]
fn mock_response_success() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success(b"output data".to_vec());
    assert_eq!(resp.stdout, b"output data");
    assert!(resp.stderr.is_empty());
    assert_eq!(resp.status_code, 0);
    Ok(())
}

#[test]
fn mock_response_success_empty() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success(Vec::new());
    assert!(resp.stdout.is_empty());
    assert_eq!(resp.status_code, 0);
    Ok(())
}

#[test]
fn mock_response_failure() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::failure(b"error text".to_vec(), 2);
    assert!(resp.stdout.is_empty());
    assert_eq!(resp.stderr, b"error text");
    assert_eq!(resp.status_code, 2);
    Ok(())
}

#[test]
fn mock_response_failure_with_negative_status() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::failure(b"signal".to_vec(), -9);
    assert_eq!(resp.status_code, -9);
    Ok(())
}

#[test]
fn mock_response_clone() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success(b"data".to_vec());
    let cloned = resp.clone();
    assert_eq!(cloned.stdout, resp.stdout);
    assert_eq!(cloned.stderr, resp.stderr);
    assert_eq!(cloned.status_code, resp.status_code);
    Ok(())
}

#[test]
fn mock_response_debug() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success(b"x".to_vec());
    let debug = format!("{resp:?}");
    assert!(debug.contains("MockResponse"));
    Ok(())
}

// ──────────────────────────── CommandInvocation ───────────────────────────

#[test]
fn command_invocation_clone() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "perl".to_string(),
        args: vec!["-e".to_string(), "1".to_string()],
        stdin: Some(b"input".to_vec()),
    };
    let cloned = inv.clone();
    assert_eq!(cloned.program, inv.program);
    assert_eq!(cloned.args, inv.args);
    assert_eq!(cloned.stdin, inv.stdin);
    Ok(())
}

#[test]
fn command_invocation_debug() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "echo".to_string(),
        args: vec!["hi".to_string()],
        stdin: None,
    };
    let debug = format!("{inv:?}");
    assert!(debug.contains("CommandInvocation"));
    assert!(debug.contains("echo"));
    Ok(())
}

// ──────────────────────────── MockSubprocessRuntime ───────────────────────

#[test]
fn mock_runtime_new_returns_empty_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    assert!(runtime.invocations().is_empty());
    Ok(())
}

#[test]
fn mock_runtime_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::default();
    assert!(runtime.invocations().is_empty());
    Ok(())
}

#[test]
fn mock_runtime_records_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _output = runtime.run_command("perltidy", &["-st", "-ce"], Some(b"my $x;"))?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].program, "perltidy");
    assert_eq!(invocations[0].args, vec!["-st", "-ce"]);
    assert_eq!(invocations[0].stdin, Some(b"my $x;".to_vec()));
    Ok(())
}

#[test]
fn mock_runtime_records_no_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _output = runtime.run_command("perl", &["-v"], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert!(invocations[0].stdin.is_none());
    Ok(())
}

#[test]
fn mock_runtime_records_empty_args() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _output = runtime.run_command("perl", &[], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert!(invocations[0].args.is_empty());
    Ok(())
}

#[test]
fn mock_runtime_records_multiple_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _o1 = runtime.run_command("cmd1", &["a"], None)?;
    let _o2 = runtime.run_command("cmd2", &["b", "c"], Some(b"in"))?;
    let _o3 = runtime.run_command("cmd3", &[], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 3);
    assert_eq!(invocations[0].program, "cmd1");
    assert_eq!(invocations[1].program, "cmd2");
    assert_eq!(invocations[2].program, "cmd3");
    Ok(())
}

#[test]
fn mock_runtime_clear_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _output = runtime.run_command("perl", &["-e", "1"], None)?;
    assert_eq!(runtime.invocations().len(), 1);

    runtime.clear_invocations();
    assert!(runtime.invocations().is_empty());
    Ok(())
}

#[test]
fn mock_runtime_clear_then_record_again() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _o1 = runtime.run_command("first", &[], None)?;
    runtime.clear_invocations();
    let _o2 = runtime.run_command("second", &[], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].program, "second");
    Ok(())
}

#[test]
fn mock_runtime_queued_response() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted".to_vec()));

    let output = runtime.run_command("perltidy", &[], None)?;
    assert!(output.success());
    assert_eq!(output.stdout_lossy(), "formatted");
    Ok(())
}

#[test]
fn mock_runtime_queued_responses_consumed_in_order() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"first".to_vec()));
    runtime.add_response(MockResponse::failure(b"second err".to_vec(), 1));
    runtime.add_response(MockResponse::success(b"third".to_vec()));

    let o1 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o1.stdout_lossy(), "first");
    assert!(o1.success());

    let o2 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o2.stderr_lossy(), "second err");
    assert!(!o2.success());

    let o3 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o3.stdout_lossy(), "third");
    assert!(o3.success());
    Ok(())
}

#[test]
fn mock_runtime_falls_back_to_default_response() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let output = runtime.run_command("anything", &[], None)?;
    assert!(output.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn mock_runtime_default_after_queue_exhausted() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"queued".to_vec()));

    let o1 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o1.stdout_lossy(), "queued");

    let o2 = runtime.run_command("cmd", &[], None)?;
    assert!(o2.success());
    assert!(o2.stdout.is_empty());
    Ok(())
}

#[test]
fn mock_runtime_set_default_response() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = MockSubprocessRuntime::new();
    runtime.set_default_response(MockResponse::failure(b"default err".to_vec(), 99));

    let output = runtime.run_command("cmd", &[], None)?;
    assert!(!output.success());
    assert_eq!(output.status_code, 99);
    assert_eq!(output.stderr_lossy(), "default err");
    Ok(())
}

#[test]
fn mock_runtime_set_default_does_not_affect_queued() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"from queue".to_vec()));
    runtime.set_default_response(MockResponse::failure(b"from default".to_vec(), 1));

    let o1 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o1.stdout_lossy(), "from queue");
    assert!(o1.success());

    let o2 = runtime.run_command("cmd", &[], None)?;
    assert_eq!(o2.stderr_lossy(), "from default");
    assert!(!o2.success());
    Ok(())
}

#[test]
fn mock_runtime_failure_response() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::failure(b"not found".to_vec(), 127));

    let output = runtime.run_command("missing", &[], None)?;
    assert!(!output.success());
    assert_eq!(output.status_code, 127);
    assert_eq!(output.stderr_lossy(), "not found");
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn mock_runtime_with_binary_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let binary_data: Vec<u8> = (0..=255).collect();
    let _output = runtime.run_command("binary", &[], Some(&binary_data))?;

    let invocations = runtime.invocations();
    assert_eq!(invocations[0].stdin.as_ref().map(|v| v.len()), Some(256));
    Ok(())
}

#[test]
fn mock_runtime_with_empty_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let _output = runtime.run_command("cmd", &[], Some(b""))?;

    let invocations = runtime.invocations();
    assert_eq!(invocations[0].stdin, Some(vec![]));
    Ok(())
}

// ──────────────────────────── SubprocessRuntime trait ─────────────────────

#[test]
fn mock_runtime_satisfies_subprocess_runtime_trait() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    let trait_obj: &dyn SubprocessRuntime = &runtime;
    let output = trait_obj.run_command("echo", &["test"], None)?;
    assert!(output.success());
    Ok(())
}

#[test]
fn mock_runtime_boxed_trait_object() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"boxed".to_vec()));

    let boxed: Box<dyn SubprocessRuntime> = Box::new(runtime);
    let output = boxed.run_command("cmd", &[], None)?;
    assert_eq!(output.stdout_lossy(), "boxed");
    Ok(())
}

// ──────────────────────────── OsSubprocessRuntime ────────────────────────

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
mod os_runtime_tests {
    use super::*;
    use perl_subprocess_runtime::OsSubprocessRuntime;

    #[test]
    fn os_runtime_new() -> Result<(), Box<dyn std::error::Error>> {
        let _runtime = OsSubprocessRuntime::new();
        Ok(())
    }

    #[test]
    fn os_runtime_default() -> Result<(), Box<dyn std::error::Error>> {
        let _runtime = OsSubprocessRuntime::new();
        Ok(())
    }

    #[test]
    fn os_runtime_echo() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["hello"], None)?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy().trim(), "hello");
        assert_eq!(output.status_code, 0);
        Ok(())
    }

    #[test]
    fn os_runtime_echo_multiple_args() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["hello", "world"], None)?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy().trim(), "hello world");
        Ok(())
    }

    #[test]
    fn os_runtime_with_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("cat", &[], Some(b"piped input"))?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), "piped input");
        Ok(())
    }

    #[test]
    fn os_runtime_nonexistent_program() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let result = runtime.run_command("nonexistent_binary_xyz_123", &[], None);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.message.contains("nonexistent_binary_xyz_123"));
        }
        Ok(())
    }

    #[test]
    fn os_runtime_failing_command() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("false", &[], None)?;
        assert!(!output.success());
        assert_ne!(output.status_code, 0);
        Ok(())
    }

    #[test]
    fn os_runtime_stderr_output() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("sh", &["-c", "echo error_msg >&2; exit 1"], None)?;
        assert!(!output.success());
        assert!(output.stderr_lossy().contains("error_msg"));
        Ok(())
    }

    #[test]
    fn os_runtime_satisfies_trait() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let trait_obj: &dyn SubprocessRuntime = &runtime;
        let output = trait_obj.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    #[test]
    fn os_runtime_no_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("true", &[], None)?;
        assert!(output.success());
        assert!(output.stdout.is_empty());
        Ok(())
    }
}
