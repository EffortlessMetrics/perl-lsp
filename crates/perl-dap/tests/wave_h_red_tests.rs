//! Wave H Collapse RED Tests — work-efd2aa1b
//!
//! These tests define what correct behavior looks like AFTER the collapse
//! of 11 perl-dap-* satellite crates into perl-dap modules.
//!
//! They are written BEFORE the implementation (RED state) and should FAIL
//! until the collapse is properly implemented.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_red_tests`

#![allow(unused_imports)]

use anyhow::Result;

/// Test that all 11 satellite modules are accessible via perl_dap::* after collapse.
/// These imports will fail to compile until the collapse is implemented.
#[test]
fn test_all_modules_accessible_from_perl_dap_root() -> Result<()> {
    // These modules should exist after the collapse:
    // breakpoint, command_args, config, eval, platform, security, shell, stack, types, value, variables
    use perl_dap::breakpoint;
    use perl_dap::command_args;
    use perl_dap::config;
    use perl_dap::eval;
    use perl_dap::platform;
    use perl_dap::security;
    use perl_dap::shell;
    use perl_dap::stack;
    use perl_dap::types;
    use perl_dap::value;
    use perl_dap::variables;

    // If we reach here, all 11 modules exist in lib.rs
    Ok(())
}

/// Test that breakpoint module exports AstBreakpointValidator
#[test]
fn test_breakpoint_module_exports_validator() -> Result<()> {
    use perl_dap::breakpoint::AstBreakpointValidator;

    // Should be constructible with source code
    let validator = AstBreakpointValidator::new("sub foo { 1 }");
    assert!(validator.is_ok(), "AstBreakpointValidator should be constructible");
    Ok(())
}

/// Test that eval module exports SafeEvaluator
#[test]
fn test_eval_module_exports_safe_evaluator() -> Result<()> {
    use perl_dap::eval::SafeEvaluator;

    let evaluator = SafeEvaluator::new();
    // Should have validate_expression method
    let result = evaluator.validate_expression("1 + 1");
    assert!(result.is_ok(), "SafeEvaluator should validate simple expressions");
    Ok(())
}

/// Test that config module exports LaunchConfiguration
#[test]
fn test_config_module_exports_launch_configuration() -> Result<()> {
    use perl_dap::config::LaunchConfiguration;
    use std::collections::HashMap;
    use std::path::PathBuf;

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

/// Test that command_args module exports format_command_args
#[test]
fn test_command_args_module_exports_formatter() -> Result<()> {
    use perl_dap::command_args::format_command_args;

    let args = vec!["perl".to_string(), "-d".to_string(), "script.pl".to_string()];
    let result = format_command_args(&args);
    assert!(!result.is_empty(), "format_command_args should return non-empty result");
    Ok(())
}

/// Test that platform module exports find_perl_interpreter
#[test]
fn test_platform_module_exports_perl_finder() -> Result<()> {
    use perl_dap::platform::find_perl_interpreter;

    let result = find_perl_interpreter(None);
    // Result should be Result<PerlInterpreter, FindPerlError>
    assert!(result.is_ok() || result.is_err(), "find_perl_interpreter should return a Result");
    Ok(())
}

/// Test that security module exports validate_expression
#[test]
fn test_security_module_exports_validate_expression() -> Result<()> {
    use perl_dap::security::validate_expression;

    // Safe expression should pass
    let result = validate_expression("1 + 1");
    assert!(result.is_ok(), "validate_expression should accept safe expressions");

    // Dangerous expression (with newline) should fail
    let dangerous = validate_expression("system('ls')\n");
    assert!(dangerous.is_err(), "validate_expression should reject dangerous expressions");
    Ok(())
}

/// Test that shell module is accessible
#[test]
fn test_shell_module_accessible() -> Result<()> {
    use perl_dap::shell;

    // Shell module should be accessible (specific exports depend on implementation)
    let _ = std::any::type_name::<shell::Shell>();
    Ok(())
}

/// Test that stack module exports PerlStackParser
#[test]
fn test_stack_module_exports_parser() -> Result<()> {
    use perl_dap::stack::PerlStackParser;

    let parser = PerlStackParser::new();
    assert!(!std::any::type_name::<PerlStackParser>().is_empty());
    Ok(())
}

/// Test that types module exports Source (aliased to avoid collision)
#[test]
fn test_types_module_exports_source() -> Result<()> {
    use perl_dap::types::Source;

    let source = Source {
        name: Some("test.pl".to_string()),
        path: "test.pl".to_string(),
        source_reference: None,
    };
    assert_eq!(source.path, "test.pl");
    Ok(())
}

/// Test that value module exports PerlValue
#[test]
fn test_value_module_exports_perl_value() -> Result<()> {
    use perl_dap::value::PerlValue;

    let value = PerlValue::Undef;
    assert!(matches!(value, PerlValue::Undef));
    Ok(())
}

/// Test that variables module exports PerlVariableRenderer
#[test]
fn test_variables_module_exports_renderer() -> Result<()> {
    use perl_dap::variables::PerlVariableRenderer;

    let renderer = PerlVariableRenderer::new();
    assert!(!std::any::type_name::<PerlVariableRenderer>().is_empty());
    Ok(())
}

/// Test that api module re-exports all collapsed module types
#[test]
fn test_api_module_reexports_all_types() -> Result<()> {
    use perl_dap::api::AstBreakpointValidator;
    use perl_dap::api::BreakpointError;
    use perl_dap::api::LaunchConfiguration;
    use perl_dap::api::PerlInterpreterResult;
    use perl_dap::api::PerlStackParser;
    use perl_dap::api::PerlValue;
    use perl_dap::api::PerlVariableRenderer;
    use perl_dap::api::SafeEvaluator;
    use perl_dap::api::SecurityError;
    use perl_dap::api::TypesSource;

    // Verify all types are accessible through api module
    assert!(!std::any::type_name::<AstBreakpointValidator>().is_empty());
    assert!(!std::any::type_name::<SafeEvaluator>().is_empty());
    assert!(!std::any::type_name::<LaunchConfiguration>().is_empty());
    assert!(!std::any::type_name::<PerlInterpreterResult>().is_empty());
    assert!(!std::any::type_name::<PerlStackParser>().is_empty());
    assert!(!std::any::type_name::<TypesSource>().is_empty());
    assert!(!std::any::type_name::<PerlValue>().is_empty());
    assert!(!std::any::type_name::<PerlVariableRenderer>().is_empty());
    assert!(!std::any::type_name::<SecurityError>().is_empty());
    assert!(!std::any::type_name::<BreakpointError>().is_empty());
    Ok(())
}

/// Test that api module re-exports functions from all collapsed modules
#[test]
fn test_api_module_reexports_functions() -> Result<()> {
    use perl_dap::api::DEFAULT_TIMEOUT_MS;
    use perl_dap::api::MAX_TIMEOUT_MS;
    use perl_dap::api::create_attach_json_snippet;
    use perl_dap::api::create_launch_json_snippet;
    use perl_dap::api::find_perl_interpreter;
    use perl_dap::api::format_command_args;
    use perl_dap::api::is_internal_frame_name_and_path;
    use perl_dap::api::validate_expression;

    // Verify functions are callable
    let args = vec!["perl".to_string()];
    let _ = format_command_args(&args);

    let _ = find_perl_interpreter(None);

    let snippet = create_launch_json_snippet();
    assert!(snippet.contains("perl"), "launch snippet should reference perl");

    let attach = create_attach_json_snippet();
    assert!(attach.contains("perl"), "attach snippet should reference perl");

    let _ = validate_expression("1 + 1");

    let _ = is_internal_frame_name_and_path("main", Some("script.pl"));

    assert!(DEFAULT_TIMEOUT_MS > 0);
    assert!(MAX_TIMEOUT_MS > DEFAULT_TIMEOUT_MS);
    Ok(())
}
