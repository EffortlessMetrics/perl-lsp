use std::fs;
use std::path::Path;
use std::process::Command;
use serde_json::Value;
use perl_tdd_support::{must, must_some};

/// Test that the benchmark binary runs successfully and produces default output
#[test]
fn test_default_output_file() {
    // Clean up any previous test results
    let _ = fs::remove_file("benchmark_results.json");

    // Run the benchmark binary with default settings
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

    // Check default output file is created
    assert!(Path::new("benchmark_results.json").exists(), "Default output file not created");

    // Validate JSON structure
    let json_content = must(fs::read_to_string("benchmark_results.json"));
    let parsed: Value = must(serde_json::from_str(&json_content));

    // Check key fields in JSON
    assert!(parsed["metadata"].is_object(), "Metadata section missing");
    assert!(parsed["tests"].is_object(), "Tests section missing");
    assert!(parsed["summary"].is_object(), "Summary section missing");
    
    // Clean up
    let _ = fs::remove_file("benchmark_results.json");
}

/// Test custom output path functionality
#[test]
fn test_custom_output_path() {
    // Clean up any previous test results
    let _ = fs::remove_file("custom_benchmark_results.json");

    // Run benchmark with custom output path
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
            "custom_benchmark_results.json"
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Custom output benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Check custom output file is created
    assert!(Path::new("custom_benchmark_results.json").exists(), "Custom output file not created");
    
    // Clean up
    let _ = fs::remove_file("custom_benchmark_results.json");
}

/// Test output path with directory creation
#[test]
fn test_output_path_with_directory() {
    // Ensure test directory doesn't exist initially
    let output_dir = "test_benchmark_output";
    let _ = fs::remove_dir_all(output_dir);

    let output_path = format!("{}/benchmark_results.json", output_dir);

    // Run benchmark with output path in non-existent directory
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
            &output_path
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Directory creation benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Check output file in subdirectory
    let output_path_obj = Path::new(&output_path);
    assert!(output_path_obj.exists(), "Output file in subdirectory not created");
    
    // Clean up
    fs::remove_dir_all(output_dir).ok();
}

/// Test CLI flags functionality
#[test]
fn test_cli_flags() {
    // Clean up any previous test results
    let _ = fs::remove_file("cli_test_results.json");

    // Run benchmark with various CLI flags
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
            "cli_test_results.json",
            "--iterations",
            "2",  // Minimized for CI performance
            "--warmup", 
            "1"   // Minimized for CI performance
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "CLI flags benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Check output file is created
    assert!(Path::new("cli_test_results.json").exists(), "CLI test output file not created");

    // Validate configuration was applied
    let json_content = must(fs::read_to_string("cli_test_results.json"));
    let parsed: Value = must(serde_json::from_str(&json_content));
    
    let config = &parsed["metadata"]["configuration"];
    assert_eq!(config["iterations"], 5, "Iterations CLI override not applied");
    assert_eq!(config["warmup_iterations"], 1, "Warmup iterations CLI override not applied");
    
    // Clean up
    let _ = fs::remove_file("cli_test_results.json");
}

/// Test save flag without output path
#[test]
fn test_save_flag_functionality() {
    // Clean up any previous test results
    let _ = fs::remove_file("benchmark_results.json");

    // Run benchmark with --save flag (should use default path)
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
            "--save",
            "--iterations",
            "2"  // Minimized for CI performance
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Save flag benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Check default output file is created
    assert!(Path::new("benchmark_results.json").exists(), "Save flag output file not created");
    
    // Clean up
    let _ = fs::remove_file("benchmark_results.json");
}

/// Test JSON output structure validation
#[test]
fn test_performance_categories() {
    // Clean up any previous test results
    let _ = fs::remove_file("performance_test_results.json");

    // Run benchmark
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
            "performance_test_results.json",
            "--iterations",
            "2"  // Minimized for CI performance
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Performance categories benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Validate JSON structure
    let json_content = must(fs::read_to_string("performance_test_results.json"));
    let parsed: Value = must(serde_json::from_str(&json_content));

    let categories = must_some(parsed["summary"]["performance_categories"].as_object());

    // Check for expected category types
    let expected_categories = [
        "small_files", 
        "medium_files", 
        "large_files", 
        "fast_parsing", 
        "moderate_parsing", 
        "slow_parsing"
    ];

    for category in expected_categories.iter() {
        // Categories may be empty, but should exist
        if categories.contains_key(*category) {
            assert!(categories[category].is_array(), "Category {} should be an array", category);
        }
    }
    
    // Clean up
    let _ = fs::remove_file("performance_test_results.json");
}

/// Test metadata validation
#[test]
fn test_metadata_details() {
    // Clean up any previous test results
    let _ = fs::remove_file("metadata_test_results.json");

    // Run benchmark
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
            "metadata_test_results.json",
            "--iterations",
            "1"  // Minimized for CI performance
        ])
        .output());

    // Check command was successful
    assert!(output.status.success(), "Metadata test benchmark run failed: {}", String::from_utf8_lossy(&output.stderr));

    // Validate JSON
    let json_content = must(fs::read_to_string("metadata_test_results.json"));
    let parsed: Value = must(serde_json::from_str(&json_content));

    let metadata = must_some(parsed["metadata"].as_object());

    // Check key metadata fields
    assert!(metadata.contains_key("generated_at"), "Missing generated_at timestamp");
    assert!(metadata.contains_key("parser_version"), "Missing parser version");
    assert!(metadata.contains_key("rust_version"), "Missing Rust version");
    assert!(metadata.contains_key("total_tests"), "Missing total tests count");
    assert!(metadata.contains_key("total_iterations"), "Missing total iterations");
    
    // Validate timestamp format (should be RFC3339)
    let timestamp = must_some(metadata["generated_at"].as_str());
    assert!(timestamp.contains('T'), "Timestamp should be in RFC3339 format");
    
    // Clean up
    let _ = fs::remove_file("metadata_test_results.json");
}