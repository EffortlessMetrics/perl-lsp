//! Execute command support for running tests and debugging
//!
//! This module provides support for the LSP executeCommand request,
//! allowing users to run tests directly from their editor.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::process::Command;
use std::path::Path;
use crate::perl_critic::{CriticAnalyzer, CriticConfig, BuiltInAnalyzer};

/// Commands supported by the Perl LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PerlCommand {
    /// Run all tests in a file
    RunTests { file_path: String },
    /// Run a specific test subroutine
    RunTestSub { file_path: String, sub_name: String },
    /// Run file with perl
    RunFile { file_path: String },
    /// Debug a test file
    DebugTests { file_path: String },
}

/// Result of executing a command
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Execute command provider
pub struct ExecuteCommandProvider;

impl Default for ExecuteCommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecuteCommandProvider {
    /// Create a new execute command provider
    pub fn new() -> Self {
        Self
    }

    /// Execute a command
    pub fn execute_command(&self, command: &str, arguments: Vec<Value>) -> Result<Value, String> {
        match command {
            "perl.runTests" => {
                if let Some(file_path) = arguments.first().and_then(|v| v.as_str()) {
                    self.run_tests(file_path)
                } else {
                    Err("Missing file path argument".to_string())
                }
            }
            "perl.runFile" => {
                if let Some(file_path) = arguments.first().and_then(|v| v.as_str()) {
                    self.run_file(file_path)
                } else {
                    Err("Missing file path argument".to_string())
                }
            }
            "perl.runTestSub" => {
                if let (Some(file_path), Some(sub_name)) = (
                    arguments.first().and_then(|v| v.as_str()),
                    arguments.get(1).and_then(|v| v.as_str()),
                ) {
                    self.run_test_sub(file_path, sub_name)
                } else {
                    Err("Missing file path or subroutine name argument".to_string())
                }
            }
            "perl.debugTests" => {
                if let Some(file_path) = arguments.first().and_then(|v| v.as_str()) {
                    self.debug_tests(file_path)
                } else {
                    Err("Missing file path argument".to_string())
                }
            }
            "perl.runCritic" => {
                if let Some(file_path) = arguments.first().and_then(|v| v.as_str()) {
                    self.run_critic(file_path)
                } else {
                    Err("Missing file path argument".to_string())
                }
            }
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    /// Run all tests in a file
    fn run_tests(&self, file_path: &str) -> Result<Value, String> {
        // Check if file looks like a test file
        let is_test_file =
            file_path.ends_with(".t") || file_path.contains("/t/") || file_path.contains("test");

        // Choose the appropriate command
        let result = if is_test_file && self.command_exists("prove") {
            // Use prove for test files if available
            Command::new("prove")
                .arg("-v")
                .arg(file_path)
                .output()
                .map_err(|e| format!("Failed to run prove: {}", e))?
        } else {
            // Fall back to perl
            Command::new("perl")
                .arg(file_path)
                .output()
                .map_err(|e| format!("Failed to run perl: {}", e))?
        };

        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        Ok(json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error,
            "command": if is_test_file && self.command_exists("prove") { "prove" } else { "perl" }
        }))
    }

    /// Run a specific test subroutine
    fn run_test_sub(&self, file_path: &str, sub_name: &str) -> Result<Value, String> {
        // For now, run the whole file with a filter if possible
        // This could be enhanced to use Test::More's test selection features
        let result = Command::new("perl")
            .arg("-e")
            .arg(format!(
                "do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}",
                file_path, sub_name, sub_name, sub_name
            ))
            .output()
            .map_err(|e| format!("Failed to run test subroutine: {}", e))?;

        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        Ok(json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error,
            "subroutine": sub_name
        }))
    }

    /// Run a Perl file
    fn run_file(&self, file_path: &str) -> Result<Value, String> {
        let result = Command::new("perl")
            .arg(file_path)
            .output()
            .map_err(|e| format!("Failed to run file: {}", e))?;

        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        Ok(json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error
        }))
    }

    /// Debug tests (placeholder for future implementation)
    fn debug_tests(&self, file_path: &str) -> Result<Value, String> {
        // For now, just run with perl -d
        // In the future, this could integrate with Perl debugger or DAP
        Ok(json!({
            "success": false,
            "output": format!("Debug mode not yet implemented for {}", file_path),
            "error": Some("Debugging support coming soon".to_string())
        }))
    }

    /// Run Perl::Critic on a file
    fn run_critic(&self, file_path: &str) -> Result<Value, String> {
        // Convert URI to file path if necessary
        let file_path = if file_path.starts_with("file://") {
            file_path.strip_prefix("file://").unwrap_or(file_path)
        } else {
            file_path
        };

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(json!({
                "status": "error",
                "error": format!("File not found: {}", file_path),
                "violations": [],
                "analyzerUsed": "none"
            }));
        }

        // Try external perlcritic first
        if self.command_exists("perlcritic") {
            match self.run_external_critic(path) {
                Ok(result) => return Ok(result),
                Err(_) => {
                    // Fall back to built-in analyzer
                }
            }
        }

        // Use built-in analyzer as fallback
        self.run_builtin_critic(path)
    }

    /// Run external perlcritic command
    fn run_external_critic(&self, file_path: &Path) -> Result<Value, String> {
        let mut config = CriticConfig::default();
        config.severity = 3; // Harsh and above
        config.verbose = true;

        let mut analyzer = CriticAnalyzer::new(config);

        match analyzer.analyze_file(file_path) {
            Ok(violations) => {
                Ok(json!({
                    "status": "success",
                    "violations": violations.iter().map(|v| json!({
                        "policy": v.policy,
                        "description": v.description,
                        "explanation": v.explanation,
                        "severity": v.severity as u8,
                        "line": v.range.start.line + 1,
                        "column": v.range.start.column + 1,
                        "file": v.file
                    })).collect::<Vec<_>>(),
                    "analyzerUsed": "external"
                }))
            }
            Err(e) => Err(format!("External perlcritic failed: {}", e))
        }
    }

    /// Run built-in critic analyzer
    fn run_builtin_critic(&self, file_path: &Path) -> Result<Value, String> {
        // Read file content for analysis
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Parse the file to get AST (simplified - would need actual parser)
        let dummy_ast = crate::ast::Node::new(
            crate::ast::NodeKind::Error { message: "dummy".to_string() },
            crate::ast::SourceLocation { start: 0, end: content.len() },
        );

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&dummy_ast, &content);

        Ok(json!({
            "status": "success",
            "violations": violations.iter().map(|v| json!({
                "policy": v.policy,
                "description": v.description,
                "explanation": v.explanation,
                "severity": v.severity as u8,
                "line": v.range.start.line + 1,
                "column": v.range.start.column + 1,
                "file": file_path.to_string_lossy()
            })).collect::<Vec<_>>(),
            "analyzerUsed": "builtin"
        }))
    }

    /// Check if a command exists in PATH
    fn command_exists(&self, command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Get the list of supported commands
pub fn get_supported_commands() -> Vec<String> {
    vec![
        "perl.runTests".to_string(),
        "perl.runFile".to_string(),
        "perl.runTestSub".to_string(),
        "perl.debugTests".to_string(),
        "perl.runCritic".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_supported_commands_includes_run_critic() {
        let commands = get_supported_commands();
        assert!(
            commands.contains(&"perl.runCritic".to_string()),
            "perl.runCritic should be in supported commands list"
        );
    }

    #[test]
    fn test_execute_command_run_critic_builtin() {
        // Create a temporary file with violations
        let test_content = r#"#!/usr/bin/perl
# Test file with policy violations
my $variable = 42;
print "Value: $variable\n";
"#;

        let temp_file = "/tmp/test_violations_unit.pl";
        fs::write(temp_file, test_content).expect("Failed to create test file");

        let provider = ExecuteCommandProvider::new();
        let result = provider.execute_command("perl.runCritic", vec![Value::String(temp_file.to_string())]);

        // Clean up
        fs::remove_file(temp_file).ok();

        // Verify result
        assert!(result.is_ok(), "perl.runCritic command should execute successfully");

        let result_value = result.unwrap();
        assert_eq!(result_value["status"], "success", "Command should succeed");
        assert!(result_value["violations"].is_array(), "Should return violations array");
        assert!(result_value["analyzerUsed"].is_string(), "Should indicate which analyzer was used");

        // Should detect missing 'use strict' and 'use warnings'
        let violations = result_value["violations"].as_array().unwrap();
        assert!(!violations.is_empty(), "Should detect policy violations");

        // Check for specific violations
        let has_strict_violation = violations.iter().any(|v| {
            v["policy"].as_str()
                .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
                .unwrap_or(false)
        });

        let has_warnings_violation = violations.iter().any(|v| {
            v["policy"].as_str()
                .map(|p| p.contains("RequireUseWarnings") || p.contains("warnings"))
                .unwrap_or(false)
        });

        assert!(has_strict_violation, "Should detect missing 'use strict'");
        assert!(has_warnings_violation, "Should detect missing 'use warnings'");
    }

    #[test]
    fn test_execute_command_invalid_command() {
        let provider = ExecuteCommandProvider::new();
        let result = provider.execute_command("perl.invalidCommand", vec![]);
        assert!(result.is_err(), "Invalid command should return error");
        assert!(result.unwrap_err().contains("Unknown command"), "Should indicate unknown command");
    }

    #[test]
    fn test_execute_command_run_critic_missing_file() {
        let provider = ExecuteCommandProvider::new();
        let result = provider.execute_command("perl.runCritic", vec![Value::String("/tmp/nonexistent.pl".to_string())]);

        assert!(result.is_ok(), "Should handle missing files gracefully");
        let result_value = result.unwrap();
        assert_eq!(result_value["status"], "error", "Should report error status");
        assert!(result_value["error"].as_str().unwrap().contains("File not found"), "Should indicate file not found");
    }
}
