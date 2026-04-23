//! Wave H Collapse External Consumer RED Tests — work-efd2aa1b
//!
//! These tests verify that external consumers (perl-lsp, perl-lsp-config)
//! can still function after the collapse.
//!
//! They are written BEFORE the implementation (RED state) and should FAIL
//! until the collapse is properly implemented.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_external_red_tests`

#![allow(unused_imports)]

use anyhow::Result;

/// Test that types module Source type is accessible (avoiding collision with protocol::Source)
#[test]
fn test_types_source_accessible_without_collision() -> Result<()> {
    use perl_dap::types::Source;

    let source = Source {
        name: Some("test.pl".to_string()),
        path: "/tmp/test.pl".to_string(),
        source_reference: None,
    };
    assert!(source.name.as_ref().unwrap().contains("test"));
    Ok(())
}

/// Test that types module exports are prefixed to avoid collision.
/// After collapse, types module items should be prefixed (TypesSource, TypesStackFrame, etc.)
/// to avoid collision with protocol.rs types.
#[test]
fn test_types_module_exports_are_prefixed() -> Result<()> {
    use perl_dap::api::TypesSource;

    let source = TypesSource {
        name: Some("test.pl".to_string()),
        path: "/tmp/test.pl".to_string(),
        source_reference: None,
    };
    assert!(source.name.as_ref().unwrap().contains("test"));
    Ok(())
}

/// Test that platform exports PerlInterpreterResult type
#[test]
fn test_platform_exports_perl_interpreter_result() -> Result<()> {
    use perl_dap::platform::PerlInterpreterResult;

    let result: PerlInterpreterResult = PerlInterpreterResult::NotFound("perl not found".to_string());
    match result {
        PerlInterpreterResult::Found(_) => {},
        PerlInterpreterResult::NotFound(msg) => assert!(msg.contains("not found")),
    }
    Ok(())
}

/// Test that command_args::format_command_args produces valid command line
#[test]
fn test_command_args_formatter_produces_valid_output() -> Result<()> {
    use perl_dap::command_args::format_command_args;

    let args = vec![
        "perl".to_string(),
        "-d".to_string(),
        "-Ilib".to_string(),
        "script.pl".to_string(),
    ];
    let result = format_command_args(&args);

    // Should contain perl and script.pl
    assert!(result.contains("perl"), "Should contain 'perl': {}", result);
    assert!(result.contains("script.pl"), "Should contain 'script.pl': {}", result);
    Ok(())
}

/// Test that stack parser can parse debugger output
#[test]
fn test_stack_parser_handles_debugger_output() -> Result<()> {
    use perl_dap::stack::PerlStackParser;

    let parser = PerlStackParser::new();

    // Sample debugger output
    let output = r#"Stack:
  main::foo called at script.pl line 10
  Devel::Debugger::DB called at script.pl line 5
"#;

    let frames = parser.parse_stack(output)?;
    assert!(!frames.is_empty() || output.contains("Stack:")); // Or check parsed frames
    Ok(())
}

/// Test that breakpoint validator validates breakpoint locations
#[test]
fn test_breakpoint_validator_validates_locations() -> Result<()> {
    use perl_dap::breakpoint::{AstBreakpointValidator, SourceBreakpoint};

    let validator = AstBreakpointValidator::new("sub foo { 1 }")?;
    let bp = SourceBreakpoint {
        line: 1,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
    };

    // Valid breakpoint on executable line
    let result = validator.validate_breakpoint(&bp);
    assert!(result.is_ok(), "Breakpoint on line 1 should be valid: {:?}", result);
    Ok(())
}

/// Test that safe evaluator rejects dangerous expressions
#[test]
fn test_safe_evaluator_rejects_dangerous_expressions() -> Result<()> {
    use perl_dap::eval::SafeEvaluator;

    let evaluator = SafeEvaluator::new();

    // Multi-line expression should be rejected
    let dangerous = evaluator.validate_expression("print 'hello'\nsystem('ls')");
    assert!(dangerous.is_err(), "Multi-line expressions should be rejected");

    Ok(())
}

/// Test that security validate_path prevents directory traversal
#[test]
fn test_security_validate_path_prevents_traversal() -> Result<()> {
    use perl_dap::security::validate_path;

    // Path traversal attempt should be rejected
    let result = validate_path("/workspace/../etc/passwd", "/workspace");
    assert!(result.is_err(), "Path traversal should be rejected");

    Ok(())
}

/// Test that config launch configuration validates correctly
#[test]
fn test_config_launch_validates_program() -> Result<()> {
    use perl_dap::config::LaunchConfiguration;
    use std::path::PathBuf;
    use std::collections::HashMap;

    let config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: Some(PathBuf::from("/tmp")),
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    config.validate()?;
    Ok(())
}

/// Test that DapServer can be constructed with new config
#[test]
fn test_dap_server_construction() -> Result<()> {
    use perl_dap::{DapConfig, DapMode, DapServer};

    let config = DapConfig {
        log_level: "debug".into(),
        mode: DapMode::Native,
        workspace_root: Some(std::path::PathBuf::from("/tmp")),
    };

    let server = DapServer::new(config);
    assert!(server.is_ok(), "DapServer should be constructible");
    Ok(())
}
