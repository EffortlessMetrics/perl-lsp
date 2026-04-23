use crate::protocol::JsonRpcError;
use serde_json::{Value, json};
use std::path::PathBuf;

use super::provider::ExecuteCommandProvider;

/// Command executor for LSP incremental server with proper JSON-RPC error handling.
pub struct CommandExecutor {
    provider: ExecuteCommandProvider,
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor {
    /// Create a new command executor.
    pub fn new() -> Self {
        Self { provider: ExecuteCommandProvider::new() }
    }

    /// Create a command executor with workspace root enforcement.
    pub fn with_workspace_roots(workspace_roots: Vec<PathBuf>) -> Self {
        Self { provider: ExecuteCommandProvider::with_workspace_roots(workspace_roots) }
    }

    /// Execute a command and map failures to JSON-RPC errors.
    pub fn execute(
        &self,
        command: &str,
        arguments: Option<&Vec<Value>>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let args = arguments.cloned().unwrap_or_default();

        match self.provider.execute_command(command, args) {
            Ok(result) => Ok(Some(result)),
            Err(e) => {
                let error_code = if e.contains("Missing") || e.contains("argument") {
                    -32602
                } else if e.contains("Unknown command") {
                    -32601
                } else {
                    -32603
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
