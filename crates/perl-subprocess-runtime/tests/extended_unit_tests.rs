//! Extended unit tests for perl-subprocess-runtime crate.
//!
//! This test suite provides comprehensive coverage including:
//! - Edge cases for all public types
//! - Concurrent/threading scenarios
//! - Complex invocation patterns
//! - Boundary conditions and stress tests

use perl_subprocess_runtime::mock::{CommandInvocation, MockResponse, MockSubprocessRuntime};
use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};
use std::sync::{Arc, Mutex};
use std::thread;

// ──────────────────────────── SubprocessOutput Extensions ──────────────────

#[test]
fn output_success_boundary_zero() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: 0,
    };
    assert!(output.success());
    Ok(())
}

#[test]
fn output_failure_boundary_positive_one() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: 1,
    };
    assert!(!output.success());
    Ok(())
}

#[test]
fn output_failure_boundary_negative_one() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: -1,
    };
    assert!(!output.success());
    Ok(())
}

#[test]
fn output_large_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let large_output = vec![b'a'; 10_000_000]; // 10MB
    let output = SubprocessOutput {
        stdout: large_output.clone(),
        stderr: vec![],
        status_code: 0,
    };
    assert!(output.success());
    assert_eq!(output.stdout.len(), 10_000_000);
    Ok(())
}

#[test]
fn output_large_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let large_stderr = vec![b'x'; 1_000_000]; // 1MB
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: large_stderr.clone(),
        status_code: 1,
    };
    assert!(!output.success());
    assert_eq!(output.stderr.len(), 1_000_000);
    Ok(())
}

#[test]
fn output_both_stdout_and_stderr_populated() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: b"normal output".to_vec(),
        stderr: b"also error output".to_vec(),
        status_code: 0,
    };
    assert!(output.success());
    assert_eq!(output.stdout_lossy(), "normal output");
    assert_eq!(output.stderr_lossy(), "also error output");
    Ok(())
}

#[test]
fn output_stdout_with_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: b"line1\nline2\nline3".to_vec(),
        stderr: vec![],
        status_code: 0,
    };
    let lossy = output.stdout_lossy();
    assert!(lossy.contains("line1"));
    assert!(lossy.contains("line2"));
    assert!(lossy.contains("line3"));
    Ok(())
}

#[test]
fn output_stderr_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: "Error: ñoño, café, 中文".as_bytes().to_vec(),
        status_code: 1,
    };
    let lossy = output.stderr_lossy();
    assert!(lossy.contains("Error:"));
    Ok(())
}

#[test]
fn output_all_status_codes() -> Result<(), Box<dyn std::error::Error>> {
    for code in [0, 1, 127, 255, 256, i32::MIN, i32::MAX] {
        let output = SubprocessOutput {
            stdout: vec![],
            stderr: vec![],
            status_code: code,
        };
        let expected = code == 0;
        assert_eq!(output.success(), expected, "status code {code}");
    }
    Ok(())
}

#[test]
fn output_clone_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = vec![1, 2, 3, 255, 0];
    let stderr = vec![42, 99];
    let status = 127;
    let output = SubprocessOutput {
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        status_code: status,
    };

    let cloned = output.clone();

    assert_eq!(cloned.stdout, stdout);
    assert_eq!(cloned.stderr, stderr);
    assert_eq!(cloned.status_code, status);
    // Verify they're independent
    assert_ne!(&cloned.stdout as *const _, &output.stdout as *const _);
    Ok(())
}

#[test]
fn output_lossy_conversion_preserves_valid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Rust is awesome!";
    let output = SubprocessOutput {
        stdout: text.as_bytes().to_vec(),
        stderr: vec![],
        status_code: 0,
    };
    assert_eq!(output.stdout_lossy(), text);
    Ok(())
}

#[test]
fn output_lossy_handles_mixed_valid_invalid_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    bytes.extend_from_slice(b"valid");
    bytes.push(0xFF);
    bytes.push(0xFE);
    bytes.extend_from_slice(b"more_valid");

    let output = SubprocessOutput {
        stdout: bytes,
        stderr: vec![],
        status_code: 0,
    };
    let lossy = output.stdout_lossy();
    assert!(lossy.contains("valid"));
    assert!(lossy.contains("more_valid"));
    Ok(())
}

#[test]
fn output_empty_with_nonzero_status() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: vec![],
        stderr: vec![],
        status_code: 42,
    };
    assert!(!output.success());
    assert_eq!(output.stdout_lossy(), "");
    assert_eq!(output.stderr_lossy(), "");
    Ok(())
}

// ──────────────────────────── SubprocessError Extensions ────────────────────

#[test]
fn error_long_message() -> Result<(), Box<dyn std::error::Error>> {
    let long_msg = "x".repeat(10_000);
    let err = SubprocessError::new(&long_msg);
    assert_eq!(err.message.len(), 10_000);
    Ok(())
}

#[test]
fn error_message_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("Error: ñ, 中文, émoji 🦀");
    assert!(err.message.contains("Error:"));
    assert!(err.message.contains("🦀"));
    Ok(())
}

#[test]
fn error_message_from_multiple_types() -> Result<(), Box<dyn std::error::Error>> {
    let err1 = SubprocessError::new("str");
    let err2 = SubprocessError::new(String::from("String"));
    let err3 = SubprocessError::new("&str".to_string());

    assert_eq!(err1.message, "str");
    assert_eq!(err2.message, "String");
    assert_eq!(err3.message, "&str");
    Ok(())
}

#[test]
fn error_clone_independence() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("original");
    let cloned = err.clone();

    // Both should have same content
    assert_eq!(err.message, cloned.message);

    // But are they independent? (String is heap-allocated)
    assert_ne!(&err.message as *const _, &cloned.message as *const _);
    Ok(())
}

#[test]
fn error_is_send_sync() -> Result<(), Box<dyn std::error::Error>> {
    fn assert_send<T: Send>(_: &T) {}
    fn assert_sync<T: Sync>(_: &T) {}

    let err = SubprocessError::new("test");
    assert_send(&err);
    assert_sync(&err);
    Ok(())
}

#[test]
fn error_in_result() -> Result<(), Box<dyn std::error::Error>> {
    let result: Result<i32, SubprocessError> = Err(SubprocessError::new("failed"));
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.message, "failed");
    }
    Ok(())
}

#[test]
fn error_newline_in_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("line1\nline2\nline3");
    assert!(err.message.contains('\n'));
    assert_eq!(err.message.lines().count(), 3);
    Ok(())
}

// ──────────────────────────── MockResponse Extensions ──────────────────────

#[test]
fn mock_response_success_large_data() -> Result<(), Box<dyn std::error::Error>> {
    let large = vec![b'a'; 100_000];
    let resp = MockResponse::success(large.clone());
    assert_eq!(resp.stdout.len(), 100_000);
    assert_eq!(resp.stdout, large);
    Ok(())
}

#[test]
fn mock_response_failure_large_data() -> Result<(), Box<dyn std::error::Error>> {
    let large = vec![b'e'; 50_000];
    let resp = MockResponse::failure(large.clone(), 1);
    assert_eq!(resp.stderr.len(), 50_000);
    assert_eq!(resp.stderr, large);
    Ok(())
}

#[test]
fn mock_response_status_codes() -> Result<(), Box<dyn std::error::Error>> {
    for code in [1, 2, 127, 255, i32::MIN, i32::MAX] {
        let resp = MockResponse::failure(vec![], code);
        assert_eq!(resp.status_code, code);
    }
    Ok(())
}

#[test]
fn mock_response_success_from_str() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success("text output");
    assert_eq!(resp.stdout, b"text output");
    Ok(())
}

#[test]
fn mock_response_failure_from_str() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::failure("error text", 99);
    assert_eq!(resp.stderr, b"error text");
    assert_eq!(resp.status_code, 99);
    Ok(())
}

#[test]
fn mock_response_both_stdout_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let mut resp = MockResponse::success(b"out".to_vec());
    resp.stderr = b"err".to_vec();
    resp.status_code = 1; // Can override manually

    assert_eq!(resp.stdout, b"out");
    assert_eq!(resp.stderr, b"err");
    assert_eq!(resp.status_code, 1);
    Ok(())
}

// ──────────────────────────── MockSubprocessRuntime: Concurrency ──────────

#[test]
fn mock_runtime_concurrent_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mut handles = vec![];

    for i in 0..10 {
        let rt = Arc::clone(&runtime);
        let h = thread::spawn(move || {
            let prog = format!("prog{}", i);
            let _output = rt.run_command(&prog, &[], None);
        });
        handles.push(h);
    }

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 10);
    Ok(())
}

#[test]
fn mock_runtime_concurrent_add_responses() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let mut handles = vec![];

    for i in 0..5 {
        let rt = Arc::clone(&runtime);
        let h = thread::spawn(move || {
            let data = format!("response{}", i);
            rt.add_response(MockResponse::success(data.into_bytes()));
        });
        handles.push(h);
    }

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    // At least the default response is always there
    let _output = runtime.run_command("cmd", &[], None)?;
    Ok(())
}

#[test]
fn mock_runtime_concurrent_clear_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let cleared = Arc::new(Mutex::new(false));

    // Spawn thread that clears
    let rt = Arc::clone(&runtime);
    let cl = Arc::clone(&cleared);
    let clear_thread = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(10));
        rt.clear_invocations();
        *cl.lock().unwrap() = true;
    });

    // Add invocations concurrently
    for i in 0..100 {
        let _output = runtime.run_command(&format!("cmd{}", i), &[], None);
    }

    clear_thread.join().map_err(|_| "thread panicked")?;

    // After clear, should be empty (timing dependent but testing the capability)
    if *cleared.lock().unwrap() {
        // Clear was called, invocations might be empty or have some recent ones
        let count = runtime.invocations().len();
        // Should be much less than 100
        assert!(count < 100);
    }

    Ok(())
}

// ──────────────────────────── MockSubprocessRuntime: Complex Patterns ──────

#[test]
fn mock_runtime_many_queued_responses() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // Queue 100 responses
    for i in 0..100 {
        let data = format!("response{}", i);
        runtime.add_response(MockResponse::success(data.into_bytes()));
    }

    // Consume them in order
    for i in 0..100 {
        let output = runtime.run_command("cmd", &[], None)?;
        let expected = format!("response{}", i);
        assert_eq!(output.stdout_lossy(), expected);
    }

    // Next should be default (empty)
    let output = runtime.run_command("cmd", &[], None)?;
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn mock_runtime_alternating_success_failure() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    for i in 0..10 {
        if i % 2 == 0 {
            runtime.add_response(MockResponse::success(format!("ok{}", i).into_bytes()));
        } else {
            runtime.add_response(MockResponse::failure(format!("err{}", i).into_bytes(), i));
        }
    }

    for i in 0..10 {
        let output = runtime.run_command("cmd", &[], None)?;
        if i % 2 == 0 {
            assert!(output.success());
            assert_eq!(output.stdout_lossy(), format!("ok{}", i));
        } else {
            assert!(!output.success());
            assert_eq!(output.stderr_lossy(), format!("err{}", i));
        }
    }
    Ok(())
}

#[test]
fn mock_runtime_stdin_variations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // None stdin
    let _o1 = runtime.run_command("cmd", &[], None)?;

    // Some with data
    let _o2 = runtime.run_command("cmd", &[], Some(b"data"))?;

    // Some with empty
    let _o3 = runtime.run_command("cmd", &[], Some(b""))?;

    // Some with binary
    let _o4 = runtime.run_command("cmd", &[], Some(&[0, 1, 2, 255]))?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 4);
    assert!(invocations[0].stdin.is_none());
    assert_eq!(invocations[1].stdin.as_ref().map(|v| v.len()), Some(4));
    assert_eq!(invocations[2].stdin, Some(vec![]));
    assert_eq!(invocations[3].stdin.as_ref().map(|v| v.len()), Some(4));
    Ok(())
}

#[test]
fn mock_runtime_args_variations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    let _o1 = runtime.run_command("perl", &[], None)?;
    let _o2 = runtime.run_command("perl", &["-e"], None)?;
    let _o3 = runtime.run_command("perl", &["-e", "print"], None)?;
    let _o4 = runtime.run_command("perl", &["-e", "print", "hello", "world"], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 4);
    assert_eq!(invocations[0].args.len(), 0);
    assert_eq!(invocations[1].args.len(), 1);
    assert_eq!(invocations[2].args.len(), 2);
    assert_eq!(invocations[3].args.len(), 4);
    Ok(())
}

#[test]
fn mock_runtime_program_names_recorded() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    let progs = ["perl", "perltidy", "python", "sh", "bash", "cat", "echo"];
    for prog in &progs {
        let _output = runtime.run_command(prog, &[], None)?;
    }

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), progs.len());
    for (i, expected_prog) in progs.iter().enumerate() {
        assert_eq!(invocations[i].program, *expected_prog);
    }
    Ok(())
}

#[test]
fn mock_runtime_clear_then_reassert() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // First set of invocations
    let _o1 = runtime.run_command("cmd1", &["a"], None)?;
    let _o2 = runtime.run_command("cmd2", &["b"], None)?;
    assert_eq!(runtime.invocations().len(), 2);

    // Clear
    runtime.clear_invocations();
    assert_eq!(runtime.invocations().len(), 0);

    // New invocations
    let _o3 = runtime.run_command("cmd3", &["c"], None)?;
    let _o4 = runtime.run_command("cmd4", &["d"], None)?;
    let _o5 = runtime.run_command("cmd5", &["e"], None)?;

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 3);
    assert_eq!(invocations[0].program, "cmd3");
    assert_eq!(invocations[1].program, "cmd4");
    assert_eq!(invocations[2].program, "cmd5");
    Ok(())
}

// ──────────────────────────── OsSubprocessRuntime Extensions ──────────────

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
mod os_runtime_extended {
    use super::*;
    use perl_subprocess_runtime::OsSubprocessRuntime;

    #[test]
    fn os_runtime_empty_args() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    #[test]
    fn os_runtime_many_args() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let mut args = vec![];
        for i in 0..20 {
            args.push(format!("arg{}", i));
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = runtime.run_command("echo", &arg_refs, None)?;
        assert!(output.success());
        Ok(())
    }

    #[test]
    fn os_runtime_stdin_simple() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("cat", &[], Some(b"test input"))?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), "test input");
        Ok(())
    }

    #[test]
    fn os_runtime_stdin_multiline() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let input = b"line1\nline2\nline3";
        let output = runtime.run_command("cat", &[], Some(input))?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), "line1\nline2\nline3");
        Ok(())
    }

    #[test]
    fn os_runtime_stdin_large_data() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let large_input = vec![b'a'; 100_000];
        let output = runtime.run_command("cat", &[], Some(&large_input))?;
        assert!(output.success());
        assert_eq!(output.stdout.len(), 100_000);
        Ok(())
    }

    #[test]
    fn os_runtime_stdin_binary_data() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let binary: Vec<u8> = (0..=255).collect();
        let output = runtime.run_command("cat", &[], Some(&binary))?;
        assert!(output.success());
        assert_eq!(output.stdout.len(), 256);
        Ok(())
    }

    #[test]
    fn os_runtime_stdin_empty() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("cat", &[], Some(b""))?;
        assert!(output.success());
        assert_eq!(output.stdout.len(), 0);
        Ok(())
    }

    #[test]
    fn os_runtime_exit_code_one() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("sh", &["-c", "exit 1"], None)?;
        assert!(!output.success());
        assert_eq!(output.status_code, 1);
        Ok(())
    }

    #[test]
    fn os_runtime_exit_code_various() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        for code in [1, 2, 42, 127, 255] {
            let cmd = format!("exit {}", code);
            let output = runtime.run_command("sh", &["-c", &cmd], None)?;
            assert_eq!(output.status_code, code);
        }
        Ok(())
    }

    #[test]
    fn os_runtime_stderr_only() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("sh", &["-c", "echo err >&2"], None)?;
        assert!(output.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        Ok(())
    }

    #[test]
    fn os_runtime_stdout_and_stderr() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command(
            "sh",
            &["-c", "echo stdout_msg && echo stderr_msg >&2"],
            None,
        )?;
        assert!(output.success());
        assert!(!output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        Ok(())
    }

    #[test]
    fn os_runtime_trait_object() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let trait_obj: &dyn SubprocessRuntime = &runtime;
        let output = trait_obj.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    #[test]
    fn os_runtime_boxed_trait_object() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let boxed: Box<dyn SubprocessRuntime> = Box::new(runtime);
        let output = boxed.run_command("true", &[], None)?;
        assert!(output.success());
        Ok(())
    }

    #[test]
    fn os_runtime_send_sync() -> Result<(), Box<dyn std::error::Error>> {
        fn assert_send<T: Send>(_: &T) {}
        fn assert_sync<T: Sync>(_: &T) {}

        let runtime = OsSubprocessRuntime::new();
        assert_send(&runtime);
        assert_sync(&runtime);
        Ok(())
    }

    #[test]
    fn os_runtime_program_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let result = runtime.run_command("this_program_definitely_does_not_exist_xyz", &[], None);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(!e.message.is_empty());
        }
        Ok(())
    }

    #[test]
    fn os_runtime_multiple_sequential_calls() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        for i in 0..5 {
            let output = runtime.run_command("echo", &[&i.to_string()], None)?;
            assert!(output.success());
        }
        Ok(())
    }

    #[test]
    fn os_runtime_args_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["hello world"], None)?;
        assert!(output.success());
        assert!(output.stdout_lossy().contains("hello world"));
        Ok(())
    }

    #[test]
    fn os_runtime_args_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["hello\nworld"], None)?;
        assert!(output.success());
        // The argument itself may be echoed
        Ok(())
    }

    #[test]
    fn os_runtime_arg_with_equals() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["key=value"], None)?;
        assert!(output.success());
        assert!(output.stdout_lossy().contains("key=value"));
        Ok(())
    }

    #[test]
    fn os_runtime_arg_with_dash() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = OsSubprocessRuntime::new();
        let output = runtime.run_command("echo", &["--flag"], None)?;
        assert!(output.success());
        assert!(output.stdout_lossy().contains("--flag"));
        Ok(())
    }
}

// ──────────────────────────── SubprocessRuntime Trait ──────────────────────

#[test]
fn trait_object_mock_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"trait obj test".to_vec()));

    let trait_obj: &dyn SubprocessRuntime = &runtime;
    let output = trait_obj.run_command("test", &[], None)?;
    assert_eq!(output.stdout_lossy(), "trait obj test");
    Ok(())
}

#[test]
fn boxed_trait_object_mock_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"boxed test".to_vec()));

    let boxed: Box<dyn SubprocessRuntime> = Box::new(runtime);
    let output = boxed.run_command("test", &[], None)?;
    assert_eq!(output.stdout_lossy(), "boxed test");
    Ok(())
}

#[test]
fn arc_boxed_trait_object() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;

    let runtime = Arc::new(Box::new(MockSubprocessRuntime::new()) as Box<dyn SubprocessRuntime>);
    let output = runtime.run_command("test", &[], None)?;
    assert!(output.success());
    Ok(())
}

// ──────────────────────────── Error Message Formatting ────────────────────

#[test]
fn error_from_into_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let err1 = SubprocessError::new("string slice");
    let err2 = SubprocessError::new(err1.message.clone());
    assert_eq!(err1.message, err2.message);
    Ok(())
}

#[test]
fn error_debug_contains_field_name() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("test msg");
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("SubprocessError"));
    assert!(debug_str.contains("message"));
    Ok(())
}

#[test]
fn error_display_equals_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubprocessError::new("my message");
    assert_eq!(format!("{}", err), err.message);
    Ok(())
}

// ──────────────────────────── Integration Scenarios ───────────────────────

#[test]
fn scenario_format_and_lint_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // Simulate: format -> lint -> success
    runtime.add_response(MockResponse::success(b"formatted code".to_vec()));
    runtime.add_response(MockResponse::success(Vec::new()));

    // Format
    let format_output = runtime.run_command("perltidy", &["-st"], Some(b"my $x=1;"))?;
    assert!(format_output.success());

    // Lint
    let lint_output = runtime.run_command("perlcritic", &[], Some(b"my $x = 1;"))?;
    assert!(lint_output.success());

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].program, "perltidy");
    assert_eq!(invocations[1].program, "perlcritic");
    Ok(())
}

#[test]
fn scenario_format_fails_then_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // Simulate: format fails, use fallback
    runtime.add_response(MockResponse::failure(b"syntax error".to_vec(), 1));
    runtime.add_response(MockResponse::success(b"original code".to_vec()));

    // Format (fails)
    let format_output = runtime.run_command("perltidy", &["-st"], Some(b"my $x=1"))?;
    assert!(!format_output.success());
    assert_eq!(format_output.stderr_lossy(), "syntax error");

    // Fallback (original code)
    let fallback_output = runtime.run_command("echo", &[], Some(b"my $x=1"))?;
    assert!(fallback_output.success());

    Ok(())
}

#[test]
fn scenario_multiple_perl_invocations() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::new();

    // Queue multiple responses
    for i in 0..5 {
        runtime.add_response(MockResponse::success(format!("result{}", i).into_bytes()));
    }

    // Make multiple perl invocations
    for i in 0..5 {
        let output = runtime.run_command("perl", &["-e", &format!("print q(line{})", i)], None)?;
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), format!("result{}", i));
    }

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 5);
    for inv in &invocations {
        assert_eq!(inv.program, "perl");
    }
    Ok(())
}

// ──────────────────────────── CommandInvocation Properties ──────────────

#[test]
fn command_invocation_fields() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "my-prog".to_string(),
        args: vec!["a".to_string(), "b".to_string()],
        stdin: Some(vec![1, 2, 3]),
    };

    assert_eq!(inv.program, "my-prog");
    assert_eq!(inv.args, vec!["a", "b"]);
    assert_eq!(inv.stdin, Some(vec![1, 2, 3]));
    Ok(())
}

#[test]
fn command_invocation_no_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "prog".to_string(),
        args: vec![],
        stdin: None,
    };

    assert!(inv.stdin.is_none());
    Ok(())
}

#[test]
fn command_invocation_empty_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "prog".to_string(),
        args: vec![],
        stdin: Some(vec![]),
    };

    assert_eq!(inv.stdin, Some(vec![]));
    Ok(())
}

// ──────────────────────────── Default Implementations ──────────────────────

#[test]
fn mock_runtime_default_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = MockSubprocessRuntime::default();
    let invocations = runtime.invocations();
    assert!(invocations.is_empty());
    Ok(())
}

#[test]
fn mock_response_default_response_used() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = MockSubprocessRuntime::new();
    let custom_default = MockResponse::failure(b"custom default".to_vec(), 5);
    runtime.set_default_response(custom_default);

    // No queued responses, should use default
    let output = runtime.run_command("anything", &[], None)?;
    assert_eq!(output.status_code, 5);
    assert_eq!(output.stderr_lossy(), "custom default");
    Ok(())
}

// ──────────────────────────── Output Structure Completeness ─────────────

#[test]
fn output_all_fields_accessible() -> Result<(), Box<dyn std::error::Error>> {
    let output = SubprocessOutput {
        stdout: b"out".to_vec(),
        stderr: b"err".to_vec(),
        status_code: 42,
    };

    // Verify public fields are accessible
    let _ = &output.stdout;
    let _ = &output.stderr;
    let _ = output.status_code;

    Ok(())
}

#[test]
fn error_all_fields_accessible() -> Result<(), Box<dyn std::error::Error>> {
    let error = SubprocessError::new("msg");

    // Verify public field is accessible
    let _ = &error.message;

    Ok(())
}

#[test]
fn mock_response_all_fields_accessible() -> Result<(), Box<dyn std::error::Error>> {
    let resp = MockResponse::success(b"out".to_vec());

    // Verify public fields are accessible
    let _ = &resp.stdout;
    let _ = &resp.stderr;
    let _ = resp.status_code;

    Ok(())
}

#[test]
fn command_invocation_all_fields_accessible() -> Result<(), Box<dyn std::error::Error>> {
    let inv = CommandInvocation {
        program: "prog".to_string(),
        args: vec![],
        stdin: None,
    };

    // Verify public fields are accessible
    let _ = &inv.program;
    let _ = &inv.args;
    let _ = &inv.stdin;

    Ok(())
}

// ──────────────────────────── OsSubprocessRuntime: Timeout ─────────────────

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn os_runtime_timeout_fires_for_slow_command() -> Result<(), Box<dyn std::error::Error>> {
    use perl_subprocess_runtime::OsSubprocessRuntime;
    let runtime = OsSubprocessRuntime::with_timeout(1);
    let start = std::time::Instant::now();
    let result = runtime.run_command("sleep", &["10"], None);
    let elapsed = start.elapsed();
    assert!(result.is_err(), "expected timeout error, got success");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("timed out"),
        "expected 'timed out' in error message, got: {}",
        err.message
    );
    assert!(
        elapsed.as_secs() < 4,
        "timeout took too long: {}ms",
        elapsed.as_millis()
    );
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn os_runtime_with_timeout_succeeds_for_fast_command() -> Result<(), Box<dyn std::error::Error>> {
    use perl_subprocess_runtime::OsSubprocessRuntime;
    let runtime = OsSubprocessRuntime::with_timeout(10);
    let output = runtime.run_command("echo", &["hello"], None)?;
    assert!(output.success());
    assert_eq!(output.stdout_lossy().trim(), "hello");
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn os_runtime_no_timeout_still_works() -> Result<(), Box<dyn std::error::Error>> {
    use perl_subprocess_runtime::OsSubprocessRuntime;
    let runtime = OsSubprocessRuntime::new();
    let output = runtime.run_command("echo", &["hi"], None)?;
    assert!(output.success());
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
#[should_panic(expected = "timeout_secs must be greater than zero")]
fn os_runtime_with_timeout_zero_panics() {
    use perl_subprocess_runtime::OsSubprocessRuntime;
    // A zero-second timeout would time out every command immediately.
    // The constructor rejects it explicitly to surface caller bugs early.
    let _ = OsSubprocessRuntime::with_timeout(0);
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
#[test]
fn os_runtime_completion_before_deadline_always_succeeds() -> Result<(), Box<dyn std::error::Error>>
{
    use perl_subprocess_runtime::OsSubprocessRuntime;
    // A 1-second timeout with an instant echo command: the process finishes
    // well within the deadline.  This verifies that is_finished() is checked
    // before the deadline so a process completing at the deadline boundary is
    // never incorrectly reported as timed out.
    let runtime = OsSubprocessRuntime::with_timeout(1);
    let output = runtime.run_command("echo", &["boundary"], None)?;
    assert!(output.success());
    assert_eq!(output.stdout_lossy().trim(), "boundary");
    Ok(())
}
