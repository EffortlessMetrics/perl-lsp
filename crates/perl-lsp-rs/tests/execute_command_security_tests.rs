//! Security regression tests for executeCommand.
//!
//! These tests verify that command injection vulnerabilities in run_test_sub,
//! run_tests, and run_file have been properly mitigated, and that the
//! PerlOracleEnv isolation applied via `with_workspace_config` prevents
//! ambient env vars from leaking into the Perl subprocess.
//!
//! Note: With secure path resolution, malicious/non-existent paths are rejected
//! early at the path validation stage (returning Err), preventing execution entirely.

use perl_lsp::execute_command::ExecuteCommandProvider;
use perl_lsp_rs_core::config::WorkspaceConfig;
use serde_json::Value;
use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::path::Path;
use tempfile::TempDir;

/// Test that run_test_sub is protected against code injection via file_path.
///
/// The vulnerable code previously constructed:
/// `do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}`
/// which allowed injection through the file_path parameter.
///
/// With secure path resolution, non-existent files are rejected before execution.
/// The key security property is that the malicious code never reaches Perl.
#[test]
fn test_run_test_sub_file_path_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Payload that would inject code if string interpolation is used
    let malicious_file_path = "nonexistent.pl'; print 'INJECTED_VIA_FILE'; '";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(malicious_file_path.to_string()), Value::String("somesub".to_string())],
    );

    // With secure path resolution, non-existent files are rejected early
    // BEFORE any shell command or Perl code is executed
    assert!(result.is_err(), "Malicious path should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path resolution (file not found/canonicalize failure)
    // NOT about Perl code execution or subroutine lookup
    assert!(
        err.contains("canonicalize") || err.contains("Failed to"),
        "Error should be about path validation, not code execution: {}",
        err
    );

    // The path may be echoed in the error (this is fine - it's just a filename),
    // but the key is that no Perl code was executed with this malicious string.
    // The secure path resolution catches it at the Rust layer.
    Ok(())
}

/// Test that run_test_sub is protected against code injection via sub_name.
///
/// Note: The sub_name is passed via @ARGV, so the malicious string is treated
/// as a literal subroutine name to look up, not as code to execute. The error
/// message will contain the literal name (safe behavior), but the injected
/// code will NOT be executed.
#[test]
fn test_run_test_sub_subname_injection() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);

    // Create a minimal test file with a marker subroutine
    let test_file = temp_dir.path().join("security_test_sub.pl");
    std::fs::write(&test_file, "sub safe_sub { print 'SAFE_SUB_EXECUTED'; }")?;

    // This payload would execute code if string interpolation was used.
    // With the fix (using @ARGV), it's treated as a literal subroutine name.
    let malicious_sub_name = "safe_sub(); print 'INJECTED_CODE_RAN'";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(test_file.to_string_lossy().to_string()),
            Value::String(malicious_sub_name.to_string()),
        ],
    );

    assert!(result.is_ok(), "Command should not fail to spawn");
    let val = result?;
    let output = val["output"].as_str().ok_or("Missing 'output' field")?;

    // Key assertions:
    // 1. The injected print statement should NOT have executed
    assert!(
        !output.contains("INJECTED_CODE_RAN"),
        "Vulnerability: code injection via sub_name succeeded! Output: {}",
        output
    );

    // 2. The safe_sub should NOT have been called either (the malicious name
    //    includes "safe_sub()" but that should be treated literally, not executed)
    assert!(
        !output.contains("SAFE_SUB_EXECUTED"),
        "Unexpected: safe_sub was called despite malicious sub_name. Output: {}",
        output
    );

    // 3. The command should have failed because no subroutine with that literal name exists
    let success = val["success"].as_bool().ok_or("Missing 'success' field")?;
    assert!(!success, "Command should have failed (subroutine not found)");
    Ok(())
}

/// Test that run_file is protected against argument injection via file_path.
///
/// A file path starting with `-` could be interpreted as a flag without `--`.
/// With secure path resolution, non-existent files are rejected before execution.
#[test]
fn test_run_file_argument_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Payload that would be interpreted as a flag without `--` separator
    // `-e print 'INJECTED'` would execute arbitrary code
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runFile", vec![Value::String(malicious_file_path.to_string())]);

    // With secure path resolution, non-existent files are rejected early
    assert!(result.is_err(), "Malicious path '-e' should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path validation
    assert!(
        err.contains("canonicalize") || err.contains("not found"),
        "Error should be about path validation: {}",
        err
    );
    Ok(())
}

/// Test that run_tests is protected against argument injection via file_path.
/// With secure path resolution, non-existent files are rejected before execution.
#[test]
fn test_run_tests_argument_injection() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // Similar test for run_tests
    let malicious_file_path = "-e";

    let result = provider
        .execute_command("perl.runTests", vec![Value::String(malicious_file_path.to_string())]);

    // With secure path resolution, non-existent files are rejected early
    assert!(result.is_err(), "Malicious path '-e' should be rejected during path resolution");
    let err = result.err().ok_or("Expected error but got Ok")?;

    // The error should be about path validation
    assert!(
        err.contains("canonicalize") || err.contains("not found"),
        "Error should be about path validation: {}",
        err
    );
    Ok(())
}

/// Test that file paths with shell metacharacters are safely rejected.
///
/// With secure path resolution, files that don't exist are rejected before
/// any shell command is executed, preventing shell metacharacter expansion.
#[test]
fn test_shell_metacharacter_safety() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    // File paths with shell metacharacters that could cause issues
    // if shell expansion occurred. These non-existent paths should be
    // rejected during path validation before any shell execution.
    let dangerous_paths = vec![
        "/tmp/test$(whoami).pl",
        "/tmp/test`id`.pl",
        "/tmp/test;rm -rf /.pl",
        "/tmp/test|cat /etc/passwd.pl",
        "/tmp/test&& echo pwned.pl",
    ];

    for path in dangerous_paths {
        let result =
            provider.execute_command("perl.runFile", vec![Value::String(path.to_string())]);

        // Non-existent paths should be rejected during path resolution
        assert!(result.is_err(), "Non-existent path should be rejected: {}", path);
        let err = result.err().ok_or("Expected error but got Ok")?;

        // Error should be about path validation, not shell execution
        assert!(
            err.contains("canonicalize") || err.contains("not found"),
            "Error should be about path validation for {}: {}",
            path,
            err
        );
    }
    Ok(())
}

/// Test that valid files with safe paths execute correctly.
#[test]
fn test_valid_file_execution() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_valid.pl");
    fs::write(&file_path, "print 'VALID_OUTPUT';")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);

    let result = provider.execute_command(
        "perl.runFile",
        vec![Value::String(file_path.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "Valid file should execute successfully");
    let val = result?;
    let output = val["output"].as_str().ok_or("Missing 'output' field")?;

    assert!(output.contains("VALID_OUTPUT"), "Output should contain expected result: {}", output);
    Ok(())
}

// ============= Slice E: executeCommand Hardening Tests =============
// These tests verify the CWD boundary fallback, path traversal protection,
// argument length caps, and command injection prevention.

/// Test that commands are rejected when workspace_roots is empty and path is outside CWD
#[test]
fn test_empty_workspace_roots_enforces_cwd_boundary() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();
    // Provider has empty workspace_roots by default, which now falls back to CWD.
    // Use a real file in a temp directory so the path exists but is still outside the repo CWD.
    let temp_dir = TempDir::new()?;
    let outside_file = temp_dir.path().join("outside_cwd.pl");
    fs::write(&outside_file, "print 'outside cwd';")?;

    let result = provider.execute_command(
        "perl.runCritic",
        vec![Value::String(outside_file.to_string_lossy().to_string())],
    );

    assert!(result.is_err(), "Should reject paths outside CWD when workspace_roots is empty");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_command_exists_does_not_execute_path_hijacked_which() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let which_path = temp_dir.path().join("which");
    let marker_path = temp_dir.path().join("which-executed.marker");

    fs::write(
        &which_path,
        format!(
            "#!/bin/sh
printf 'executed' > '{}'
exit 0
",
            marker_path.display()
        ),
    )?;
    fs::set_permissions(&which_path, fs::Permissions::from_mode(0o755))?;

    let original_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![temp_dir.path().to_path_buf()];
    path_entries.extend(std::env::split_paths(&original_path));
    let joined_path = std::env::join_paths(path_entries)?;

    // SAFETY: test-only PATH mutation; restored before returning.
    unsafe {
        std::env::set_var("PATH", &joined_path);
    }
    let _ = perl_lsp::execute_command::command_exists("perlcritic");
    // SAFETY: restore process environment for test isolation.
    unsafe {
        std::env::set_var("PATH", original_path);
    }

    assert!(
        !Path::new(&marker_path).exists(),
        "security regression: command_exists executed a PATH-hijacked 'which' probe"
    );
    Ok(())
}
#[cfg(unix)]
#[test]
fn test_command_exists_does_not_execute_candidate_binary() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let script_path = temp_dir.path().join("fake-security-probe");
    let marker_path = temp_dir.path().join("executed.marker");

    fs::write(
        &script_path,
        format!("#!/bin/sh\nprintf 'executed' > '{}'\nexit 0\n", marker_path.display()),
    )?;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;

    let original_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![temp_dir.path().to_path_buf()];
    path_entries.extend(std::env::split_paths(&original_path));
    let joined_path = std::env::join_paths(path_entries)?;

    // SAFETY: test-only PATH mutation; restored before returning.
    unsafe {
        std::env::set_var("PATH", &joined_path);
    }
    let exists = perl_lsp::execute_command::command_exists("fake-security-probe");
    // SAFETY: restore process environment for test isolation.
    unsafe {
        std::env::set_var("PATH", original_path);
    }

    assert!(exists, "command_exists should report that the candidate is discoverable in PATH");
    assert!(
        !Path::new(&marker_path).exists(),
        "security regression: command_exists executed the candidate binary instead of probing PATH"
    );
    Ok(())
}

/// Test that path traversal via .. is rejected
#[test]
fn test_path_traversal_with_dot_dot() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let result = provider
        .execute_command("perl.runCritic", vec![Value::String("../../../etc/passwd".to_string())]);

    assert!(result.is_err(), "Path traversal with .. should be rejected");
    let err = result.err().ok_or("Expected error")?;
    assert!(
        err.contains("traversal") || err.contains(".."),
        "Error should mention traversal: {}",
        err
    );
    Ok(())
}

/// Test that extremely long arguments are rejected
#[test]
fn test_argument_length_cap() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let long_path = "a".repeat(5000);
    let result = provider.execute_command("perl.runCritic", vec![Value::String(long_path)]);

    assert!(result.is_err(), "Extremely long arguments should be rejected");
    let err = result.err().ok_or("Expected error")?;
    assert!(
        err.contains("too long") || err.contains("4096"),
        "Error should mention length limit: {}",
        err
    );
    Ok(())
}

/// Test command injection attempts in file paths
#[test]
fn test_command_injection_semicolon() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let malicious = "; rm -rf /tmp/test";
    let result =
        provider.execute_command("perl.runCritic", vec![Value::String(malicious.to_string())]);

    // Should fail at path validation, not reach shell execution.
    // perl.runCritic returns graceful error responses for file-not-found,
    // so check either Err or Ok-with-error-status.
    match result {
        Err(_) => {} // Rejected outright - good
        Ok(val) => {
            assert_eq!(val["status"], "error", "Should report error status for malicious path");
        }
    }
    Ok(())
}

/// Test command injection via backticks
#[test]
fn test_command_injection_backticks() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let malicious = "`echo pwned`";
    let result =
        provider.execute_command("perl.runCritic", vec![Value::String(malicious.to_string())]);

    match result {
        Err(_) => {} // Rejected outright - good
        Ok(val) => {
            assert_eq!(val["status"], "error", "Should report error status for backtick injection");
        }
    }
    Ok(())
}

/// Test command injection via $()
#[test]
fn test_command_injection_dollar_paren() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let malicious = "$(cat /etc/shadow)";
    let result =
        provider.execute_command("perl.runCritic", vec![Value::String(malicious.to_string())]);

    match result {
        Err(_) => {} // Rejected outright - good
        Ok(val) => {
            assert_eq!(val["status"], "error", "Should report error status for $() injection");
        }
    }
    Ok(())
}

/// Test pipe injection
#[test]
fn test_command_injection_pipe() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();

    let malicious = "| cat /etc/passwd";
    let result =
        provider.execute_command("perl.runCritic", vec![Value::String(malicious.to_string())]);

    match result {
        Err(_) => {} // Rejected outright - good
        Ok(val) => {
            assert_eq!(val["status"], "error", "Should report error status for pipe injection");
        }
    }
    Ok(())
}

// ── PerlOracleEnv poisoned-env tests (#8684) ──────────────────────────────────
//
// Verify that `run_file` and `run_test_sub` do NOT propagate PERL5LIB or
// PERL5OPT to the Perl subprocess unless the workspace config explicitly
// allows them — regression guard for the #8493-class incident.
//
// These tests require a real Perl installation; they are skipped when no Perl
// binary can be resolved.

/// Serializes tests that mutate process-global env vars.
///
/// `std::env::set_var` / `remove_var` are process-global and unsafe in Rust
/// 2024. This mutex ensures that concurrent test threads do not race over the
/// same env vars when running with `RUST_TEST_THREADS > 1`.
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Helper: resolve the system Perl binary via the toolchain resolver.
fn find_perl() -> Option<std::path::PathBuf> {
    perl_lsp_rs_core::platform::resolve_perl_path_with_toolchain().ok()
}

/// Helper: build a minimal `WorkspaceConfig` with a resolved Perl binary and
/// `use_perl5lib` set as requested.
fn config_with_perl5lib(use_perl5lib: bool) -> Option<WorkspaceConfig> {
    let perl = find_perl()?;
    let mut config = WorkspaceConfig::default();
    config.perl_path = Some(perl.to_string_lossy().into_owned());
    config.use_perl5lib = use_perl5lib;
    Some(config)
}

/// `run_file` with `use_perl5lib=false` must strip PERL5LIB from the subprocess.
///
/// Regression guard for the #8493-class incident: ambient PERL5LIB must not
/// reach the subprocess when the workspace config opts out.
#[test]
#[allow(unsafe_code)] // set_var / remove_var are unsafe in Rust 2024
fn run_file_strips_perl5lib_when_use_perl5lib_false() -> Result<(), Box<dyn Error>> {
    let config = match config_with_perl5lib(false) {
        Some(c) => c,
        None => return Ok(()), // no Perl — skip
    };

    let temp_dir = TempDir::new()?;
    // Script prints the PERL5LIB env var or "UNSET" if absent.
    let script = temp_dir.path().join("check_env.pl");
    std::fs::write(&script, "print $ENV{PERL5LIB} // 'UNSET';\n")?;

    let poison_dir = TempDir::new()?;
    let poison_path = poison_dir.path().to_string_lossy().into_owned();

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()])
            .with_workspace_config(config);

    // Serialize env-var mutation across concurrent tests.
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    // SAFETY: test-only; ENV_MUTEX serializes these mutations.
    unsafe { std::env::set_var("PERL5LIB", &poison_path) };
    let result = provider
        .execute_command("perl.runFile", vec![Value::String(script.to_string_lossy().to_string())]);
    unsafe { std::env::remove_var("PERL5LIB") };
    drop(_guard);

    let result = result?;
    let output = result["output"].as_str().unwrap_or("");
    assert!(
        !output.contains(&poison_path),
        "PERL5LIB poison path ({poison_path}) must NOT appear in subprocess output \
         when use_perl5lib=false; got: {output:?}",
    );
    assert!(
        output.contains("UNSET"),
        "subprocess should see PERL5LIB as UNSET when use_perl5lib=false; got: {output:?}",
    );
    Ok(())
}

/// `run_file` with `use_perl5lib=true` must pass PERL5LIB through to the subprocess.
#[test]
#[allow(unsafe_code)]
fn run_file_passes_perl5lib_when_use_perl5lib_true() -> Result<(), Box<dyn Error>> {
    let config = match config_with_perl5lib(true) {
        Some(c) => c,
        None => return Ok(()),
    };

    let temp_dir = TempDir::new()?;
    let script = temp_dir.path().join("check_env.pl");
    std::fs::write(&script, "print $ENV{PERL5LIB} // 'UNSET';\n")?;

    let marker_dir = TempDir::new()?;
    let marker_path = marker_dir.path().to_string_lossy().into_owned();

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()])
            .with_workspace_config(config);

    let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("PERL5LIB", &marker_path) };
    let result = provider
        .execute_command("perl.runFile", vec![Value::String(script.to_string_lossy().to_string())]);
    unsafe { std::env::remove_var("PERL5LIB") };
    drop(_guard);

    let result = result?;
    let output = result["output"].as_str().unwrap_or("");
    assert!(
        output.contains(&marker_path),
        "PERL5LIB must be passed through when use_perl5lib=true; got: {output:?}",
    );
    Ok(())
}

/// `run_test_sub` with `use_perl5lib=false` must strip PERL5LIB from the subprocess.
#[test]
#[allow(unsafe_code)]
fn run_test_sub_strips_perl5lib_when_use_perl5lib_false() -> Result<(), Box<dyn Error>> {
    let config = match config_with_perl5lib(false) {
        Some(c) => c,
        None => return Ok(()),
    };

    let temp_dir = TempDir::new()?;
    // A Perl file with a sub that prints the env var.
    let script = temp_dir.path().join("check_env_sub.pl");
    std::fs::write(&script, "sub check_env { print $ENV{PERL5LIB} // 'UNSET'; }\n")?;

    let poison_dir = TempDir::new()?;
    let poison_path = poison_dir.path().to_string_lossy().into_owned();

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()])
            .with_workspace_config(config);

    let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    unsafe { std::env::set_var("PERL5LIB", &poison_path) };
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(script.to_string_lossy().to_string()),
            Value::String("check_env".to_string()),
        ],
    );
    unsafe { std::env::remove_var("PERL5LIB") };
    drop(_guard);

    let result = result?;
    let output = result["output"].as_str().unwrap_or("");
    assert!(
        !output.contains(&poison_path),
        "PERL5LIB poison path must NOT appear when use_perl5lib=false; got: {output:?}",
    );
    Ok(())
}
