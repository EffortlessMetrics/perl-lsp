//! Subprocess execution abstraction for provider purity
//!
//! This crate provides a trait-based abstraction for subprocess execution,
//! enabling testing with mock implementations and WASM compatibility.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::fmt;
#[cfg(all(not(target_arch = "wasm32"), windows))]
use std::path::Path;

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

/// Abstraction trait for subprocess execution.
pub trait SubprocessRuntime: Send + Sync {
    /// Execute a command with the given arguments and optional stdin.
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError>;
}

/// Default implementation using `std::process::Command`.
#[cfg(not(target_arch = "wasm32"))]
pub struct OsSubprocessRuntime {
    timeout_secs: Option<u64>,
}

#[cfg(not(target_arch = "wasm32"))]
impl OsSubprocessRuntime {
    /// Create a new OS subprocess runtime with no timeout.
    pub fn new() -> Self {
        Self { timeout_secs: None }
    }

    /// Create a new OS subprocess runtime with the given wall-clock timeout.
    ///
    /// If the subprocess does not complete within `timeout_secs` seconds the
    /// call returns a `SubprocessError` with a "timed out" message and attempts
    /// to terminate the spawned process before returning.
    ///
    /// # Stdin size caveat
    ///
    /// Stdin data is written synchronously before the timeout poll loop begins.
    /// If the subprocess hangs before consuming stdin and the data exceeds the
    /// OS pipe buffer (~64 KiB on Linux), `run_command` will block in the write
    /// phase and the timeout will not fire.  For typical Perl source files this
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// Panics if `timeout_secs` is zero (a zero-second timeout would time out
    /// every command immediately and is almost certainly a caller bug).
    pub fn with_timeout(timeout_secs: u64) -> Self {
        assert!(timeout_secs > 0, "timeout_secs must be greater than zero");
        Self { timeout_secs: Some(timeout_secs) }
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

        let (resolved_program, resolved_args) = resolve_command_invocation(program, args);
        let mut cmd = Command::new(&resolved_program);
        cmd.args(resolved_args.iter().map(String::as_str));

        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| SubprocessError::new(format!("Failed to start {}: {}", program, e)))?;

        if let Some(input) = stdin
            && let Some(mut child_stdin) = child.stdin.take()
        {
            child_stdin.write_all(input).map_err(|e| {
                SubprocessError::new(format!("Failed to write to {} stdin: {}", program, e))
            })?;
        }

        match self.timeout_secs {
            None => {
                let output = child.wait_with_output().map_err(|e| {
                    SubprocessError::new(format!("Failed to wait for {}: {}", program, e))
                })?;
                Ok(SubprocessOutput {
                    stdout: output.stdout,
                    stderr: output.stderr,
                    status_code: output.status.code().unwrap_or(-1),
                })
            }
            Some(secs) => {
                use std::time::{Duration, Instant};

                let deadline = Instant::now() + Duration::from_secs(secs);
                loop {
                    if child
                        .try_wait()
                        .map_err(|e| {
                            SubprocessError::new(format!("Failed to poll {}: {}", program, e))
                        })?
                        .is_some()
                    {
                        let output = child.wait_with_output().map_err(|e| {
                            SubprocessError::new(format!("Failed to wait for {}: {}", program, e))
                        })?;
                        return Ok(SubprocessOutput {
                            stdout: output.stdout,
                            stderr: output.stderr,
                            status_code: output.status.code().unwrap_or(-1),
                        });
                    }

                    if Instant::now() >= deadline {
                        if let Err(kill_err) = child.kill() {
                            // Best effort: process may have already exited between `try_wait`
                            // and `kill`.
                            let already_exited = child
                                .try_wait()
                                .map_err(|e| {
                                    SubprocessError::new(format!(
                                        "Failed to poll {}: {}",
                                        program, e
                                    ))
                                })?
                                .is_some();
                            if !already_exited {
                                return Err(SubprocessError::new(format!(
                                    "subprocess timed out after {} seconds and failed to terminate {}: {}",
                                    secs, program, kill_err
                                )));
                            }
                        }
                        let _ = child.wait();
                        return Err(SubprocessError::new(format!(
                            "subprocess timed out after {} seconds",
                            secs
                        )));
                    }

                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_command_invocation(program: &str, args: &[&str]) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let resolved_program =
            resolve_windows_program(program).unwrap_or_else(|| program.to_string());

        if windows_requires_cmd_shell(&resolved_program) {
            let mut shell_args = vec!["/C".to_string(), resolved_program];
            shell_args.extend(args.iter().map(|arg| (*arg).to_string()));
            return ("cmd.exe".to_string(), shell_args);
        }

        (resolved_program, args.iter().map(|arg| (*arg).to_string()).collect())
    }

    #[cfg(not(windows))]
    {
        (program.to_string(), args.iter().map(|arg| (*arg).to_string()).collect())
    }
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
fn resolve_windows_program(program: &str) -> Option<String> {
    let program_path = Path::new(program);
    let has_separator = program.contains('\\') || program.contains('/');
    let has_extension = program_path.extension().is_some();

    if has_separator || has_extension {
        return Some(program.to_string());
    }

    let output = std::process::Command::new("where")
        .arg(program)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .max_by_key(|candidate| windows_program_priority(candidate))
        .map(String::from)
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
fn windows_program_priority(candidate: &str) -> u8 {
    match Path::new(candidate)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "exe" => 5,
        Some(ext) if ext == "com" => 4,
        Some(ext) if ext == "cmd" => 3,
        Some(ext) if ext == "bat" => 2,
        Some(_) => 1,
        None => 0,
    }
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
fn windows_requires_cmd_shell(program: &str) -> bool {
    Path::new(program)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false)
}

/// Mock subprocess runtime for testing.
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex, MutexGuard};

    fn lock<'a, T>(mutex: &'a Mutex<T>) -> MutexGuard<'a, T> {
        match mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A recorded command invocation.
    #[derive(Debug, Clone)]
    pub struct CommandInvocation {
        /// The program that was called.
        pub program: String,
        /// The arguments passed.
        pub args: Vec<String>,
        /// The stdin data provided.
        pub stdin: Option<Vec<u8>>,
    }

    /// Builder for mock responses.
    #[derive(Debug, Clone)]
    pub struct MockResponse {
        /// Stdout to return.
        pub stdout: Vec<u8>,
        /// Stderr to return.
        pub stderr: Vec<u8>,
        /// Status code to return.
        pub status_code: i32,
    }

    impl MockResponse {
        /// Create a successful mock response with the given stdout.
        pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
            Self { stdout: stdout.into(), stderr: Vec::new(), status_code: 0 }
        }

        /// Create a failed mock response with the given stderr.
        pub fn failure(stderr: impl Into<Vec<u8>>, status_code: i32) -> Self {
            Self { stdout: Vec::new(), stderr: stderr.into(), status_code }
        }
    }

    /// Mock subprocess runtime for testing.
    pub struct MockSubprocessRuntime {
        invocations: Arc<Mutex<Vec<CommandInvocation>>>,
        responses: Arc<Mutex<Vec<MockResponse>>>,
        default_response: MockResponse,
    }

    impl MockSubprocessRuntime {
        /// Create a new mock runtime with a default successful response.
        pub fn new() -> Self {
            Self {
                invocations: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(Vec::new())),
                default_response: MockResponse::success(Vec::new()),
            }
        }

        /// Add a response to be returned for the next command.
        pub fn add_response(&self, response: MockResponse) {
            lock(&self.responses).push(response);
        }

        /// Set the default response when no queued responses remain.
        pub fn set_default_response(&mut self, response: MockResponse) {
            self.default_response = response;
        }

        /// Get all recorded invocations.
        pub fn invocations(&self) -> Vec<CommandInvocation> {
            lock(&self.invocations).clone()
        }

        /// Clear recorded invocations.
        pub fn clear_invocations(&self) {
            lock(&self.invocations).clear();
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
            lock(&self.invocations).push(CommandInvocation {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                stdin: stdin.map(|s| s.to_vec()),
            });

            let response = {
                let mut responses = lock(&self.responses);
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
        let output = perl_tdd_support::must(result);
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
        #[cfg(windows)]
        let result = runtime.run_command("cmd.exe", &["/C", "echo", "hello"], None);
        #[cfg(not(windows))]
        let result = runtime.run_command("echo", &["hello"], None);

        assert!(result.is_ok());
        let output = perl_tdd_support::must(result);
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

    #[cfg(windows)]
    #[test]
    fn test_resolve_command_invocation_uses_cmd_for_batch_wrappers() {
        let (program, args) =
            resolve_command_invocation(r"C:\Strawberry\perl\bin\perltidy.bat", &["-st", "-se"]);

        assert_eq!(program, "cmd.exe");
        assert_eq!(
            args,
            vec![
                "/C".to_string(),
                r"C:\Strawberry\perl\bin\perltidy.bat".to_string(),
                "-st".to_string(),
                "-se".to_string(),
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_resolve_command_invocation_preserves_executable_paths() {
        let (program, args) =
            resolve_command_invocation(r"C:\tools\perlcritic.exe", &["--version"]);

        assert_eq!(program, r"C:\tools\perlcritic.exe");
        assert_eq!(args, vec!["--version".to_string()]);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_program_priority_prefers_real_wrappers_over_extensionless_shims() {
        let mut candidates = vec![
            r"C:\Strawberry\perl\bin\perltidy".to_string(),
            r"C:\Strawberry\perl\bin\perltidy.bat".to_string(),
            r"C:\tools\perltidy.exe".to_string(),
        ];
        candidates.sort_by_key(|candidate| windows_program_priority(candidate));

        assert_eq!(candidates.last().map(String::as_str), Some(r"C:\tools\perltidy.exe"));
        assert!(
            windows_program_priority(r"C:\Strawberry\perl\bin\perltidy.bat")
                > windows_program_priority(r"C:\Strawberry\perl\bin\perltidy")
        );
    }
}
