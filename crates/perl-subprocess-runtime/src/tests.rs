#[cfg(windows)]
use crate::os_runtime::{
    resolve_command_invocation, windows_program_priority, windows_quote_for_cmd,
};
use crate::*;

#[test]
fn test_subprocess_output_success() {
    let output = SubprocessOutput { stdout: vec![1, 2, 3], stderr: vec![], status_code: 0 };
    assert!(output.success());
}

#[test]
fn test_subprocess_output_failure() {
    let output = SubprocessOutput { stdout: vec![], stderr: b"error".to_vec(), status_code: 1 };
    assert!(!output.success());
    assert_eq!(output.stderr_lossy(), "error");
}

#[test]
fn test_subprocess_error_display() {
    let error = SubprocessError::new("test error");
    assert_eq!(format!("{}", error), "test error");
}

#[test]
fn test_mock_runtime() {
    use mock::*;

    let runtime = MockSubprocessRuntime::new();
    runtime.add_response(MockResponse::success(b"formatted code".to_vec()));

    let result = runtime.run_command("perltidy", &["-st"], Some(b"my $x = 1;"));

    assert!(result.is_ok());
    let output = perl_tdd_support::must(result);
    assert!(output.success());
    assert_eq!(output.stdout_lossy(), "formatted code");

    let invocations = runtime.invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].program, "perltidy");
    assert_eq!(invocations[0].args, vec!["-st"]);
    assert_eq!(invocations[0].stdin, Some(b"my $x = 1;".to_vec()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_echo() {
    let runtime = OsSubprocessRuntime::new();
    #[cfg(windows)]
    let result = runtime.run_command("cmd.exe", &["/C", "echo", "hello"], None);
    #[cfg(not(windows))]
    let result = runtime.run_command("echo", &["hello"], None);

    assert!(result.is_ok());
    let output = perl_tdd_support::must(result);
    assert!(output.success());
    assert!(output.stdout_lossy().trim() == "hello");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_nonexistent() {
    let runtime = OsSubprocessRuntime::new();

    let result = runtime.run_command("nonexistent_program_xyz", &[], None);

    assert!(result.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_rejects_empty_program_name() {
    let runtime = OsSubprocessRuntime::new();
    let result = runtime.run_command("   ", &["--version"], None);
    assert!(result.is_err());
    let err = result.expect_err("empty program name must be rejected");
    assert!(err.message.contains("must not be empty"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_os_runtime_rejects_nul_bytes_in_program_or_args() {
    let runtime = OsSubprocessRuntime::new();

    let bad_program = runtime.run_command("perl\0", &["--version"], None);
    assert!(bad_program.is_err());
    let bad_program_err = bad_program.expect_err("NUL in program must be rejected");
    assert!(bad_program_err.message.contains("NUL"));

    let bad_arg = runtime.run_command("perl", &["-e", "print \"ok\"\0"], None);
    assert!(bad_arg.is_err());
    let bad_arg_err = bad_arg.expect_err("NUL in arg must be rejected");
    assert!(bad_arg_err.message.contains("NUL"));
}

#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_uses_cmd_for_batch_wrappers() {
    let (program, args) =
        resolve_command_invocation(r"C:\Strawberry\perl\bin\perltidy.bat", &["-st", "-se"]);

    assert_eq!(program, "cmd.exe");
    assert_eq!(
        args,
        vec![
            "/D".to_string(),
            "/V:OFF".to_string(),
            "/S".to_string(),
            "/C".to_string(),
            "\"C:\\Strawberry\\perl\\bin\\perltidy.bat\" \"-st\" \"-se\"".to_string(),
        ]
    );
}

/// Verify that cmd.exe shell metacharacters are handled correctly inside
/// double-quoted tokens.
///
/// Inside a cmd.exe double-quoted region, shell metacharacters (`&`, `|`,
/// `<`, `>`, `(`, `)`) are already literal — no `^` prefix is used.
/// `^` is also literal and must not be doubled.
/// `%` is doubled to prevent `%VAR%` expansion.
/// `"` is doubled (`""`) per the cmd.exe shell convention.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_metacharacters_are_literal_inside_quotes() {
    // Metacharacters & | < > are passed through literally — no ^ prefix.
    // ^ is literal — must NOT be doubled.
    // % is doubled to prevent %VAR% expansion.
    // " is doubled (cmd.exe "" convention), not backslash-escaped.
    let quoted = windows_quote_for_cmd(r#"profile&name|1>%TEMP%^"x""#);
    assert_eq!(quoted, r#""profile&name|1>%%TEMP%%^""x""""#);
}

/// Verify that a caret in an argument is not doubled.
///
/// The original PR erroneously included `'^'` in the metacharacter match
/// arm, which caused `windows_quote_for_cmd("foo^bar")` to return
/// `"foo^^bar"` — delivering two carets to the program.  Inside a
/// cmd.exe double-quoted region `^` is literal and must not be escaped.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_caret_not_doubled() {
    let quoted = windows_quote_for_cmd(r"foo^bar");
    assert_eq!(quoted, r#""foo^bar""#);
}

/// Verify that an embedded double-quote uses the cmd.exe `""` convention.
///
/// The original PR used `\"` which is the `CommandLineToArgvW` / C-runtime
/// convention.  In cmd.exe context the backslash is literal and the `"`
/// terminates the quoted region, breaking argument boundaries.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_embedded_quote_uses_doubling() {
    let quoted = windows_quote_for_cmd(r#"arg"with"quotes"#);
    // cmd.exe convention: "" represents a literal " inside a quoted token.
    assert_eq!(quoted, r#""arg""with""quotes""#);
}

/// Verify that an attacker-controlled injection attempt is rendered inert.
///
/// An arg like `&calc.exe` must not break out of the quoted token.
/// After quoting, cmd.exe sees `&` as a literal character inside the
/// double-quoted region.
#[cfg(windows)]
#[test]
fn test_windows_quote_for_cmd_injection_attempt_is_inert() {
    let quoted = windows_quote_for_cmd("&calc.exe");
    assert_eq!(quoted, "\"&calc.exe\"");
}

/// Verify that /V:OFF is present in the cmd.exe argument list.
///
/// Without /V:OFF, cmd.exe with delayed expansion enabled would expand
/// `!VAR!` patterns inside arguments, which is an information-disclosure
/// vector and, in edge cases, an injection vector.
#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_includes_v_off_flag() {
    let (program, args) =
        resolve_command_invocation(r"C:\tools\perlcritic.bat", &["--profile=!TEMP!"]);

    assert_eq!(program, "cmd.exe");
    assert!(
        args.contains(&"/V:OFF".to_string()),
        "/V:OFF must be present to disable delayed expansion; got: {:?}",
        args
    );
}

#[cfg(windows)]
#[test]
fn test_resolve_command_invocation_preserves_executable_paths() {
    let (program, args) = resolve_command_invocation(r"C:\tools\perlcritic.exe", &["--version"]);

    assert_eq!(program, r"C:\tools\perlcritic.exe");
    assert_eq!(args, vec!["--version".to_string()]);
}

#[cfg(windows)]
#[test]
fn test_windows_program_priority_prefers_real_wrappers_over_extensionless_shims() {
    let mut candidates = [
        r"C:\Strawberry\perl\bin\perltidy".to_string(),
        r"C:\Strawberry\perl\bin\perltidy.bat".to_string(),
        r"C:\tools\perltidy.exe".to_string(),
    ];
    candidates.sort_by_key(|candidate| windows_program_priority(candidate));

    assert_eq!(candidates.last().map(String::as_str), Some(r"C:\tools\perltidy.exe"));
    assert!(
        windows_program_priority(r"C:\Strawberry\perl\bin\perltidy.bat")
            > windows_program_priority(r"C:\Strawberry\perl\bin\perltidy")
    );
}

// --- Lossy UTF-8 conversion contracts (invalid bytes) ---

#[test]
fn test_stdout_lossy_replaces_invalid_utf8() {
    // 0xFF is never valid UTF-8; lossy conversion must substitute U+FFFD and
    // preserve the valid prefix rather than panic or drop bytes.
    let output =
        SubprocessOutput { stdout: vec![b'o', b'k', 0xFF], stderr: Vec::new(), status_code: 0 };
    let text = output.stdout_lossy();
    assert!(text.starts_with("ok"), "valid prefix preserved: {text:?}");
    assert!(text.contains('\u{FFFD}'), "invalid byte replaced with U+FFFD: {text:?}");
}

#[test]
fn test_stderr_lossy_replaces_each_invalid_byte() {
    let output = SubprocessOutput { stdout: Vec::new(), stderr: vec![0xFF, 0xFE], status_code: 1 };
    // Two independent invalid bytes become two replacement characters.
    assert_eq!(output.stderr_lossy(), "\u{FFFD}\u{FFFD}");
}

#[test]
fn test_lossy_helpers_on_empty_streams_return_empty() {
    let output = SubprocessOutput { stdout: Vec::new(), stderr: Vec::new(), status_code: 0 };
    assert_eq!(output.stdout_lossy(), "");
    assert_eq!(output.stderr_lossy(), "");
}

// --- MockResponse constructors ---

#[test]
fn test_mock_response_success_sets_zero_status_and_empty_stderr() {
    let response = mock::MockResponse::success(b"out".to_vec());
    assert_eq!(response.stdout, b"out");
    assert!(response.stderr.is_empty(), "success carries no stderr");
    assert_eq!(response.status_code, 0);
}

#[test]
fn test_mock_response_failure_sets_stderr_and_status_with_empty_stdout() {
    let response = mock::MockResponse::failure(b"boom".to_vec(), 2);
    assert!(response.stdout.is_empty(), "failure carries no stdout");
    assert_eq!(response.stderr, b"boom");
    assert_eq!(response.status_code, 2);
}

// --- SubprocessError as a std::error::Error trait object ---

#[test]
fn test_subprocess_error_usable_as_std_error_trait_object() {
    let error = SubprocessError::new("disk full");
    let dyn_error: &dyn std::error::Error = &error;
    assert_eq!(dyn_error.to_string(), "disk full");
    assert!(dyn_error.source().is_none(), "leaf error has no source");
}
