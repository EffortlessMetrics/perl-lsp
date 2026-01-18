use perl_lsp::execute_command::ExecuteCommandProvider;
use serde_json::Value;

#[test]
fn test_security_repro_run_test_sub_injection() {
    let provider = ExecuteCommandProvider::new();

    // Payload: "'; print 'INJECTION_SUCCESS'; '"
    // Resulting command: perl -e "do ''; print 'INJECTION_SUCCESS'; ''; if ..."
    let payload = "'; print 'INJECTION_SUCCESS'; '";

    let args = vec![
        Value::String(payload.to_string()),
        Value::String("dummy_sub".to_string()),
    ];

    let result = provider.execute_command("perl.runTestSub", args).unwrap();
    let output = result["output"].as_str().unwrap();

    // If injection works, we should see INJECTION_SUCCESS in output
    assert!(!output.contains("INJECTION_SUCCESS"), "Vulnerability confirmed: Command injection allowed arbitrary code execution!");
}

#[test]
fn test_security_repro_run_tests_argument_injection() {
    let provider = ExecuteCommandProvider::new();

    // If we pass "-v", run_tests might treat it as a flag if not handled correctly.
    // In `run_tests`, it does: Command::new("perl").arg(file_path)
    // If file_path is "-v", it becomes `perl -v`.

    let args = vec![Value::String("-v".to_string())];
    let result = provider.execute_command("perl.runTests", args).unwrap();

    let output = result["output"].as_str().unwrap();

    // If it ran `perl -v`, output contains "This is perl".
    // If it tried to run file named "-v", it would fail (file not found).

    assert!(!output.contains("This is perl"), "Vulnerability confirmed: Argument injection allowed flag execution!");
}
