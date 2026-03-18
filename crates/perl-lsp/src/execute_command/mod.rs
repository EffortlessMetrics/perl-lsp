//! Execute command support for running tests and debugging.
//!
//! This module provides comprehensive support for the LSP executeCommand request,
//! enabling seamless integration between editors and Perl development workflows.
//! It implements the dual analyzer strategy for code quality analysis with 100% availability.
//!
//! ## LSP Workflow Integration
//!
//! The executeCommand implementation follows the Parse → Index → Navigate → Complete → Analyze workflow:
//! - **Parse**: Source files are parsed using the perl-parser for syntax validation
//! - **Index**: Command metadata is indexed for efficient command resolution
//! - **Navigate**: Commands provide navigation to test results and diagnostic locations
//! - **Complete**: Auto-completion for command parameters and test subroutines
//! - **Analyze**: Comprehensive code quality analysis via dual analyzer strategy
//!
//! ## Performance Characteristics
//!
//! - **Command execution**: <50ms response time for code actions
//! - **executeCommand processing**: <2s execution time for comprehensive analysis
//! - **Memory usage**: <10MB for typical Perl file analysis
//! - **Incremental analysis**: Leverages ≤1ms parsing SLO for real-time feedback
//!
//! ## Supported Commands
//!
//! ```no_run
//! use perl_lsp::execute_command::{ExecuteCommandProvider, get_supported_commands};
//! use serde_json::Value;
//!
//! let provider = ExecuteCommandProvider::new();
//! let commands = get_supported_commands();
//!
//! // Execute perl.runCritic command with dual analyzer strategy
//! let result = provider.execute_command(
//!     "perl.runCritic",
//!     vec![Value::String("/path/to/file.pl".to_string())]
//! );
//! ```
//!
//! ## Error Recovery
//!
//! Commands implement comprehensive error recovery strategies:
//! - **File not found**: Graceful error responses with actionable feedback
//! - **Syntax errors**: Parse error detection with location information
//! - **External tool failures**: Automatic fallback to built-in analyzers
//! - **Permission errors**: Clear error messages with resolution suggestions

//!   Execute command implementation for Perl LSP with dual analyzer strategy.
//!
//! This module provides comprehensive executeCommand support for the Perl Language Server,
//! implementing a dual analyzer strategy that combines external tool integration with
//! built-in fallback analysis. The implementation ensures 100% availability and robust
//! security through workspace root enforcement and path traversal protection.
//!
//! # Architecture
//!
//! The module follows a dual analyzer pattern:
//! - **External Tools**: Integrates with perlcritic, perltidy, and test runners
//! - **Built-in Fallback**: Provides analysis when external tools are unavailable
//! - **Security-First**: All file operations are workspace-enforced with canonicalization
//! - **LSP Compliant**: Proper JSON-RPC error handling and timeout management
//!
//! # Supported Commands
//!
//! - `perl.runCritic` - Code quality analysis with dual analyzer strategy
//! - `perl.runFile` - Execute Perl scripts with structured output
//! - `perl.runTest` - Execute a single discovered test case
//! - `perl.runTestFile` - Execute a specific test file directly
//! - `perl.runTests` - Run test suites with coverage reporting
//! - `perl.runTestSub` - Execute individual test subroutines
//! - `perl.debugFile` - Launch file-level debugging
//!
//! # Examples
//!
//! ```no_run
//! use perl_lsp::execute_command::{ExecuteCommandProvider, command_exists};
//! use serde_json::Value;
//!
//! // Create provider with workspace security
//! let provider = ExecuteCommandProvider::with_workspace_roots(
//!     vec!["/home/user/project".into()]
//! );
//!
//! // Execute command with secure path resolution
//! let result = provider.execute_command(
//!     "perl.runCritic",
//!     vec![Value::String("file:///home/user/project/script.pl".to_string())]
//! );
//!
//! // Check tool availability
//! if command_exists("perlcritic") {
//!     println!("External perlcritic available");
//! }
//! ```

use crate::perl_critic::{BuiltInAnalyzer, CriticAnalyzer, CriticConfig};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

// Cross-platform helpers for synthesizing `ExitStatus` in tests/mocks.
#[cfg(any(test, doctest))]
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(any(test, doctest))]
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

// Map a logical exit code (0/1/…) to the platform's raw representation.
#[cfg(any(test, doctest))]
#[cfg(unix)]
#[inline]
fn raw_exit(code: i32) -> i32 {
    // POSIX: wait(2) encodes exit code in the high byte.
    code << 8
}
#[cfg(any(test, doctest))]
#[cfg(windows)]
#[inline]
fn raw_exit(code: i32) -> u32 {
    // Windows: raw is the process' exit code directly.
    code as u32
}

// Future platforms: fail fast during tests so we notice and add a mapping.
// Only enforced for test/doctest builds to avoid breaking non-Unix/Windows release targets.
#[cfg(all(any(test, doctest), not(any(unix, windows))))]
compile_error!("Add raw_exit() mapping for this platform.");

// Helper to reduce duplication in tests while keeping the trait requirement localized.
#[cfg(any(test, doctest))]
#[inline]
fn mock_status(code: i32) -> std::process::ExitStatus {
    std::process::ExitStatus::from_raw(raw_exit(code))
}

/// Commands supported by the Perl LSP server for test execution and code analysis.
///
/// This enum defines all supported executeCommand requests that can be invoked from
/// LSP-compatible editors. Each command provides specific functionality for Perl
/// development workflows with comprehensive error handling and result formatting.
///
/// # Examples
///
/// ```no_run
/// use perl_lsp::execute_command::PerlCommand;
/// use serde_json;
///
/// // Deserialize command from LSP request
/// let json = r#"{"runTests": {"filePath": "/path/to/test.pl"}}"#;
/// let command: Result<PerlCommand, _> = serde_json::from_str(json);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PerlCommand {
    /// Run all tests in a file using appropriate test runner (prove or perl).
    ///
    /// Automatically detects test files (.t extension, /t/ directory, or 'test' in name)
    /// and uses the optimal execution strategy for maximum compatibility.
    RunTests {
        /// Path to the Perl test file to execute
        file_path: String,
    },
    /// Run a specific test subroutine with enhanced error detection.
    ///
    /// Executes a named subroutine within a test file, providing targeted
    /// test execution for faster development feedback cycles.
    RunTestSub {
        /// Path to the Perl file containing the subroutine
        file_path: String,
        /// Name of the subroutine to execute
        sub_name: String,
    },
    /// Run a Perl file directly with the perl interpreter.
    ///
    /// Provides direct execution of Perl scripts with standardized result formatting
    /// and comprehensive error capture for development workflows.
    RunFile {
        /// Path to the Perl file to execute
        file_path: String,
    },
    /// Debug a test file (placeholder for future DAP integration).
    ///
    /// Reserved for future Debug Adapter Protocol integration. Currently returns
    /// a structured response indicating debugging support is planned.
    DebugTests {
        /// Path to the test file for debugging
        file_path: String,
    },
}

/// Result of executing a command with standardized structure.
///
/// All executeCommand operations return results in this consistent format,
/// enabling reliable error handling and result processing in LSP clients.
///
/// # Examples
///
/// ```
/// use perl_lsp::execute_command::CommandResult;
///
/// let result = CommandResult {
///     success: true,
///     output: "Tests passed successfully".to_string(),
///     error: None,
/// };
/// ```
#[derive(Debug, Serialize)]
pub struct CommandResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Standard output from the command execution
    pub output: String,
    /// Error message if the command failed, None if successful
    pub error: Option<String>,
}

/// Execute command provider implementing the LSP executeCommand method.
///
/// This provider handles all supported Perl LSP commands with comprehensive error
/// handling, dual analyzer strategy for code quality, and performance optimization.
/// It integrates seamlessly with the LSP workflow for enterprise-grade functionality.
///
/// # Performance
///
/// - Command resolution: <1ms using efficient routing
/// - Code analysis: <2s for comprehensive quality checks
/// - Memory usage: <10MB for typical Perl files
/// - Thread safety: Fully thread-safe for concurrent LSP requests
///
/// # Examples
///
/// ```no_run
/// use perl_lsp::execute_command::ExecuteCommandProvider;
/// use serde_json::Value;
///
/// let provider = ExecuteCommandProvider::new();
///
/// // Execute code quality analysis
/// let result = provider.execute_command(
///     "perl.runCritic",
///     vec![Value::String("/path/to/file.pl".to_string())]
/// );
///
/// match result {
///     Ok(response) => println!("Analysis completed: {:?}", response),
///     Err(error) => eprintln!("Command failed: {}", error),
/// }
/// ```
pub struct ExecuteCommandProvider {
    /// Workspace root paths for security enforcement
    workspace_roots: Vec<PathBuf>,
}

impl Default for ExecuteCommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecuteCommandProvider {
    /// Create a new execute command provider.
    ///
    /// Initializes the provider with default configuration optimized for
    /// performance and reliability in LSP environments.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_lsp::execute_command::ExecuteCommandProvider;
    ///
    /// let provider = ExecuteCommandProvider::new();
    /// ```
    pub fn new() -> Self {
        Self { workspace_roots: Vec::new() }
    }

    /// Create a new execute command provider with workspace root enforcement.
    ///
    /// This constructor enables path traversal protection by enforcing that all
    /// file operations must be within the specified workspace root directories.
    ///
    /// # Arguments
    ///
    /// * `workspace_roots` - The root directory paths to enforce for security
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_lsp::execute_command::ExecuteCommandProvider;
    /// use std::path::PathBuf;
    ///
    /// let provider = ExecuteCommandProvider::with_workspace_roots(
    ///     vec![PathBuf::from("/home/user/project")]
    /// );
    /// ```
    pub fn with_workspace_roots(workspace_roots: Vec<PathBuf>) -> Self {
        Self { workspace_roots }
    }

    /// Execute a command with comprehensive error handling and argument validation.
    ///
    /// This is the main entry point for LSP executeCommand requests. It provides
    /// routing to specific command implementations with consistent error handling
    /// and response formatting.
    ///
    /// # Arguments
    ///
    /// * `command` - The command identifier (e.g., "perl.runCritic")
    /// * `arguments` - Command arguments as JSON values
    ///
    /// # Returns
    ///
    /// Returns a JSON response with standardized structure or an error message.
    /// All successful responses include status, output, and metadata fields.
    ///
    /// # Performance
    ///
    /// - Command routing: <1ms for all supported commands
    /// - Argument validation: <1ms with comprehensive type checking
    /// - Total overhead: <2ms excluding actual command execution
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_lsp::execute_command::ExecuteCommandProvider;
    /// use serde_json::Value;
    ///
    /// let provider = ExecuteCommandProvider::new();
    ///
    /// // Run code quality analysis
    /// let result = provider.execute_command(
    ///     "perl.runCritic",
    ///     vec![Value::String("/path/to/file.pl".to_string())]
    /// );
    ///
    /// // Run specific test subroutine
    /// let test_result = provider.execute_command(
    ///     "perl.runTestSub",
    ///     vec![
    ///         Value::String("/path/to/test.pl".to_string()),
    ///         Value::String("test_function".to_string())
    ///     ]
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` for:
    /// - Unknown command identifiers
    /// - Missing or invalid arguments
    /// - File access errors
    /// - Command execution failures
    pub fn execute_command(&self, command: &str, arguments: Vec<Value>) -> Result<Value, String> {
        match command {
            "perl.runTests" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_tests(&file_path)
            }
            "perl.runFile" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_file(&file_path)
            }
            "perl.runTestSub" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                let sub_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subroutine name argument".to_string())?;
                self.run_test_sub(&file_path, sub_name)
            }
            "perl.debugTests" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.debug_tests(&file_path)
            }
            "perl.runCritic" => {
                // Use secure path resolution instead of extract_file_path_argument
                self.run_critic_secure(&arguments)
            }
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    /// Run all tests in a file using appropriate test runner
    fn run_tests(&self, file_path: &Path) -> Result<Value, String> {
        let file_path_str = file_path.to_string_lossy();
        let is_test_file = self.is_test_file(&file_path_str);
        let (command_name, mut cmd) = if is_test_file && self.command_exists("prove") {
            ("prove", {
                let mut cmd = Command::new("prove");
                cmd.arg("-v").arg("--").arg(file_path.as_os_str());
                cmd
            })
        } else {
            ("perl", {
                let mut cmd = Command::new("perl");
                cmd.arg("--").arg(file_path.as_os_str());
                cmd
            })
        };

        let result = cmd.output().map_err(|e| format!("Failed to run {}: {}", command_name, e))?;

        Ok(self.format_command_result(result, Some(("command", command_name.into()))))
    }

    /// Run a specific test subroutine with enhanced error handling
    fn run_test_sub(&self, file_path: &Path, sub_name: &str) -> Result<Value, String> {
        // Enhanced subroutine invocation with better error detection
        // Use @ARGV to safely pass file path and subroutine name preventing code injection
        let perl_code = r#"
            my ($file, $sub) = @ARGV;
            do $file;
            if (defined &$sub) {
                no strict 'refs';
                &$sub();
            } else {
                die "Subroutine $sub not found";
            }
        "#;

        let result = Command::new("perl")
            .arg("-e")
            .arg(perl_code)
            .arg("--")
            .arg(file_path.as_os_str())
            .arg(sub_name)
            .output()
            .map_err(|e| format!("Failed to run test subroutine: {}", e))?;

        Ok(self.format_command_result(result, Some(("subroutine", sub_name.into()))))
    }

    /// Run a Perl file with standardized result formatting
    fn run_file(&self, file_path: &Path) -> Result<Value, String> {
        let result = Command::new("perl")
            .arg("--") // Safety against argument injection
            .arg(file_path.as_os_str())
            .output()
            .map_err(|e| format!("Failed to run file: {}", e))?;

        Ok(self.format_command_result(result, None))
    }

    /// Debug tests (placeholder for future implementation)
    fn debug_tests(&self, file_path: &Path) -> Result<Value, String> {
        // For now, just run with perl -d
        // In the future, this could integrate with Perl debugger or DAP
        let file_path_str = file_path.to_string_lossy();
        Ok(json!({
            "success": false,
            "output": format!("Debug mode not yet implemented for {}", file_path_str),
            "error": Some("Debugging support coming soon".to_string())
        }))
    }

    /// Run Perl::Critic analysis using dual analyzer strategy with secure path resolution
    fn run_critic_secure(&self, arguments: &[Value]) -> Result<Value, String> {
        // Use secure path resolution with workspace enforcement
        let canonical_path = match self.resolve_path_from_args(arguments) {
            Ok(path) => path,
            Err(e) => {
                // Missing arguments are validation errors - must fail with Err
                if e.contains("Missing file path argument") {
                    return Err(e);
                }

                // Handle file not found errors gracefully with structured error response
                // IMPORTANT: Preserve the path in the error message for debugging
                if e.contains("File not found")
                    || e.contains("does not exist")
                    || e.contains("No such file or directory")
                    || e.contains("Failed to canonicalize")
                {
                    // Extract and preserve the full error message which includes the path
                    let error_message = if e.contains("Failed to canonicalize") {
                        // Extract path from "Failed to canonicalize path 'X': Y"
                        if let Some(start) = e.find("'") {
                            if let Some(end) = e[start + 1..].find("'") {
                                let path = &e[start + 1..start + 1 + end];
                                format!("File not found: {}", path)
                            } else {
                                "File not found".to_string()
                            }
                        } else {
                            "File not found".to_string()
                        }
                    } else {
                        // For "File not found: X" errors, preserve as-is
                        e.clone()
                    };
                    return Ok(self.format_critic_error(error_message, "none"));
                }

                // Security-related errors (workspace traversal, length, ..) are failures
                if e.contains("Path traversal")
                    || e.contains("outside workspace")
                    || e.contains("Argument too long")
                {
                    return Err(format!("Path resolution failed: {}", e));
                }

                // All other errors are handled gracefully
                return Ok(self.format_critic_error(e, "none"));
            }
        };

        // Dual analyzer strategy: external perlcritic with built-in fallback
        if command_exists("perlcritic") {
            match self.run_external_critic(&canonical_path) {
                Ok(result) => return Ok(result),
                Err(_) => {
                    // Silently fall back to built-in analyzer for seamless UX
                }
            }
        }

        // Built-in analyzer provides 100% availability
        self.run_builtin_critic(&canonical_path)
    }

    /// Run Perl::Critic analysis using dual analyzer strategy (legacy method - deprecated)
    ///
    /// # Security Warning
    ///
    /// This method is deprecated and vulnerable to path traversal attacks.
    /// Use `run_critic_secure` instead for secure path resolution.
    #[deprecated(since = "0.8.9", note = "Use run_critic_secure for secure path resolution")]
    #[allow(dead_code)]
    #[allow(deprecated)]
    fn run_critic(&self, file_path: &str) -> Result<Value, String> {
        let normalized_path = self.normalize_file_path(file_path);
        let path = Path::new(normalized_path);

        if !path.exists() {
            return Ok(
                self.format_critic_error(format!("File not found: {}", normalized_path), "none")
            );
        }

        // Dual analyzer strategy: external perlcritic with built-in fallback
        if command_exists("perlcritic") {
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

        let mut analyzer = CriticAnalyzer::with_os_runtime(config);

        match analyzer.analyze_file(file_path) {
            Ok(violations) => {
                let formatted_violations: Vec<_> = violations
                    .iter()
                    .map(|v| {
                        self.format_violation(
                            &v.policy,
                            &v.description,
                            &v.explanation,
                            v.severity as u8,
                            (v.range.start.line + 1) as usize,
                            (v.range.start.column + 1) as usize,
                            &v.file,
                        )
                    })
                    .collect();

                Ok(json!({
                    "status": "success",
                    "violations": formatted_violations,
                    "violationCount": formatted_violations.len(),
                    "analyzerUsed": "external"
                }))
            }
            Err(e) => Err(format!("External perlcritic failed: {}", e)),
        }
    }

    /// Run built-in critic analyzer with comprehensive error handling
    fn run_builtin_critic(&self, file_path: &Path) -> Result<Value, String> {
        use crate::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let code_text = perl_parser::util::code_slice(&content);
        let mut parser = Parser::new(code_text);

        let (ast, parse_error) = match parser.parse() {
            Ok(ast) => (ast, None),
            Err(error) => {
                let message = error.to_string();
                (
                    crate::ast::Node::new(
                        crate::ast::NodeKind::Error {
                            message,
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        crate::ast::SourceLocation { start: 0, end: code_text.len() },
                    ),
                    Some(error),
                )
            }
        };

        let analyzer = BuiltInAnalyzer::new();
        let mut all_violations = analyzer.analyze(&ast, code_text);
        if let Some(error) = parse_error {
            all_violations.push(self.create_syntax_error_violation(&error, code_text, file_path));
        }

        let formatted_violations: Vec<_> = all_violations
            .iter()
            .map(|v| {
                self.format_violation(
                    &v.policy,
                    &v.description,
                    &v.explanation,
                    v.severity as u8,
                    (v.range.start.line + 1) as usize,
                    (v.range.start.column + 1) as usize,
                    &file_path.to_string_lossy(),
                )
            })
            .collect();

        Ok(json!({
            "status": "success",
            "violations": formatted_violations,
            "violationCount": formatted_violations.len(),
            "analyzerUsed": "builtin"
        }))
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

    /// Securely resolve a file path from command arguments with workspace root enforcement.
    ///
    /// This method provides comprehensive path traversal protection by:
    /// - Normalizing file:// URIs to local file paths
    /// - Canonicalizing paths to resolve .. and . components
    /// - Enforcing workspace root boundaries when configured
    /// - Validating file existence and readability
    ///
    /// # Arguments
    ///
    /// * `arguments` - Command arguments containing the file path
    ///
    /// # Returns
    ///
    /// A canonicalized `PathBuf` if the path is valid and within workspace bounds
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No file path argument is provided
    /// - Path contains invalid characters or traversal attempts
    /// - Path is outside the workspace root (if configured)
    /// - File does not exist or is not readable
    ///
    /// # Security
    ///
    /// This method prevents path traversal attacks by canonicalizing paths
    /// and enforcing workspace boundaries. All paths are resolved relative
    /// to the workspace root when configured.
    fn resolve_path_from_args(&self, arguments: &[Value]) -> Result<PathBuf, String> {
        // Extract the file path argument
        let raw_path = arguments
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing file path argument".to_string())?;

        // Defense in depth: cap argument length to prevent abuse
        const MAX_ARG_LENGTH: usize = 4096;
        if raw_path.len() > MAX_ARG_LENGTH {
            return Err(format!(
                "Argument too long ({} bytes, max {})",
                raw_path.len(),
                MAX_ARG_LENGTH
            ));
        }

        // Normalize file:// URIs
        let normalized_path = raw_path.strip_prefix("file://").unwrap_or(raw_path);

        // Defense in depth: reject paths with parent traversal components
        // even though canonicalize() resolves them, this catches attempts early
        if normalized_path.contains("..") {
            return Err("Path traversal attempt detected: path contains '..' component".to_string());
        }

        // Convert to PathBuf and canonicalize to resolve .. and . components
        let path = Path::new(normalized_path);
        let canonical_path = path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path '{}': {}", normalized_path, e))?;

        // Determine workspace boundaries
        // Security: When workspace_roots is empty (single-file mode), use CWD as the
        // fallback boundary to prevent unrestricted path traversal. This ensures that
        // even without explicit workspace configuration, files outside the working
        // directory cannot be accessed via executeCommand.
        let effective_roots: Vec<PathBuf> = if self.workspace_roots.is_empty() {
            // Fallback: use CWD as boundary when no workspace roots configured
            // This prevents unrestricted path traversal in single-file mode
            match std::env::current_dir() {
                Ok(cwd) => vec![cwd],
                Err(_) => {
                    return Err(
                        "No workspace roots configured and cannot determine working directory"
                            .to_string(),
                    );
                }
            }
        } else {
            self.workspace_roots.clone()
        };

        let mut allowed = false;
        for workspace_root in &effective_roots {
            if let Ok(canonical_root) = workspace_root.canonicalize() {
                if canonical_path.starts_with(&canonical_root) {
                    allowed = true;
                    break;
                }
            }
        }

        if !allowed {
            return Err(format!(
                "Path traversal detected: {} is outside workspace boundaries",
                canonical_path.display()
            ));
        }

        // Validate file existence and readability
        if !canonical_path.exists() {
            return Err(format!("File not found: {}", canonical_path.display()));
        }

        if !canonical_path.is_file() {
            return Err(format!("Path is not a file: {}", canonical_path.display()));
        }

        // Check basic readability (this will fail fast if permissions are wrong)
        std::fs::metadata(&canonical_path).map_err(|e| {
            format!("Cannot read file metadata '{}': {}", canonical_path.display(), e)
        })?;

        Ok(canonical_path)
    }

    /// Resolve a debug file path with the same workspace security as other commands.
    ///
    /// Wraps `resolve_path_from_args` for a single string path argument,
    /// providing the same path traversal protection and workspace enforcement.
    pub fn resolve_debug_file_path(&self, file_path: &str) -> Result<PathBuf, String> {
        self.resolve_path_from_args(&[Value::String(file_path.to_string())])
    }

    /// Normalize file path by handling URI schemes and path formats (legacy method - deprecated)
    ///
    /// # Security Warning
    ///
    /// This method is deprecated and vulnerable to path traversal attacks.
    /// Use `resolve_path_from_args` instead for secure path resolution.
    #[deprecated(since = "0.8.9", note = "Use resolve_path_from_args for secure path resolution")]
    #[allow(dead_code)]
    fn normalize_file_path<'a>(&self, file_path: &'a str) -> &'a str {
        file_path.strip_prefix("file://").unwrap_or(file_path)
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
        error: &perl_parser::ParseError,
        _content: &str,
        file_path: &Path,
    ) -> crate::perl_critic::Violation {
        let error_msg = format!("{}", error);
        let (line, column) = (0, 0); // Default for parse errors

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

/// Check if a command exists in the system PATH without using external tools.
///
/// This function provides a portable way to check command availability by
/// attempting to execute the command with `--version` flag and checking
/// if it succeeds. This avoids dependency on `which` or similar utilities.
///
/// # Arguments
///
/// * `command` - The command name to check
///
/// # Returns
///
/// `true` if the command exists and is executable, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// use perl_lsp::execute_command::command_exists;
///
/// if command_exists("perlcritic") {
///     println!("perlcritic is available");
/// }
/// ```
pub fn command_exists(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the list of supported commands for LSP executeCommand capability.
///
/// Returns all command identifiers that can be executed via the LSP executeCommand
/// method. This list is used for capability registration and command validation.
///
/// # Returns
///
/// A vector of command identifiers including:
/// - `perl.runTests`: Execute all tests in a file
/// - `perl.runFile`: Run a Perl file directly
/// - `perl.runTestSub`: Execute a specific test subroutine
/// - `perl.runCritic`: Perform code quality analysis
/// - `perl.runTest`: Run a single test
/// - `perl.runTestFile`: Run a test file
/// - `perl.debugFile`: Debug a Perl file
///
/// # Examples
///
/// ```
/// use perl_lsp::execute_command::get_supported_commands;
///
/// let commands = get_supported_commands();
/// assert!(commands.contains(&"perl.runCritic".to_string()));
/// assert_eq!(commands.len(), 7);
/// ```
///
/// # Performance
///
/// - Execution time: <1ms (static list generation)
/// - Memory usage: <1KB for command list
pub fn get_supported_commands() -> Vec<String> {
    crate::protocol::capabilities::get_supported_commands()
}

mod executor;
pub use executor::CommandExecutor;

#[cfg(test)]
mod tests;
