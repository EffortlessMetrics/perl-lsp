use crate::{SubprocessError, SubprocessOutput, SubprocessRuntime};
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
            if responses.is_empty() { self.default_response.clone() } else { responses.remove(0) }
        };

        Ok(SubprocessOutput {
            stdout: response.stdout,
            stderr: response.stderr,
            status_code: response.status_code,
        })
    }
}
