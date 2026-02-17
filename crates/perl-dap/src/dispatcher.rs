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
//! - **Event Emission**: Supports DAP events like `initialized` after capability exchange
//!
//! # Message Flow
//!
//! 1. Client sends Request with command and arguments
//! 2. Dispatcher routes to command handler
//! 3. Handler processes request and returns Result
//! 4. Dispatcher wraps in Response with success/error status
//! 5. For `initialize`, also queues an `initialized` event
//!
//! # DAP Protocol Requirements
//!
//! The DAP spec requires:
//! - Client sends `initialize` request
//! - Adapter responds with capabilities
//! - Adapter sends `initialized` event (signals ready for configuration)
//! - Client sends configuration requests like `setBreakpoints`, `configurationDone`
//!
//! # References
//!
//! - [DAP Protocol Schema](../../docs/DAP_PROTOCOL_SCHEMA.md)
//! - [DAP Implementation Spec](../../docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac5-adapter-scaffolding)
//! - [DAP Spec - Initialization](https://microsoft.github.io/debug-adapter-protocol/specification#Requests_Initialize)

use crate::breakpoints::BreakpointStore;
use crate::inline_values::collect_inline_values;
use crate::protocol::{
    Breakpoint, Capabilities, Event, InitializeRequestArguments, InlineValuesArguments,
    InlineValuesResponseBody, Request, Response, SetBreakpointsArguments,
    SetBreakpointsResponseBody,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Result of dispatching a request
///
/// Contains the response and any events that should be sent to the client.
pub struct DispatchResult {
    /// The response to send
    pub response: Response,
    /// Events to send after the response (e.g., `initialized` after `initialize`)
    pub events: Vec<Event>,
}

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
    /// Event sequence number (monotonically increasing)
    event_seq: Arc<Mutex<i64>>,
    /// Whether initialization is complete
    initialized: Arc<Mutex<bool>>,
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
        Self {
            breakpoint_store: BreakpointStore::new(),
            response_seq: Arc::new(Mutex::new(1)),
            event_seq: Arc::new(Mutex::new(1)),
            initialized: Arc::new(Mutex::new(false)),
        }
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
        self.dispatch_with_events(request).response
    }

    /// Dispatch a request and return both response and any events
    ///
    /// This method should be used when you need to send events after responses.
    /// For example, after `initialize`, the adapter must send an `initialized` event.
    ///
    /// # Arguments
    ///
    /// * `request` - Incoming DAP request
    ///
    /// # Returns
    ///
    /// `DispatchResult` containing the response and any events to send.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap::dispatcher::DapDispatcher;
    /// use perl_dap::protocol::Request;
    ///
    /// let dispatcher = DapDispatcher::new();
    /// let request = Request {
    ///     seq: 1,
    ///     msg_type: "request".to_string(),
    ///     command: "initialize".to_string(),
    ///     arguments: None,
    /// };
    ///
    /// let result = dispatcher.dispatch_with_events(&request);
    /// assert!(result.response.success);
    /// // After initialize, we get an initialized event
    /// assert_eq!(result.events.len(), 1);
    /// assert_eq!(result.events[0].event, "initialized");
    /// ```
    pub fn dispatch_with_events(&self, request: &Request) -> DispatchResult {
        let result = self.dispatch_inner(request);
        let success = result.is_ok();
        let command = request.command.as_str();
        let response = self.create_response(request, result);

        // Generate events based on command
        let events = match (command, success) {
            ("initialize", true) => {
                // After successful initialize, send initialized event
                vec![self.create_initialized_event()]
            }
            _ => Vec::new(),
        };

        DispatchResult { response, events }
    }

    /// Internal dispatch logic (returns Result for error handling)
    fn dispatch_inner(&self, request: &Request) -> Result<Value> {
        match request.command.as_str() {
            "initialize" => self.handle_initialize(request),
            "configurationDone" => self.handle_configuration_done(request),
            "setBreakpoints" => self.handle_set_breakpoints(request),
            "inlineValues" => self.handle_inline_values(request),
            _ => {
                // Unknown command - return error
                anyhow::bail!("Unknown command: {}", request.command)
            }
        }
    }

    /// Create an initialized event
    ///
    /// This event signals to the client that the adapter is ready to receive
    /// configuration requests (breakpoints, etc.)
    fn create_initialized_event(&self) -> Event {
        let mut seq = self.event_seq.lock().unwrap_or_else(|e| e.into_inner());
        let event_seq = *seq;
        *seq += 1;

        // Mark as initialized
        if let Ok(mut init) = self.initialized.lock() {
            *init = true;
        }

        Event {
            seq: event_seq,
            msg_type: "event".to_string(),
            event: "initialized".to_string(),
            body: None, // initialized event has no body
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
            supports_conditional_breakpoints: Some(false), // Phase 2 (AC7) - See #450
            supports_hit_conditional_breakpoints: Some(false),
            supports_log_points: Some(false),
            supports_exception_options: Some(false),
            supports_exception_filter_options: Some(false),
            supports_terminate_request: Some(true),
        };

        serde_json::to_value(&capabilities).context("Failed to serialize capabilities")
    }

    /// Handle configurationDone request
    ///
    /// This request is sent by the client after it has finished sending all
    /// configuration requests (breakpoints, exception filters, etc.)
    /// The adapter can use this to start the debuggee if needed.
    fn handle_configuration_done(&self, _request: &Request) -> Result<Value> {
        // Verify we're in initialized state
        let initialized = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
        if !*initialized {
            anyhow::bail!("configurationDone received before initialized");
        }

        // configurationDone has no response body per spec
        Ok(serde_json::Value::Null)
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

    /// Handle inlineValues request
    ///
    /// Returns inline value text hints for scalar variables in the specified range.
    fn handle_inline_values(&self, request: &Request) -> Result<Value> {
        let args: InlineValuesArguments = request
            .arguments
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing arguments for inlineValues"))
            .and_then(|v| serde_json::from_value(v.clone()).context("Invalid arguments"))?;

        let source_path =
            args.source.path.ok_or_else(|| anyhow::anyhow!("inlineValues requires source.path"))?;

        if args.start_line <= 0 || args.end_line <= 0 {
            anyhow::bail!("inlineValues requires positive startLine/endLine");
        }

        let start_line = args.start_line.min(args.end_line);
        let end_line = args.end_line.max(args.start_line);
        let content = std::fs::read_to_string(&source_path)
            .with_context(|| format!("Failed to read source file: {}", source_path))?;

        let inline_values = collect_inline_values(&content, start_line, end_line);
        let body = InlineValuesResponseBody { inline_values };

        serde_json::to_value(&body).context("Failed to serialize inlineValues response")
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
    use crate::protocol::InlineValuesResponseBody;
    use perl_tdd_support::must;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a temp file with valid Perl code for testing breakpoints.
    /// NOTE: Avoid sub immediately followed by for loop (triggers parser hang - known issue)
    fn create_test_perl_file() -> (NamedTempFile, String) {
        let mut file = must(NamedTempFile::with_suffix(".pl"));
        let perl_code = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
my $y = 2;
my $z = $x + $y;

if ($x > 0) {
    print "positive\n";
}

my @arr = (1, 2, 3);
while (my $item = shift @arr) {
    my $doubled = $item * 2;
    print "$doubled\n";
}

sub process {
    my ($value) = @_;
    my $result = $value * 2;
    return $result;
}

print "done\n";
my $final = process($x);
print "result: $final\n";
"#;
        must(file.write_all(perl_code.as_bytes()));
        must(file.flush());
        let path = file.path().to_string_lossy().to_string();
        (file, path)
    }

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = DapDispatcher::new();
        let breakpoints = dispatcher.breakpoint_store.get_breakpoints("/test.pl");
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_handle_initialize() -> Result<(), Box<dyn std::error::Error>> {
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
        let capabilities: Capabilities =
            serde_json::from_value(response.body.ok_or("Expected body")?)?;
        assert_eq!(capabilities.supports_configuration_done_request, Some(true));
        assert_eq!(capabilities.supports_evaluate_for_hovers, Some(true));
        Ok(())
    }

    #[test]
    fn test_handle_set_breakpoints() -> Result<(), Box<dyn std::error::Error>> {
        let (_file, source_path) = create_test_perl_file();
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": {
                    "path": source_path,
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
            serde_json::from_value(response.body.ok_or("Expected body")?)?;
        assert_eq!(body.breakpoints.len(), 2);
        assert_eq!(body.breakpoints[0].line, 10);
        assert_eq!(body.breakpoints[1].line, 25);
        assert!(body.breakpoints[0].verified);
        assert!(body.breakpoints[1].verified);
        Ok(())
    }

    #[test]
    fn test_handle_set_breakpoints_replace_semantics() -> Result<(), Box<dyn std::error::Error>> {
        let (_file, source_path) = create_test_perl_file();
        let dispatcher = DapDispatcher::new();

        // Set initial breakpoints
        let request1 = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": { "path": &source_path },
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
                "source": { "path": &source_path },
                "breakpoints": [{ "line": 20 }, { "line": 26 }]
            })),
        };
        let response = dispatcher.dispatch(&request2);

        assert!(response.success);
        let body: SetBreakpointsResponseBody =
            serde_json::from_value(response.body.ok_or("Expected body")?)?;
        assert_eq!(body.breakpoints.len(), 2);
        assert_eq!(body.breakpoints[0].line, 20);
        assert_eq!(body.breakpoints[1].line, 26);
        Ok(())
    }

    #[test]
    fn test_handle_set_breakpoints_preserves_order() -> Result<(), Box<dyn std::error::Error>> {
        let (_file, source_path) = create_test_perl_file();
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "setBreakpoints".to_string(),
            arguments: Some(json!({
                "source": { "path": &source_path },
                // Use lines within our 27-line test file, but out of order
                "breakpoints": [
                    { "line": 25 },
                    { "line": 10 },
                    { "line": 15 }
                ]
            })),
        };

        let response = dispatcher.dispatch(&request);

        assert!(response.success);
        let body: SetBreakpointsResponseBody =
            serde_json::from_value(response.body.ok_or("Expected body")?)?;

        // Order must match request
        assert_eq!(body.breakpoints[0].line, 25);
        assert_eq!(body.breakpoints[1].line, 10);
        assert_eq!(body.breakpoints[2].line, 15);
        Ok(())
    }

    #[test]
    fn test_handle_unknown_command() -> Result<(), Box<dyn std::error::Error>> {
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
        assert!(
            response.message.ok_or("Expected message")?.contains("Unknown command: unknownCommand")
        );
        Ok(())
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
    fn test_handle_inline_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = NamedTempFile::with_suffix(".pl")?;
        let perl_code = "my $x = 1;\nmy $y = $x + 2;\n";
        file.write_all(perl_code.as_bytes())?;
        file.flush()?;
        let path = file.path().to_string_lossy().to_string();

        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 3,
            msg_type: "request".to_string(),
            command: "inlineValues".to_string(),
            arguments: Some(json!({
                "source": { "path": path },
                "startLine": 1,
                "endLine": 2
            })),
        };

        let response = dispatcher.dispatch(&request);
        assert!(response.success);

        let body: InlineValuesResponseBody =
            serde_json::from_value(response.body.ok_or("Expected body")?)?;
        assert!(body.inline_values.iter().any(|v| v.text.contains("$x")));
        assert!(body.inline_values.iter().any(|v| v.text.contains("$y")));
        Ok(())
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

    #[test]
    fn test_initialize_emits_initialized_event() {
        let dispatcher = DapDispatcher::new();
        let request = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: Some(json!({
                "clientId": "vscode",
                "adapterId": "perl-rs",
            })),
        };

        let result = dispatcher.dispatch_with_events(&request);

        // Response should be successful
        assert!(result.response.success);
        assert_eq!(result.response.command, "initialize");

        // Should emit initialized event
        assert_eq!(result.events.len(), 1);
        let event = &result.events[0];
        assert_eq!(event.event, "initialized");
        assert_eq!(event.msg_type, "event");
        assert!(event.body.is_none()); // initialized event has no body
    }

    #[test]
    fn test_configuration_done_after_initialize() {
        let dispatcher = DapDispatcher::new();

        // First, initialize
        let init_request = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: None,
        };
        let init_result = dispatcher.dispatch_with_events(&init_request);
        assert!(init_result.response.success);

        // Then, configurationDone should succeed
        let config_done_request = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "configurationDone".to_string(),
            arguments: None,
        };
        let response = dispatcher.dispatch(&config_done_request);

        assert!(response.success);
        assert_eq!(response.command, "configurationDone");
    }

    #[test]
    fn test_configuration_done_before_initialize_fails() -> Result<(), Box<dyn std::error::Error>> {
        let dispatcher = DapDispatcher::new();

        // configurationDone without initialize should fail
        let request = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "configurationDone".to_string(),
            arguments: None,
        };
        let response = dispatcher.dispatch(&request);

        assert!(!response.success);
        assert!(response.message.is_some());
        assert!(response.message.ok_or("Expected message")?.contains("before initialized"));
        Ok(())
    }

    #[test]
    fn test_event_sequence_numbers() {
        let dispatcher = DapDispatcher::new();

        // Multiple initializations should have incrementing event seq numbers
        let request1 = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: None,
        };
        let result1 = dispatcher.dispatch_with_events(&request1);

        let request2 = Request {
            seq: 2,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: None,
        };
        let result2 = dispatcher.dispatch_with_events(&request2);

        // Event sequence numbers should increment
        assert_eq!(result1.events[0].seq, 1);
        assert_eq!(result2.events[0].seq, 2);
    }

    #[test]
    fn test_failed_initialize_no_event() {
        let dispatcher = DapDispatcher::new();

        // Invalid arguments that cause parsing to fail
        let request = Request {
            seq: 1,
            msg_type: "request".to_string(),
            command: "initialize".to_string(),
            arguments: Some(json!({
                "adapterId": 123 // Should be string, not number
            })),
        };

        let result = dispatcher.dispatch_with_events(&request);

        // If initialization fails, no event should be emitted
        if !result.response.success {
            assert!(result.events.is_empty());
        }
    }
}
