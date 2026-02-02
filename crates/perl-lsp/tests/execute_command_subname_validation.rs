use perl_lsp::execute_command::ExecuteCommandProvider;
use serde_json::Value;
use std::error::Error;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_run_test_sub_invalid_names() -> Result<(), Box<dyn Error>> {
    let provider = ExecuteCommandProvider::new();
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_sub_validation.pl");
    fs::write(&file_path, "sub safe_sub { print 'SAFE'; }")?;

    let file_arg = Value::String(file_path.to_string_lossy().to_string());

    // List of invalid subroutine names that should be rejected
    let invalid_names = vec![
        "system('calc')",       // Code injection attempt (though blocked by @ARGV, validation should reject it)
        "CORE::system",         // Built-in
        "CORE::GLOBAL::system", // Global override
        "Foo'Bar",              // Old package separator (we enforce ::)
        "123sub",               // Starts with digit
        "-flag",                // Starts with hyphen
        "sub; rm -rf /",        // Shell injection attempt chars
        "",                     // Empty
        "   ",                  // Whitespace
        "Foo::Bar::",           // Trailing ::
        "::Foo",                // Leading ::
    ];

    for name in invalid_names {
        let result = provider.execute_command(
            "perl.runTestSub",
            vec![file_arg.clone(), Value::String(name.to_string())],
        );

        // Expect an error about invalid subroutine name
        assert!(result.is_err(), "Should have blocked invalid subroutine name: '{}'", name);

        let error = result.unwrap_err();
        assert!(
            error.contains("Invalid subroutine name") ||
            error.contains("Subroutine name cannot be empty") ||
            error.contains("CORE:: subroutines are not allowed"),
            "Unexpected error message for '{}': {}", name, error
        );
    }

    Ok(())
}
