use std::fs;
use std::path::Path;
use std::process::Command;
use serde_json::Value;
use perl_tdd_support::{must, must_some};

/// Integration test for benchmark output functionality
/// Tests the actual binary execution and file output
#[test]
fn integration_test_benchmark_output() {
    // Clean up any previous test results
    let _ = fs::remove_file("benchmark_results.json");

    // Use cargo to run the benchmark binary with default settings
    let output = must(Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "tree-sitter-perl", 
            "--bin",
            "benchmark_parsers",
            "--features",
            "pure-rust"
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify output file exists
    assert!(Path::new("benchmark_results.json").exists(), "Default output file not created");

    // Validate JSON content
    let json_content = must(fs::read_to_string("benchmark_results.json"));
    let parsed: Value = must(serde_json::from_str(&json_content));

    // Validate key sections
    assert!(parsed["metadata"].is_object(), "Missing metadata section");
    assert!(parsed["tests"].is_object(), "Missing tests section");
    assert!(parsed["summary"].is_object(), "Missing summary section");

    // Check metadata details
    let metadata = must_some(parsed["metadata"].as_object());
    assert!(metadata.contains_key("generated_at"), "Missing generation timestamp");
    assert!(metadata.contains_key("parser_version"), "Missing parser version");
    assert!(metadata.contains_key("rust_version"), "Missing Rust version");
    assert!(metadata.contains_key("total_tests"), "Missing total tests count");
    assert!(metadata.contains_key("total_iterations"), "Missing total iterations");

    // Basic performance categories test
    let summary = must_some(parsed["summary"].as_object());
    let perf_categories = must_some(summary.get("performance_categories")
        .and_then(|v| v.as_object()));

    let expected_categories = [
        "small_files", 
        "medium_files", 
        "large_files", 
        "fast_parsing", 
        "moderate_parsing", 
        "slow_parsing"
    ];

    for category in expected_categories.iter() {
        // Categories may be empty, but if they exist, they should be arrays
        if perf_categories.contains_key(*category) {
            assert!(perf_categories[category].is_array(), 
                "Performance category {} should be an array", category);
        }
    }
    
    // Clean up
    let _ = fs::remove_file("benchmark_results.json");
}

/// Test custom output path with directory creation
#[test]
fn test_custom_output_path() {
    let test_output_path = "test_benchmark_output/custom_results.json";
    
    // Ensure test directory doesn't exist initially
    let _ = fs::remove_dir_all("test_benchmark_output");

    // Run benchmark with custom output
    let output = must(Command::new("cargo")
        .args(&[
            "run", 
            "-p",
            "tree-sitter-perl",
            "--bin", 
            "benchmark_parsers",
            "--features",
            "pure-rust",
            "--", 
            "--output", 
            test_output_path,
            "--iterations",
            "1"  // Minimized for CI performance
        ])
        .output());

    assert!(output.status.success(), "Benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(Path::new(test_output_path).exists(), "Custom output file not created");

    // Validate the content structure
    let json_content = must(fs::read_to_string(test_output_path));
    let parsed: Value = must(serde_json::from_str(&json_content));
    
    // Basic structure validation
    assert!(parsed["metadata"].is_object(), "Metadata section missing in custom output");
    assert!(parsed["tests"].is_object(), "Tests section missing in custom output");
    assert!(parsed["summary"].is_object(), "Summary section missing in custom output");

    // Clean up
    fs::remove_file(test_output_path).ok();
    fs::remove_dir("test_benchmark_output").ok();
}

/// Test that directory creation works correctly
#[test]
fn test_output_directory_creation() {
    let test_output_path = "new_benchmark_output_dir/results.json";
    
    // Ensure directory does not exist
    let _ = fs::remove_dir_all("new_benchmark_output_dir");

    // Run benchmark with path in non-existent directory
    let output = must(Command::new("cargo")
        .args(&[
            "run", 
            "-p",
            "tree-sitter-perl",
            "--bin", 
            "benchmark_parsers",
            "--features",
            "pure-rust", 
            "--", 
            "--output", 
            test_output_path,
            "--iterations",
            "1"  // Minimized for CI performance
        ])
        .output());

    assert!(output.status.success(), "Benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(Path::new(test_output_path).exists(), "Output file not created in new directory");

    // Clean up
    fs::remove_file(test_output_path).ok();
    fs::remove_dir("new_benchmark_output_dir").ok();
}

/// Test error handling for invalid configurations
#[test]
fn test_error_handling() {
    // Test with zero iterations (should fail)
    let output = must(Command::new("cargo")
        .args(&[
            "run", 
            "-p",
            "tree-sitter-perl",
            "--bin", 
            "benchmark_parsers",
            "--features",
            "pure-rust",
            "--", 
            "--iterations",
            "0"  // Invalid: zero iterations
        ])
        .output());

    // Command should fail with zero iterations
    assert!(!output.status.success(), "Benchmark should have failed with zero iterations");
}

/// Test CLI help output
#[test] 
fn test_help_output() {
    let output = must(Command::new("cargo")
        .args(&[
            "run", 
            "-p",
            "tree-sitter-perl",
            "--bin", 
            "benchmark_parsers",
            "--features",
            "pure-rust",
            "--", 
            "--help"
        ])
        .output());

    // Help should succeed
    assert!(output.status.success(), "Help command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check for key help content
    assert!(stdout.contains("benchmark_parsers"), "Help should contain program name");
    assert!(stdout.contains("--output"), "Help should contain --output option");
    assert!(stdout.contains("--save"), "Help should contain --save option");
    assert!(stdout.contains("--iterations"), "Help should contain --iterations option");
}

/// Test version output  
#[test]
fn test_version_output() {
    let output = must(Command::new("cargo")
        .args(&[
            "run", 
            "-p",
            "tree-sitter-perl",
            "--bin", 
            "benchmark_parsers",
            "--features",
            "pure-rust",
            "--", 
            "--version"
        ])
        .output());

    // Version should succeed
    assert!(output.status.success(), "Version command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should contain version information
    assert!(stdout.contains("benchmark_parsers"), "Version output should contain program name");
}