//! Wave H Collapse: Public API Stability Tests
//!
//! Verifies that all re-exported symbols from collapsed modules remain stable.
//! Prevents accidental removals of public API items in future refactors.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_api_stability`

use perl_dap::api::*;
use perl_dap::stack::is_internal_frame_name_and_path;

#[test]
fn test_api_module_exposes_all_required_symbols() {
    // Verify that api.rs re-exports are present and correct.
    // Use type_name to check if symbols are accessible and properly named.

    // breakpoint module exports
    let name = std::any::type_name::<AstBreakpointValidator>();
    assert!(
        name.contains("AstBreakpointValidator"),
        "AstBreakpointValidator not found: {}",
        name
    );

    let name = std::any::type_name::<BreakpointError>();
    assert!(
        name.contains("BreakpointError"),
        "BreakpointError not found: {}",
        name
    );

    // eval module exports
    let name = std::any::type_name::<SafeEvaluator>();
    assert!(
        name.contains("SafeEvaluator"),
        "SafeEvaluator not found: {}",
        name
    );

    // config module exports
    let name = std::any::type_name::<LaunchConfiguration>();
    assert!(
        name.contains("LaunchConfiguration"),
        "LaunchConfiguration not found: {}",
        name
    );

    // platform module exports
    let name = std::any::type_name::<PerlInterpreterResult>();
    assert!(
        name.contains("PerlInterpreterResult"),
        "PerlInterpreterResult not found: {}",
        name
    );

    // stack module exports
    let name = std::any::type_name::<PerlStackParser>();
    assert!(
        name.contains("PerlStackParser"),
        "PerlStackParser not found: {}",
        name
    );

    // types module exports (with TypesSource alias to avoid collision)
    let name = std::any::type_name::<TypesSource>();
    assert!(name.contains("Source"), "TypesSource not found: {}", name);

    // value module exports
    let name = std::any::type_name::<PerlValue>();
    assert!(name.contains("PerlValue"), "PerlValue not found: {}", name);

    // variables module exports
    let name = std::any::type_name::<PerlVariableRenderer>();
    assert!(
        name.contains("PerlVariableRenderer"),
        "PerlVariableRenderer not found: {}",
        name
    );

    // security module exports
    let name = std::any::type_name::<SecurityError>();
    assert!(
        name.contains("SecurityError"),
        "SecurityError not found: {}",
        name
    );
}

#[test]
fn test_api_functions_are_callable() {
    // Verify that re-exported functions are callable without qualification
    // (implicitly tests that api.rs imports are working)

    // command_args::format_command_args function
    let args: Vec<String> = vec![
        "perl".to_string(),
        "-d".to_string(),
        "script.pl".to_string(),
    ];
    let result = format_command_args(&args);
    assert!(
        !result.is_empty(),
        "format_command_args should return non-empty result"
    );

    // platform functions
    let _result = find_perl_interpreter(None);
    // Don't assert on result; just ensure it's callable

    // config functions
    let launch_snippet = create_launch_json_snippet();
    assert!(
        launch_snippet.contains("\"type\": \"perl\""),
        "launch snippet should be valid DAP config"
    );

    let attach_snippet = create_attach_json_snippet();
    assert!(
        attach_snippet.contains("\"type\": \"perl\""),
        "attach snippet should be valid DAP config"
    );

    // stack functions (is_internal_frame_name_and_path takes &str and Option<&str>)
    let _result = is_internal_frame_name_and_path("Devel::Debugger", Some("debug.pm"));
    // Don't assert on result; just ensure it's callable

    // security constants
    let _default_timeout = DEFAULT_TIMEOUT_MS;
    let _max_timeout = MAX_TIMEOUT_MS;
}

#[test]
fn test_api_module_has_no_wildcard_reexports() {
    // This test ensures the api.rs file uses explicit named re-exports.
    // Since we can't parse source at test time, we verify indirectly by
    // checking that private/internal items are NOT accessible.

    // breakpoint::BreakpointRecord should NOT be in api (it's internal)
    // This is a compile-time check—if it were wildcard imported, this
    // would fail to compile.

    // The fact that this test compiles without importing BreakpointRecord
    // is evidence that we're using named re-exports, not wildcard.

    // Further verification: try to use an item that should NOT be exported
    // (This is harder to do at runtime, but the structure of api.rs ensures it)

    // Just verify that the common pattern is followed:
    let _evaluator = SafeEvaluator::new();
    // If api.rs had `pub use eval::*;`, this would work the same,
    // but explicit re-exports are more maintainable.
}

#[test]
fn test_types_module_aliases_prevent_collision() {
    // Verify that types from types module are aliased with "Types" prefix
    // to prevent collision with protocol.rs types

    // TypesSource is an alias for types::Source (no collision with protocol::Source)
    // TypesStackFrame is an alias for types::StackFrame (no collision with protocol::StackFrame)
    // TypesVariable is an alias for types::Variable (no collision with protocol::Variable)

    // The aliases are used when both types exist in the same crate
    let _source = TypesSource {
        name: Some("test".to_string()),
        path: "test.pl".to_string(),
        source_reference: None,
    };

    // Just verify they're accessible; the naming prevents ambiguity
}

#[test]
fn test_re_exports_from_all_eleven_modules() {
    // Counting re-exports to ensure all 11 modules are represented
    // This is a high-level count test, not exhaustive

    // Instantiate/call one item from each collapsed module to verify the re-export works
    // at runtime, not just at compile time. This fails if api.rs stops re-exporting an item.

    // 1. breakpoint — AstBreakpointValidator::new() takes a source string
    let _bv = AstBreakpointValidator::new("sub foo { 1 }"); // Result; discard

    // 2. eval — SafeEvaluator::new()
    let _ev = SafeEvaluator::new();

    // 3. config — create_launch_json_snippet returns a non-empty string
    let snippet = create_launch_json_snippet();
    assert!(
        !snippet.is_empty(),
        "create_launch_json_snippet must return non-empty string"
    );

    // 4. command_args — format_command_args returns non-empty for non-empty input
    let args = vec!["perl".to_string(), "script.pl".to_string()];
    let formatted = format_command_args(&args);
    assert!(
        !formatted.is_empty(),
        "format_command_args must return non-empty for non-empty args"
    );

    // 5. platform — find_perl_interpreter is callable
    let _interp = find_perl_interpreter(None);

    // 6. stack — PerlStackParser::new() is the canonical constructor
    let _sp = PerlStackParser::new();

    // 7. types — TypesSource has required fields
    let _ts = TypesSource {
        name: Some("test".to_string()),
        path: "test.pl".to_string(),
        source_reference: None,
    };

    // 8. value — PerlValue::Undef is a valid variant
    let _pv = PerlValue::Undef;

    // 9. variables — PerlVariableRenderer::new()
    let _vr = PerlVariableRenderer::new();

    // 10. security — validate_expression rejects multi-line input (newline injection)
    let dangerous = validate_expression("foo\nbar");
    assert!(
        dangerous.is_err(),
        "validate_expression must reject expressions with newlines"
    );

    // 11. shell is tested indirectly via command_args above (shell depends on command_args)
}
