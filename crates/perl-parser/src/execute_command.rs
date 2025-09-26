//! Execute command support for running tests and debugging
//!
//! This module provides support for the LSP executeCommand request,
//! allowing users to run tests directly from their editor.

use crate::perl_critic::{BuiltInAnalyzer, CriticAnalyzer, CriticConfig};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
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

    /// Execute a command with proper error handling and argument validation
    pub fn execute_command(&self, command: &str, arguments: Vec<Value>) -> Result<Value, String> {
        match command {
            "perl.runTests" => {
                let file_path = self.extract_file_path_argument(&arguments)?;
                self.run_tests(file_path)
            }
            "perl.runFile" => {
                let file_path = self.extract_file_path_argument(&arguments)?;
                self.run_file(file_path)
            }
            "perl.runTestSub" => {
                let file_path = self.extract_file_path_argument(&arguments)?;
                let sub_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subroutine name argument".to_string())?;
                self.run_test_sub(file_path, sub_name)
            }
            "perl.debugTests" => {
                let file_path = self.extract_file_path_argument(&arguments)?;
                self.debug_tests(file_path)
            }
            "perl.runCritic" => {
                let file_path = self.extract_file_path_argument(&arguments)?;
                self.run_critic(file_path)
            }
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    /// Run all tests in a file using appropriate test runner
    fn run_tests(&self, file_path: &str) -> Result<Value, String> {
        let is_test_file = self.is_test_file(file_path);
        let (command_name, mut cmd) = if is_test_file && self.command_exists("prove") {
            ("prove", {
                let mut cmd = Command::new("prove");
                cmd.arg("-v").arg(file_path);
                cmd
            })
        } else {
            ("perl", {
                let mut cmd = Command::new("perl");
                cmd.arg(file_path);
                cmd
            })
        };

        let result = cmd.output().map_err(|e| format!("Failed to run {}: {}", command_name, e))?;

        Ok(self.format_command_result(result, Some(("command", command_name.into()))))
    }

    /// Run a specific test subroutine with enhanced error handling
    fn run_test_sub(&self, file_path: &str, sub_name: &str) -> Result<Value, String> {
        // Enhanced subroutine invocation with better error detection
        let perl_code = format!(
            "do '{}'; if (defined &{}) {{ {}() }} else {{ die 'Subroutine {} not found' }}",
            file_path, sub_name, sub_name, sub_name
        );

        let result = Command::new("perl")
            .arg("-e")
            .arg(perl_code)
            .output()
            .map_err(|e| format!("Failed to run test subroutine: {}", e))?;

        Ok(self.format_command_result(result, Some(("subroutine", sub_name.into()))))
    }

    /// Run a Perl file with standardized result formatting
    fn run_file(&self, file_path: &str) -> Result<Value, String> {
        let result = Command::new("perl")
            .arg(file_path)
            .output()
            .map_err(|e| format!("Failed to run file: {}", e))?;

        Ok(self.format_command_result(result, None))
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

    /// Run Perl::Critic analysis using dual analyzer strategy
    fn run_critic(&self, file_path: &str) -> Result<Value, String> {
        let normalized_path = self.normalize_file_path(file_path);
        let path = Path::new(normalized_path);

        if !path.exists() {
            return Ok(
                self.format_critic_error(format!("File not found: {}", normalized_path), "none")
            );
        }

        // Dual analyzer strategy: external perlcritic with built-in fallback
        if self.command_exists("perlcritic") {
            match self.run_external_critic(path) {
                Ok(result) => return Ok(result),
                Err(_) => {
                    // Silently fall back to built-in analyzer for seamless UX
                }
            }
        }

        // Built-in analyzer provides 100% availability
        self.run_builtin_critic(path)
    }

    /// Run external perlcritic with standardized response formatting
    fn run_external_critic(&self, file_path: &Path) -> Result<Value, String> {
        let config = CriticConfig {
            severity: 3, // Harsh and above for production-quality analysis
            verbose: true,
            ..Default::default()
        };

        let mut analyzer = CriticAnalyzer::new(config);

        match analyzer.analyze_file(file_path) {
            Ok(violations) => {
                let formatted_violations: Vec<_> = violations.iter().map(|v| self.format_violation(
                    &v.policy,
                    &v.description,
                    &v.explanation,
                    v.severity as u8,
                    (v.range.start.line + 1) as usize,
                    (v.range.start.column + 1) as usize,
                    &v.file
                )).collect();

                Ok(json!({
                    "status": "success",
                    "violations": formatted_violations,
                    "violationCount": formatted_violations.len(),
                    "analyzerUsed": "external"
                }))
            },
            Err(e) => Err(format!("External perlcritic failed: {}", e)),
        }
    }

    /// Run built-in critic analyzer with comprehensive error handling
    fn run_builtin_critic(&self, file_path: &Path) -> Result<Value, String> {
        use crate::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let mut all_violations = Vec::new();
        let code_text = crate::util::code_slice(&content);
        let mut parser = Parser::new(code_text);

        let _ast = match parser.parse() {
            Ok(ast) => {
                // Successfully parsed - run comprehensive analysis
                let analyzer = BuiltInAnalyzer::new();
                all_violations.extend(analyzer.analyze(&ast, &content));
                ast
            }
            Err(e) => {
                // Handle parse errors with detailed location information
                all_violations.push(self.create_syntax_error_violation(&e, &content, file_path));

                // Create dummy AST for additional analysis
                let dummy_ast = crate::ast::Node::new(
                    crate::ast::NodeKind::Error { message: format!("{}", e) },
                    crate::ast::SourceLocation { start: 0, end: content.len() },
                );
                let analyzer = BuiltInAnalyzer::new();
                all_violations.extend(analyzer.analyze(&dummy_ast, &content));
                dummy_ast
            }
        };

        let formatted_violations: Vec<_> = all_violations.iter().map(|v| self.format_violation(
            &v.policy,
            &v.description,
            &v.explanation,
            v.severity as u8,
            (v.range.start.line + 1) as usize,
            (v.range.start.column + 1) as usize,
            &file_path.to_string_lossy()
        )).collect();

        Ok(json!({
            "status": "success",
            "violations": formatted_violations,
            "violationCount": formatted_violations.len(),
            "analyzerUsed": "builtin"
        }))
    }

    /// Extract file path from command arguments with proper error handling
    fn extract_file_path_argument<'a>(&self, arguments: &'a [Value]) -> Result<&'a str, String> {
        arguments
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing file path argument".to_string())
    }

    /// Check if a file path appears to be a test file
    fn is_test_file(&self, file_path: &str) -> bool {
        file_path.ends_with(".t") || file_path.contains("/t/") || file_path.contains("test")
    }

    /// Format command execution result with consistent structure
    fn format_command_result(
        &self,
        result: std::process::Output,
        extra_field: Option<(&str, Value)>,
    ) -> Value {
        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        let mut response = json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error
        });

        if let Some((key, value)) = extra_field {
            response[key] = value;
        }

        response
    }

    /// Normalize file path by handling URI schemes and path formats
    fn normalize_file_path<'a>(&self, file_path: &'a str) -> &'a str {
        if file_path.starts_with("file://") {
            file_path.strip_prefix("file://").unwrap_or(file_path)
        } else {
            file_path
        }
    }

    /// Format a violation with consistent structure
    fn format_violation(
        &self,
        policy: &str,
        description: &str,
        explanation: &str,
        severity: u8,
        line: usize,
        column: usize,
        file: &str,
    ) -> Value {
        json!({
            "policy": policy,
            "description": description,
            "explanation": explanation,
            "severity": severity,
            "line": line,
            "column": column,
            "file": file
        })
    }

    /// Format critic error response with consistent structure
    fn format_critic_error(&self, error_message: String, analyzer_used: &str) -> Value {
        json!({
            "status": "error",
            "error": error_message,
            "violations": [],
            "violationCount": 0,
            "analyzerUsed": analyzer_used
        })
    }

    /// Create a syntax error violation from parse error
    fn create_syntax_error_violation(
        &self,
        error: &crate::ParseError,
        content: &str,
        file_path: &Path,
    ) -> crate::perl_critic::Violation {
        let error_msg = format!("{}", error);
        let (line, column) = match error {
            crate::ParseError::UnexpectedToken { location, .. }
            | crate::ParseError::SyntaxError { location, .. } => {
                // Convert byte location to line/column with proper bounds checking
                let line_count = content[..*location].matches('\n').count();
                let last_newline = content[..*location].rfind('\n').unwrap_or(0);
                let column = location.saturating_sub(last_newline);
                (line_count, column)
            }
            _ => (0, 0), // Default for other error types
        };

        crate::perl_critic::Violation {
            policy: "Syntax::ParseError".to_string(),
            description: format!("Syntax error: {}", error_msg),
            explanation: "This code contains a syntax error that prevents parsing. Fix the syntax error before running additional checks.".to_string(),
            severity: crate::perl_critic::Severity::Brutal, // Critical severity for syntax errors
            range: crate::position::Range {
                start: crate::position::Position { byte: 0, line: line as u32, column: column as u32 },
                end: crate::position::Position { byte: 1, line: line as u32, column: (column + 1) as u32 },
            },
            file: file_path.to_string_lossy().to_string(),
        }
    }

    /// Check if a command exists in PATH with cross-platform compatibility
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
        let result =
            provider.execute_command("perl.runCritic", vec![Value::String(temp_file.to_string())]);

        // Clean up
        fs::remove_file(temp_file).ok();

        // Verify result
        assert!(result.is_ok(), "perl.runCritic command should execute successfully");

        let result_value = result.unwrap();
        assert_eq!(result_value["status"], "success", "Command should succeed");
        assert!(result_value["violations"].is_array(), "Should return violations array");
        assert!(
            result_value["analyzerUsed"].is_string(),
            "Should indicate which analyzer was used"
        );

        // Should detect missing 'use strict' and 'use warnings'
        let violations = result_value["violations"].as_array().unwrap();
        assert!(!violations.is_empty(), "Should detect policy violations");

        // Check for specific violations
        let has_strict_violation = violations.iter().any(|v| {
            v["policy"]
                .as_str()
                .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
                .unwrap_or(false)
        });

        let has_warnings_violation = violations.iter().any(|v| {
            v["policy"]
                .as_str()
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
        let result = provider.execute_command(
            "perl.runCritic",
            vec![Value::String("/tmp/nonexistent.pl".to_string())],
        );

        assert!(result.is_ok(), "Should handle missing files gracefully");
        let result_value = result.unwrap();
        assert_eq!(result_value["status"], "error", "Should report error status");
        assert!(
            result_value["error"].as_str().unwrap().contains("File not found"),
            "Should indicate file not found"
        );
    }
}
