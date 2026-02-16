//! Subprocess execution abstraction for provider purity
//!
//! This module provides a trait-based abstraction for subprocess execution,
//! enabling testing with mock implementations and WASM compatibility.

use std::fmt;

/// Output from a subprocess execution
#[derive(Debug, Clone)]
pub struct SubprocessOutput {
    /// Standard output bytes
    pub stdout: Vec<u8>,
    /// Standard error bytes
    pub stderr: Vec<u8>,
    /// Exit status code (0 typically indicates success)
    pub status_code: i32,
}

impl SubprocessOutput {
    /// Returns true if the subprocess exited successfully (status code 0)
    pub fn success(&self) -> bool {
        self.status_code == 0
    }

    /// Returns stdout as a UTF-8 string, lossy converting invalid bytes
    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Returns stderr as a UTF-8 string, lossy converting invalid bytes
    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// Error type for subprocess execution failures
#[derive(Debug, Clone)]
pub struct SubprocessError {
    /// Human-readable error message
    pub message: String,
}

impl SubprocessError {
    /// Create a new subprocess error with the given message
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for SubprocessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SubprocessError {}

/// Abstraction trait for subprocess execution
///
/// This trait allows providers to execute external commands without
/// directly depending on `std::process::Command`, enabling:
/// - Unit testing with mock implementations
/// - WASM compatibility with alternative implementations
/// - Sandboxing and security controls
pub trait SubprocessRuntime: Send + Sync {
    /// Execute a command with the given arguments and optional stdin
    ///
    /// # Arguments
    /// * `program` - The program to execute (e.g., "perltidy", "perlcritic")
    /// * `args` - Command line arguments
    /// * `stdin` - Optional data to write to the process's stdin
    ///
    /// # Returns
    /// * `Ok(SubprocessOutput)` - The command completed (check status_code for success)
    /// * `Err(SubprocessError)` - The command failed to start or other I/O error
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError>;
}

/// Default implementation using `std::process::Command`
///
/// This implementation is only available on non-WASM targets.
#[cfg(not(target_arch = "wasm32"))]
pub struct OsSubprocessRuntime;

#[cfg(not(target_arch = "wasm32"))]
impl OsSubprocessRuntime {
    /// Create a new OS subprocess runtime
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for OsSubprocessRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SubprocessRuntime for OsSubprocessRuntime {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut cmd = Command::new(program);
        cmd.args(args);

        // Configure stdin based on whether we need to write to it
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| SubprocessError::new(format!("Failed to start {}: {}", program, e)))?;

        // Write to stdin if provided
        if let Some(input) = stdin
            && let Some(mut child_stdin) = child.stdin.take()
        {
            child_stdin.write_all(input).map_err(|e| {
                SubprocessError::new(format!("Failed to write to {} stdin: {}", program, e))
            })?;
        }

        // Wait for completion
        let output = child
            .wait_with_output()
            .map_err(|e| SubprocessError::new(format!("Failed to wait for {}: {}", program, e)))?;

        Ok(SubprocessOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            status_code: output.status.code().unwrap_or(-1),
        })
    }
}

/// Mock subprocess runtime for testing
///
/// This implementation allows tests to define expected command invocations
/// and their responses without actually executing subprocesses.
#[cfg(test)]
pub mod mock {
    use super::*;
    use perl_tdd_support::must;
    use std::sync::{Arc, Mutex};

    /// A recorded command invocation
    #[derive(Debug, Clone)]
    pub struct CommandInvocation {
        /// The program that was called
        pub program: String,
        /// The arguments passed
        pub args: Vec<String>,
        /// The stdin data provided
        pub stdin: Option<Vec<u8>>,
    }

    /// Builder for mock responses
    #[derive(Debug, Clone)]
    pub struct MockResponse {
        /// Stdout to return
        pub stdout: Vec<u8>,
        /// Stderr to return
        pub stderr: Vec<u8>,
        /// Status code to return
        pub status_code: i32,
    }

    impl MockResponse {
        /// Create a successful mock response with the given stdout
        pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
            Self { stdout: stdout.into(), stderr: Vec::new(), status_code: 0 }
        }

        /// Create a failed mock response with the given stderr
        pub fn failure(stderr: impl Into<Vec<u8>>, status_code: i32) -> Self {
            Self { stdout: Vec::new(), stderr: stderr.into(), status_code }
        }
    }

    /// Mock subprocess runtime for testing
    pub struct MockSubprocessRuntime {
        /// Recorded invocations
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
        /// Responses to return (in order)
        responses: Arc<Mutex<Vec<MockResponse>>>,
        /// Default response if responses are exhausted
        default_response: MockResponse,
    }

    impl MockSubprocessRuntime {
        /// Create a new mock runtime with a default successful response
        pub fn new() -> Self {
            Self {
                invocations: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(Vec::new())),
                default_response: MockResponse::success(Vec::new()),
            }
        }

        /// Add a response to be returned for the next command
        pub fn add_response(&self, response: MockResponse) {
            must(self.responses.lock()).push(response);
        }

        /// Set the default response when no queued responses remain
        pub fn set_default_response(&mut self, response: MockResponse) {
            self.default_response = response;
        }

        /// Get all recorded invocations
        pub fn invocations(&self) -> Vec<CommandInvocation> {
            must(self.invocations.lock()).clone()
        }

        /// Clear recorded invocations
        pub fn clear_invocations(&self) {
            must(self.invocations.lock()).clear();
        }
    }

    impl Default for MockSubprocessRuntime {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SubprocessRuntime for MockSubprocessRuntime {
        fn run_command(
            &self,
            program: &str,
            args: &[&str],
            stdin: Option<&[u8]>,
        ) -> Result<SubprocessOutput, SubprocessError> {
            // Record the invocation
            must(self.invocations.lock()).push(CommandInvocation {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                stdin: stdin.map(|s| s.to_vec()),
            });

            // Get the next response or use default
            let response = {
                let mut responses = must(self.responses.lock());
                if responses.is_empty() {
                    self.default_response.clone()
                } else {
                    responses.remove(0)
                }
            };

            Ok(SubprocessOutput {
                stdout: response.stdout,
                stderr: response.stderr,
                status_code: response.status_code,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn test_subprocess_output_success() {
        let output = SubprocessOutput { stdout: vec![1, 2, 3], stderr: vec![], status_code: 0 };
        assert!(output.success());
    }

    #[test]
    fn test_subprocess_output_failure() {
        let output = SubprocessOutput { stdout: vec![], stderr: b"error".to_vec(), status_code: 1 };
        assert!(!output.success());
        assert_eq!(output.stderr_lossy(), "error");
    }

    #[test]
    fn test_subprocess_error_display() {
        let error = SubprocessError::new("test error");
        assert_eq!(format!("{}", error), "test error");
    }

    #[test]
    fn test_mock_runtime() {
        use mock::*;

        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success(b"formatted code".to_vec()));

        let result = runtime.run_command("perltidy", &["-st"], Some(b"my $x = 1;"));

        assert!(result.is_ok());
        let output = must(result);
        assert!(output.success());
        assert_eq!(output.stdout_lossy(), "formatted code");

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perltidy");
        assert_eq!(invocations[0].args, vec!["-st"]);
        assert_eq!(invocations[0].stdin, Some(b"my $x = 1;".to_vec()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_os_runtime_echo() {
        let runtime = OsSubprocessRuntime::new();

        // Test with echo which should be available on most systems
        let result = runtime.run_command("echo", &["hello"], None);

        assert!(result.is_ok());
        let output = must(result);
        assert!(output.success());
        assert!(output.stdout_lossy().trim() == "hello");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_os_runtime_nonexistent() {
        let runtime = OsSubprocessRuntime::new();

        let result = runtime.run_command("nonexistent_program_xyz", &[], None);

        assert!(result.is_err());
    }
}
