//! Execute command support for running tests and debugging
//! 
//! This module provides support for the LSP executeCommand request,
//! allowing users to run tests directly from their editor.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;

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
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    /// Run all tests in a file
    fn run_tests(&self, file_path: &str) -> Result<Value, String> {
        // Check if file looks like a test file
        let is_test_file = file_path.ends_with(".t") 
            || file_path.contains("/t/")
            || file_path.contains("test");

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
    ]
}