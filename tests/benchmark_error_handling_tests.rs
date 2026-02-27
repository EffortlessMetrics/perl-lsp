use std::fs;
use std::path::Path;
use std::process::Command;
use std::os::unix::fs::PermissionsExt;
use perl_tdd_support::{must, must_some};

/// Test error handling for various edge cases and error scenarios
mod error_handling {
    use super::*;

    /// Test that invalid iterations parameter is handled correctly
    #[test]
    fn test_invalid_iterations() {
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
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("iterations must be greater than 0") || stderr.contains("error"), 
                "Error message should indicate invalid iterations: {}", stderr);
    }

    /// Test permission denied scenario (Unix only)
    #[test]
    #[cfg(unix)]
    fn test_permission_denied() {
        // Create a directory with restricted permissions
        let restricted_dir = "restricted_test_dir";
        let _ = fs::remove_dir_all(restricted_dir);
        must(fs::create_dir_all(restricted_dir));
        
        // Set directory to read-only
        let mut perms = must(fs::metadata(restricted_dir)).permissions();
        perms.set_mode(0o444); // Read-only
        must(fs::set_permissions(restricted_dir, perms));

        let output_path = format!("{}/benchmark_results.json", restricted_dir);
        
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
                &output_path,
                "--iterations",
                "1"
            ])
            .output());

        // Should fail due to permission denied
        assert!(!output.status.success(), "Benchmark should have failed due to permission denied");
        
        // Clean up (restore permissions first)
        let mut perms = must(fs::metadata(restricted_dir)).permissions();
        perms.set_mode(0o755);
        fs::set_permissions(restricted_dir, perms).ok();
        let _ = fs::remove_dir_all(restricted_dir);
    }

    /// Test invalid JSON config file handling
    #[test]
    fn test_invalid_config_file() {
        let config_file = "invalid_benchmark_config.json";
        
        // Create invalid JSON config file
        must(fs::write(config_file, "{ invalid json content }"));
        
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
                "--config",
                config_file,
                "--iterations",
                "1"
            ])
            .output());

        // Should fail due to invalid JSON
        assert!(!output.status.success(), "Benchmark should have failed due to invalid JSON config");
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Invalid JSON") || stderr.contains("error"), 
                "Error message should indicate invalid JSON: {}", stderr);
        
        // Clean up
        let _ = fs::remove_file(config_file);
    }

    /// Test nonexistent config file handling
    #[test]
    fn test_nonexistent_config_file() {
        let config_file = "nonexistent_benchmark_config.json";
        
        // Ensure file doesn't exist
        let _ = fs::remove_file(config_file);
        
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
                "--config",
                config_file,
                "--iterations",
                "1"
            ])
            .output());

        // Should fail due to nonexistent config file
        assert!(!output.status.success(), "Benchmark should have failed due to nonexistent config file");
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Failed to read config file") || stderr.contains("No such file"), 
                "Error message should indicate missing config file: {}", stderr);
    }

    /// Test that very long output paths are handled correctly
    #[test]
    fn test_very_long_output_path() {
        // Create a very long path name (but still within reasonable limits)
        let long_dir = "a".repeat(100);
        let long_filename = "b".repeat(100);
        let long_path = format!("{}/{}.json", long_dir, long_filename);
        
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
                &long_path,
                "--iterations",
                "1"
            ])
            .output());

        // Should either succeed or fail gracefully
        if output.status.success() {
            // If successful, the file should exist
            assert!(Path::new(&long_path).exists(), "Long path output file should exist");
            // Clean up
            let _ = fs::remove_file(&long_path);
            let _ = fs::remove_dir(&long_dir);
        } else {
            // If failed, should have an appropriate error message
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.is_empty(), "Should have error message for long path");
        }
    }

    /// Test edge case with empty test files list (though config validation should catch this)
    #[test]
    fn test_empty_test_files() {
        let config_file = "empty_test_files_config.json";
        
        // Create config with empty test files
        let config_content = r#"
        {
            "iterations": 1,
            "warmup_iterations": 0,
            "test_files": [],
            "output_path": "empty_test_results.json",
            "detailed_stats": true,
            "memory_tracking": false,
            "save_results": true
        }
        "#;
        
        must(fs::write(config_file, config_content));
        
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
                "--config",
                config_file
            ])
            .output());

        // Should fail due to empty test files list
        assert!(!output.status.success(), "Benchmark should have failed due to empty test files list");
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("test_files cannot be empty") || stderr.contains("No test files found"), 
                "Error message should indicate empty test files: {}", stderr);
        
        // Clean up
        let _ = fs::remove_file(config_file);
        let _ = fs::remove_file("empty_test_results.json");
    }

    /// Test argument parsing with invalid numbers
    #[test]
    fn test_invalid_number_arguments() {
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
                "not_a_number"  // Invalid number
            ])
            .output());

        // Should fail due to invalid number
        assert!(!output.status.success(), "Benchmark should have failed due to invalid number argument");
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error") || stderr.contains("invalid"), 
                "Error message should indicate invalid number: {}", stderr);
    }

    /// Test handling of very small iterations
    #[test]
    fn test_minimal_valid_configuration() {
        // Clean up any previous test results
        let _ = fs::remove_file("minimal_test_results.json");

        // Run with minimal valid configuration
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
                "minimal_test_results.json",
                "--iterations",
                "1",  // Minimal valid iterations
                "--warmup",
                "0"   // No warmup
            ])
            .output());

        // Should succeed with minimal valid configuration
        assert!(output.status.success(), "Minimal configuration should succeed: {}", String::from_utf8_lossy(&output.stderr));
        
        // Verify output file exists and is valid JSON
        assert!(Path::new("minimal_test_results.json").exists(), "Minimal test output file should exist");
        
        let json_content = must(fs::read_to_string("minimal_test_results.json"));
        let _: serde_json::Value = must(serde_json::from_str(&json_content));
        
        // Clean up
        let _ = fs::remove_file("minimal_test_results.json");
    }
}