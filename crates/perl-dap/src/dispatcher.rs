//! DAP Message Dispatcher
//!
//! This module handles routing of incoming DAP requests to appropriate handlers
//! and constructs responses following the JSON-RPC 2.0 protocol.
//!
//! # Architecture
//!
//! - **DapDispatcher**: Central message router with handler registry
//! - **Handler Pattern**: Command-specific handlers for initialize, setBreakpoints, etc.
//! - **Error Handling**: Consistent error response formatting with proper status codes
//!
//! # Message Flow
//!
//! 1. Client sends Request with command and arguments
//! 2. Dispatcher routes to command handler
//! 3. Handler processes request and returns Result
//! 4. Dispatcher wraps in Response with success/error status
//!
//! # References
//!
//! - [DAP Protocol Schema](../../docs/DAP_PROTOCOL_SCHEMA.md)
//! - [DAP Implementation Spec](../../docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac5-adapter-scaffolding)

use crate::breakpoints::BreakpointStore;
use crate::protocol::{
    Breakpoint, Capabilities, InitializeRequestArguments, Request, Response,
    SetBreakpointsArguments, SetBreakpointsResponseBody,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// DAP message dispatcher
///
/// Routes incoming requests to command handlers and manages session state
/// including breakpoint storage and sequence number tracking.
#[derive(Debug, Clone)]
pub struct DapDispatcher {
    /// Breakpoint storage (shared across session)
    breakpoint_store: BreakpointStore,
    /// Response sequence number (monotonically increasing)
    response_seq: Arc<Mutex<i64>>,
}

impl DapDispatcher {
    /// Create a new DAP dispatcher
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_dap::dispatcher::DapDispatcher;
    ///
    /// let dispatcher = DapDispatcher::new();
    /// ```
    pub fn new() -> Self {
        Self { breakpoint_store: BreakpointStore::new(), response_seq: Arc::new(Mutex::new(1)) }
    }

    /// Dispatch a request to the appropriate handler
    ///
    /// Routes the request based on command name and returns a Response.
    /// All errors are caught and converted to error responses.
    ///
    /// # Arguments
    ///
    /// * `request` - Incoming DAP request
    ///
    /// # Returns
    ///
    /// DAP response with success=true and body, or success=false with message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap::dispatcher::DapDispatcher;
    /// use perl_dap::protocol::{Request, Source, SourceBreakpoint, SetBreakpointsArguments};
    /// use serde_json::json;
    ///
    /// let dispatcher = DapDispatcher::new();
    /// let request = Request {
    ///     seq: 1,
    ///     msg_type: "request".to_string(),
    ///     command: "setBreakpoints".to_string(),
    ///     arguments: Some(json!({
    ///         "source": { "path": "/workspace/script.pl" },
    ///         "breakpoints": [{ "line": 10 }]
    ///     })),
    /// };
    ///
    /// let response = dispatcher.dispatch(&request);
    /// assert!(response.success);
    /// ```
    pub fn dispatch(&self, request: &Request) -> Response {
        let result = self.dispatch_inner(request);
        self.create_response(request, result)
    }

    /// Internal dispatch logic (returns Result for error handling)
    fn dispatch_inner(&self, request: &Request) -> Result<Value> {
        match request.command.as_str() {
            "initialize" => self.handle_initialize(request),
            "setBreakpoints" => self.handle_set_breakpoints(request),
            _ => {
                // Unknown command - return error
                anyhow::bail!("Unknown command: {}", request.command)
            }
        }
    }

    /// Handle initialize request
    ///
    /// Returns adapter capabilities to the client.
    fn handle_initialize(&self, request: &Request) -> Result<Value> {
        // Parse initialize arguments (optional validation)
        let _args: InitializeRequestArguments = request
            .arguments
            .as_ref()
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .context("Failed to parse initialize arguments")?
            .unwrap_or(InitializeRequestArguments {
                client_id: None,
                client_name: None,
                adapter_id: "perl-rs".to_string(),
                locale: None,
                lines_start_at1: Some(true),
                columns_start_at1: Some(true),
                path_format: None,
            });

        // Return adapter capabilities
        let capabilities = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_evaluate_for_hovers: Some(true),
            supports_conditional_breakpoints: Some(false), // TODO: Phase 2 (AC7)
            supports_terminate_request: Some(true),
        };

        serde_json::to_value(&capabilities).context("Failed to serialize capabilities")
    }

    /// Handle setBreakpoints request
    ///
    /// Sets breakpoints for a source file using REPLACE semantics.
    /// Returns verified breakpoints in SAME ORDER as request.
    fn handle_set_breakpoints(&self, request: &Request) -> Result<Value> {
        // Parse setBreakpoints arguments
        let args: SetBreakpointsArguments = request
            .arguments
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing arguments for setBreakpoints"))
            .and_then(|v| serde_json::from_value(v.clone()).context("Invalid arguments"))?;

        // Set breakpoints (REPLACE semantics)
        let breakpoints: Vec<Breakpoint> = self.breakpoint_store.set_breakpoints(&args);

        // Create response body
        let body = SetBreakpointsResponseBody { breakpoints };

        serde_json::to_value(&body).context("Failed to serialize setBreakpoints response")
    }

    /// Create response from request and result
    fn create_response(&self, request: &Request, result: Result<Value>) -> Response {
        let mut seq = self.response_seq.lock().unwrap_or_else(|e| e.into_inner());
        let response_seq = *seq;
        *seq += 1;

        match result {
            Ok(body) => Response {
                seq: response_seq,
                msg_type: "response".to_string(),
                request_seq: request.seq,
                success: true,
                command: request.command.clone(),
                message: None,
                body: Some(body),
            },
            Err(err) => Response {
                seq: response_seq,
                msg_type: "response".to_string(),
                request_seq: request.seq,
                success: false,
                command: request.command.clone(),
                message: Some(err.to_string()),
                body: None,
            },
        }
    }

    /// Get reference to breakpoint store (for testing)
    #[cfg(test)]
    pub fn breakpoint_store(&self) -> &BreakpointStore {
        &self.breakpoint_store
    }
}

impl Default for DapDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = DapDispatcher::new();
        let breakpoints = dispatcher.breakpoint_store.get_breakpoints("/test.pl");
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_handle_initialize() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: Some(json!({
                "clientId": "vscode",
                "clientName": "Visual Studio Code",
                "adapterId": "perl-rs",
                "linesStartAt1": true,
                "columnsStartAt1": true,
            })),
        };

        let response = dispatcher.dispatch(&request);

        if !response.success {
            eprintln!("Error: {:?}", response.message);
        }
        assert!(response.success);
        assert_eq!(response.command, "initialize");
        assert!(response.body.is_some());

        // Parse capabilities
        let capabilities: Capabilities = serde_json::from_value(response.body.unwrap()).unwrap();
        assert_eq!(capabilities.supports_configuration_done_request, Some(true));
        assert_eq!(capabilities.supports_evaluate_for_hovers, Some(true));
    }

    #[test]
    fn test_handle_set_breakpoints() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": {
                    "path": "/workspace/script.pl",
                    "name": "script.pl"
                },
                "breakpoints": [
                    { "line": 10, "column": 0 },
                    { "line": 25, "column": 0 }
                ]
            })),
        };

        let response = dispatcher.dispatch(&request);

        assert!(response.success);
        assert_eq!(response.command, "setBreakpoints");
        assert!(response.body.is_some());

        // Parse response body
        let body: SetBreakpointsResponseBody =
            serde_json::from_value(response.body.unwrap()).unwrap();
        assert_eq!(body.breakpoints.len(), 2);
        assert_eq!(body.breakpoints[0].line, 10);
        assert_eq!(body.breakpoints[1].line, 25);
        assert!(body.breakpoints[0].verified);
        assert!(body.breakpoints[1].verified);
    }

    #[test]
    fn test_handle_set_breakpoints_replace_semantics() {
        let dispatcher = DapDispatcher::new();

        // Set initial breakpoints
        let request1 = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": { "path": "/workspace/script.pl" },
                "breakpoints": [{ "line": 10 }]
            })),
        };
        dispatcher.dispatch(&request1);

        // Replace with new breakpoints
        let request2 = Request {
            seq: 3,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": { "path": "/workspace/script.pl" },
                "breakpoints": [{ "line": 20 }, { "line": 30 }]
            })),
        };
        let response = dispatcher.dispatch(&request2);

        assert!(response.success);
        let body: SetBreakpointsResponseBody =
            serde_json::from_value(response.body.unwrap()).unwrap();
        assert_eq!(body.breakpoints.len(), 2);
        assert_eq!(body.breakpoints[0].line, 20);
        assert_eq!(body.breakpoints[1].line, 30);
    }

    #[test]
    fn test_handle_set_breakpoints_preserves_order() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": { "path": "/workspace/script.pl" },
                "breakpoints": [
                    { "line": 100 },
                    { "line": 50 },
                    { "line": 75 }
                ]
            })),
        };

        let response = dispatcher.dispatch(&request);

        assert!(response.success);
        let body: SetBreakpointsResponseBody =
            serde_json::from_value(response.body.unwrap()).unwrap();

        // Order must match request
        assert_eq!(body.breakpoints[0].line, 100);
        assert_eq!(body.breakpoints[1].line, 50);
        assert_eq!(body.breakpoints[2].line, 75);
    }

    #[test]
    fn test_handle_unknown_command() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 99,
            msg_type: "request".to_string(),
            command: "unknownCommand".to_string(),
            arguments: None,
        };

        let response = dispatcher.dispatch(&request);

        assert!(!response.success);
        assert_eq!(response.command, "unknownCommand");
        assert!(response.message.is_some());
        assert!(response.message.unwrap().contains("Unknown command: unknownCommand"));
    }

    #[test]
    fn test_handle_set_breakpoints_missing_arguments() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: None,
        };

        let response = dispatcher.dispatch(&request);

        assert!(!response.success);
        assert!(response.message.is_some());
    }

    #[test]
    fn test_response_sequence_numbers() {
        let dispatcher = DapDispatcher::new();

        let request1 = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: None,
        };
        let response1 = dispatcher.dispatch(&request1);

        let request2 = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: None,
        };
        let response2 = dispatcher.dispatch(&request2);

        // Response sequence numbers should increment
        assert_eq!(response1.seq, 1);
        assert_eq!(response2.seq, 2);
    }
}
