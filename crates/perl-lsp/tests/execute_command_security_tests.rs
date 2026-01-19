#[cfg(test)]
mod tests {
    use perl_lsp::execute_command::ExecuteCommandProvider;
    use serde_json::Value;

    #[test]
    fn test_run_test_sub_vulnerability_repro() {
        let provider = ExecuteCommandProvider::new();

        // Payload to inject code via file_path
        // The vulnerable code constructs: "do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}"
        // We inject into the first {}
        let malicious_file_path = "nonexistent.pl'; print 'INJECTED_CODE'; '";

        let result = provider.execute_command(
            "perl.runTestSub",
            vec![Value::String(malicious_file_path.to_string()), Value::String("somesub".to_string())]
        );

        if let Ok(val) = result {
            let output = val["output"].as_str().unwrap_or("");
            // If the output contains "INJECTED_CODE", the vulnerability exists.
            assert!(!output.contains("INJECTED_CODE"), "Vulnerability detected: arbitrary code execution via file path!");
        } else {
             // If it fails, check if it failed due to the injection not working (safe) or syntax error (might still be unsafe but broken)
             // But for now, we assume if it returns OK and has output, check output.
             // The command execution itself might succeed (exit code 0) even if `do` fails, because we added valid perl code after it.
             // Actually `do` failing doesn't exit perl.
             // But we have `if (defined &somesub) ... else { die ... }`.
             // `die` will cause non-zero exit code.
             // So `success` might be false.
             // But `output` should still contain "INJECTED_CODE".

             // Wait, execute_command returns Ok(Value) even if command failed (exit code != 0),
             // unless command execution *itself* failed (e.g. perl not found).
             // execute_command returns Err(String) if `perl` command fails to spawn?
             // No, `cmd.output()` error maps to Err. But if perl runs and exits with 1, it returns Ok(Value) with success: false.
        }
    }
}
