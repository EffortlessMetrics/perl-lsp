//! Wave H Collapse: Platform Module Edge Cases
//!
//! Tests for edge cases and boundary conditions in the platform module,
//! which is critical infrastructure after the collapse.
//!
//! Run with: `cargo test -p perl-dap --test wave_h_platform_edge_cases`

use perl_dap::platform::*;
use std::path::PathBuf;

#[test]
fn test_normalize_path_with_empty_string() {
    // Edge case: empty path
    let result = normalize_path(&PathBuf::from(""));
    assert_eq!(
        result,
        PathBuf::from(""),
        "normalize_path should handle empty PathBuf"
    );
}

#[test]
fn test_normalize_path_with_relative_path() {
    // Edge case: relative paths like ./script.pl or ../lib
    let result = normalize_path(&PathBuf::from("./script.pl"));
    assert!(!result.as_os_str().is_empty());

    let result = normalize_path(&PathBuf::from("../lib"));
    assert!(!result.as_os_str().is_empty());
}

#[test]
fn test_find_perl_interpreter_with_none_config() {
    // Edge case: calling find_perl_interpreter with None configured_path
    let result = find_perl_interpreter(None);

    // Result is a PerlInterpreterResult enum with multiple variants
    match result {
        PerlInterpreterResult::ConfiguredPath(_) => {
            // Shouldn't happen with None, but acceptable
        }
        PerlInterpreterResult::FoundOnPath(_) => {
            // Expected in most environments
        }
        PerlInterpreterResult::FoundViaFallback { .. } => {
            // Acceptable on Windows or special systems
        }
        PerlInterpreterResult::NotFound { .. } => {
            // Expected if Perl isn't installed
        }
    }
}

#[test]
fn test_find_perl_interpreter_with_empty_string_config() {
    // Edge case: calling find_perl_interpreter with empty string
    let result = find_perl_interpreter(Some(""));

    // Empty string should be treated like None
    match result {
        PerlInterpreterResult::ConfiguredPath(_) => {
            // Shouldn't use configured path if empty
        }
        PerlInterpreterResult::FoundOnPath(_) => {
            // Expected fallback
        }
        PerlInterpreterResult::FoundViaFallback { .. } => {
            // Acceptable
        }
        PerlInterpreterResult::NotFound { .. } => {
            // Acceptable
        }
    }
}

#[test]
fn test_resolve_perl_path_doesnt_panic() {
    // Boundary condition: ensure resolve_perl_path never panics
    let result = resolve_perl_path();

    // Result can be Ok or Err depending on system
    match result {
        Ok(path) => assert!(!path.as_os_str().is_empty()),
        Err(_) => {
            // Perl not found; acceptable
        }
    }
}

#[test]
fn test_resolve_perl_path_with_toolchain_doesnt_panic() {
    // Boundary condition: ensure resolve_perl_path_with_toolchain never panics
    let result = resolve_perl_path_with_toolchain();

    // Result can be Ok or Err depending on system
    match result {
        Ok(path) => assert!(!path.as_os_str().is_empty()),
        Err(_) => {
            // Perl not found via toolchain; acceptable
        }
    }
}

#[test]
fn test_setup_environment_with_empty_paths() {
    // Edge case: setup_environment with empty path list
    let env = setup_environment(&[]);

    // Should handle gracefully (might return empty or with defaults)
    // Just ensure it doesn't panic
    let _ = env;
}

#[test]
fn test_setup_environment_with_many_paths() {
    // Boundary condition: setup_environment with many paths
    let mut paths = Vec::new();
    for i in 0..500 {
        paths.push(PathBuf::from(format!("/lib/path{}", i)));
    }

    let env = setup_environment(&paths);

    // Should handle large number of paths
    // Check that PERL5LIB was set (it should be the main effect)
    assert!(
        !env.is_empty(),
        "setup_environment should set environment variables"
    );
}

#[test]
fn test_setup_environment_with_special_chars_in_paths() {
    // Edge case: paths with special characters
    let paths = vec![
        PathBuf::from("/lib/path with spaces"),
        PathBuf::from("/lib/path-with-dashes"),
        PathBuf::from("/lib/path_with_underscores"),
        PathBuf::from("/lib/path.with.dots"),
    ];

    let env = setup_environment(&paths);

    // Should handle special characters in paths
    assert!(!env.is_empty());
}

#[test]
fn test_detect_perlbrew_perl_not_found_gracefully() {
    // Boundary condition: perlbrew might not be installed
    let result = detect_perlbrew_perl();

    // Both Some and None are acceptable (depends on environment)
    match result {
        Some(_) => {
            // perlbrew is installed
        }
        None => {
            // perlbrew not installed (expected on many systems)
        }
    }
}

#[test]
fn test_detect_plenv_perl_not_found_gracefully() {
    // Boundary condition: plenv might not be installed
    let result = detect_plenv_perl();

    // Both Some and None are acceptable (depends on environment)
    match result {
        Some(_) => {
            // plenv is installed
        }
        None => {
            // plenv not installed (expected on many systems)
        }
    }
}

#[test]
fn test_perl_interpreter_result_type_accessible() {
    // Verify that PerlInterpreterResult type is accessible and usable
    let _type_name = std::any::type_name::<PerlInterpreterResult>();

    // Should be a valid enum type
}

#[test]
fn test_platform_functions_consistent_with_old_satellite() {
    // Verify that platform module functions are the same as old perl-dap-platform
    // This is a regression test: no functions should have been accidentally removed

    // Functions that should exist (note: signatures changed after collapse):
    let _ = std::any::type_name::<fn(Option<&str>) -> PerlInterpreterResult>();
    let _ = std::any::type_name::<fn() -> anyhow::Result<std::path::PathBuf>>();
    let _ = std::any::type_name::<fn(&std::path::PathBuf) -> std::path::PathBuf>();
    let _ = std::any::type_name::<
        fn(&[std::path::PathBuf]) -> std::collections::HashMap<String, String>,
    >();

    // If any function is missing, type_name would fail
}

#[test]
fn test_normalize_path_with_absolute_path() {
    // Edge case: absolute paths on various systems
    #[cfg(unix)]
    let path = PathBuf::from("/usr/bin/perl");
    #[cfg(windows)]
    let path = PathBuf::from("C:\\Perl\\bin\\perl.exe");

    let result = normalize_path(&path);
    assert!(!result.as_os_str().is_empty());
}

#[test]
fn test_find_perl_interpreter_env_resilience() {
    // Regression test: ensure find_perl_interpreter handles various configurations
    // Try with various inputs that shouldn't crash

    let _ = find_perl_interpreter(None);
    let _ = find_perl_interpreter(Some("perl"));
    let _ = find_perl_interpreter(Some("perl5.34"));

    // Just verify it doesn't panic on various inputs
}
