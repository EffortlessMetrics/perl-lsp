use perl_lsp::execute_command::ExecuteCommandProvider;
use serde_json::Value;

#[test]
fn test_security_run_test_sub_injection_prevention() {
    let provider = ExecuteCommandProvider::new();

    // Malicious file path that injects code
    let malicious_path = "ignored'; print \"INJECTED\"; '";
    let sub_name = "test_sub";

    let result = provider.execute_command(
        "perl.runTestSub",
        vec![
            Value::String(malicious_path.to_string()),
            Value::String(sub_name.to_string())
        ]
    );

    // The command should fail because the file doesn't exist, OR succeed with empty output (if perl runs but does nothing)
    // But it should definitely NOT output "INJECTED".

    if let Ok(res) = result {
        let output = res["output"].as_str().unwrap_or("");
        assert!(!output.contains("INJECTED"), "Code injection detected! 'INJECTED' found in output");

        // It should probably fail with "File not found" or "Subroutine not found" depending on where it stops.
        // In our fix, `do $file` runs. If $file doesn't exist, it returns undef.
        // Then it checks defined &$sub.
        // So it should die with "Subroutine test_sub not found".
        // Or if I added check for file existence?
        // My fix:
        // do $file;
        // if ($@) { die ... }
        // ...
        // if (defined &{$sub}) ... else { die "Subroutine $sub not found" }

        let error = res["error"].as_str().unwrap_or("");
        // We expect it to complain about subroutine not found, because the file didn't load (so sub isn't defined).
        // OR if the file loaded (if I used a real file), it would run.
        // Since we use a fake file name that includes injection characters, it won't be found.
        assert!(error.contains("Subroutine test_sub not found") || error.contains("File not found") || output.contains("Subroutine test_sub not found"),
            "Expected error about missing subroutine or file, got output: '{}', error: '{}'", output, error);
    } else {
        // If it returns Err, that's also acceptable (e.g. if provider validates path presence)
        // But currently execute_command returns Ok with error field for command failures.
    }
}

#[test]
fn test_security_run_tests_arg_injection_prevention() {
    let provider = ExecuteCommandProvider::new();

    // Malicious file path that tries to inject flags
    let malicious_path = "-v";

    let result = provider.execute_command(
        "perl.runTests",
        vec![Value::String(malicious_path.to_string())]
    );

    if let Ok(res) = result {
        let output = res["output"].as_str().unwrap_or("");

        // If vulnerable, output contains perl version info (because -v is version flag)
        // If fixed, it tries to open file named "-v" and fails.
        assert!(!output.contains("This is perl"), "Argument injection detected! Perl version info found.");
    }
}

#[test]
fn test_security_run_file_arg_injection_prevention() {
    let provider = ExecuteCommandProvider::new();

    let malicious_path = "-v";

    let result = provider.execute_command(
        "perl.runFile",
        vec![Value::String(malicious_path.to_string())]
    );

    if let Ok(res) = result {
        let output = res["output"].as_str().unwrap_or("");
        assert!(!output.contains("This is perl"), "Argument injection detected in runFile! Perl version info found.");
    }
}
