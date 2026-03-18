use super::ExecuteCommandProvider;
use crate::protocol::JsonRpcError;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Command executor for LSP incremental server with proper JSON-RPC error handling.
///
/// This struct provides a bridge between the incremental LSP server and the
/// ExecuteCommandProvider, ensuring that errors are returned as proper JSON-RPC
/// errors rather than embedded in the result payload.
///
/// # JSON-RPC Error Mapping
///
/// - Invalid arguments → `-32602` (InvalidParams)
/// - Unknown commands → `-32601` (MethodNotFound)
/// - Security violations → `-32603` (InternalError)
/// - General failures → `-32603` (InternalError)
///
/// # Examples
///
/// ```no_run
/// use perl_lsp::execute_command::CommandExecutor;
/// use serde_json::Value;
///
/// let executor = CommandExecutor::new();
/// let result = executor.execute("perl.runCritic", Some(&vec![
///     Value::String("file:///path/to/file.pl".to_string())
/// ]));
/// ```
pub struct CommandExecutor {
    provider: ExecuteCommandProvider,
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor {
    /// Create a new command executor with default configuration.
    ///
    /// The executor is initialized with workspace-agnostic configuration.
    /// For workspace-aware security enforcement, use `with_workspace_roots`.
    pub fn new() -> Self {
        Self { provider: ExecuteCommandProvider::new() }
    }

    /// Create a new command executor with workspace root enforcement.
    ///
    /// This constructor enables path traversal protection by enforcing that all
    /// file operations must be within the specified workspace root directory.
    ///
    /// # Arguments
    ///
    /// * `workspace_roots` - The root directory paths to enforce for security
    pub fn with_workspace_roots(workspace_roots: Vec<PathBuf>) -> Self {
        Self { provider: ExecuteCommandProvider::with_workspace_roots(workspace_roots) }
    }

    /// Execute a command with proper JSON-RPC error handling.
    ///
    /// This method converts ExecuteCommandProvider results into proper LSP-compatible
    /// JSON-RPC responses, mapping errors to appropriate error codes according to
    /// LSP 3.17 specification.
    ///
    /// # Arguments
    ///
    /// * `command` - The command name to execute
    /// * `arguments` - Optional array of command arguments
    ///
    /// # Returns
    ///
    /// A Result containing either the command result Value or a JsonRpcError
    /// with appropriate error code and contextual information.
    ///
    /// # Error Codes
    ///
    /// - `-32602`: Invalid arguments or missing required parameters
    /// - `-32601`: Unknown or unsupported command
    /// - `-32603`: Internal errors including security violations
    pub fn execute(
        &self,
        command: &str,
        arguments: Option<&Vec<Value>>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Convert arguments to the format expected by ExecuteCommandProvider
        let args = arguments.cloned().unwrap_or_default();

        match self.provider.execute_command(command, args) {
            Ok(result) => Ok(Some(result)),
            Err(e) => {
                // Map errors to appropriate JSON-RPC error codes
                let error_code = if e.contains("Missing") || e.contains("argument") {
                    -32602 // InvalidParams
                } else if e.contains("Unknown command") {
                    -32601 // MethodNotFound
                } else if e.contains("Path traversal")
                    || e.contains("security")
                    || e.contains("workspace root")
                {
                    -32603 // InternalError (security)
                } else {
                    -32603 // InternalError (general)
                };

                Err(JsonRpcError {
                    code: error_code,
                    message: format!("Execute command failed: {}", e),
                    data: Some(json!({
                        "command": command,
                        "errorType": "executeCommand",
                        "originalError": e
                    })),
                })
            }
        }
    }
}
