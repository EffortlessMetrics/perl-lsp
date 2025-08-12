//! Debug Adapter Protocol (DAP) implementation for Perl debugging
//!
//! This module provides a DAP server that integrates with Perl's built-in debugger
//! to enable debugging support in VSCode and other DAP-compatible editors.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

/// DAP server that handles debug sessions
pub struct DebugAdapter {
    /// Sequence number for messages
    seq: Arc<Mutex<i64>>,
    /// Active debug session
    session: Arc<Mutex<Option<DebugSession>>>,
    /// Breakpoints indexed by file path
    breakpoints: Arc<Mutex<HashMap<String, Vec<Breakpoint>>>>,
    /// Thread ID counter
    thread_counter: Arc<Mutex<i32>>,
    /// Output channel for sending events to client
    event_sender: Option<Sender<DapMessage>>,
}

/// Active debug session
struct DebugSession {
    /// Perl debugger process
    process: Child,
    /// Current execution state
    state: DebugState,
    /// Stack frames
    stack_frames: Vec<StackFrame>,
    /// Variables in current scope
    variables: HashMap<i32, Vec<Variable>>,
    /// Thread ID
    thread_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum DebugState {
    Running,
    Stopped,
    Terminated,
}

/// DAP message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DapMessage {
    #[serde(rename = "request")]
    Request {
        seq: i64,
        command: String,
        arguments: Option<Value>,
    },
    #[serde(rename = "response")]
    Response {
        seq: i64,
        request_seq: i64,
        success: bool,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    #[serde(rename = "event")]
    Event {
        seq: i64,
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
}

/// Breakpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Breakpoint {
    id: i32,
    verified: bool,
    line: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Stack frame information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StackFrame {
    id: i32,
    name: String,
    source: Source,
    line: i32,
    column: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<i32>,
}

/// Source file information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_reference: Option<i32>,
}

/// Variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Variable {
    name: String,
    value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<String>,
    variables_reference: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    named_variables: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    indexed_variables: Option<i32>,
}

impl Default for DebugAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugAdapter {
    /// Create a new debug adapter
    pub fn new() -> Self {
        Self {
            seq: Arc::new(Mutex::new(0)),
            session: Arc::new(Mutex::new(None)),
            breakpoints: Arc::new(Mutex::new(HashMap::new())),
            thread_counter: Arc::new(Mutex::new(0)),
            event_sender: None,
        }
    }

    /// Run the debug adapter server
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        // Create channel for events
        let (tx, rx) = channel::<DapMessage>();
        self.event_sender = Some(tx.clone());

        // Start event handler thread
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                if let Ok(json) = serde_json::to_string(&msg) {
                    let content_length = json.len();
                    let output = format!("Content-Length: {}\r\n\r\n{}", content_length, json);
                    print!("{}", output);
                    io::stdout().flush().unwrap();
                }
            }
        });

        // Read messages from stdin
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        loop {
            // Read headers
            let mut headers = HashMap::new();
            while let Some(Ok(line)) = lines.next() {
                if line.is_empty() {
                    break;
                }
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim();
                    let value = line[colon_pos + 1..].trim();
                    headers.insert(key.to_string(), value.to_string());
                }
            }

            // Read content
            if let Some(content_length) = headers.get("Content-Length") {
                if let Ok(length) = content_length.parse::<usize>() {
                    let mut buffer = vec![0; length];
                    let handle = lines.by_ref().take(length);
                    let mut bytes_read = 0;

                    // Read the JSON content
                    for line in handle {
                        if let Ok(line) = line {
                            let line_bytes = line.as_bytes();
                            buffer[bytes_read..bytes_read + line_bytes.len()]
                                .copy_from_slice(line_bytes);
                            bytes_read += line_bytes.len();
                            if bytes_read >= length {
                                break;
                            }
                        }
                    }

                    // Parse and handle the message
                    if let Ok(msg) = serde_json::from_slice::<DapMessage>(&buffer[..bytes_read]) {
                        if let DapMessage::Request {
                            seq,
                            command,
                            arguments,
                        } = msg
                        {
                            let response = self.handle_request(seq, &command, arguments);
                            if let Ok(json) = serde_json::to_string(&response) {
                                let content_length = json.len();
                                writeln!(
                                    stdout,
                                    "Content-Length: {}\r\n\r\n{}",
                                    content_length, json
                                )?;
                                stdout.flush()?;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Handle a DAP request
    fn handle_request(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        eprintln!("DAP request: {} {:?}", command, arguments);

        let seq = self.next_seq();

        match command {
            "initialize" => self.handle_initialize(seq, request_seq, arguments),
            "launch" => self.handle_launch(seq, request_seq, arguments),
            "attach" => self.handle_attach(seq, request_seq, arguments),
            "disconnect" => self.handle_disconnect(seq, request_seq, arguments),
            "setBreakpoints" => self.handle_set_breakpoints(seq, request_seq, arguments),
            "configurationDone" => self.handle_configuration_done(seq, request_seq),
            "threads" => self.handle_threads(seq, request_seq),
            "stackTrace" => self.handle_stack_trace(seq, request_seq, arguments),
            "scopes" => self.handle_scopes(seq, request_seq, arguments),
            "variables" => self.handle_variables(seq, request_seq, arguments),
            "continue" => self.handle_continue(seq, request_seq, arguments),
            "next" => self.handle_next(seq, request_seq, arguments),
            "stepIn" => self.handle_step_in(seq, request_seq, arguments),
            "stepOut" => self.handle_step_out(seq, request_seq, arguments),
            "pause" => self.handle_pause(seq, request_seq, arguments),
            "evaluate" => self.handle_evaluate(seq, request_seq, arguments),
            _ => DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: command.to_string(),
                body: None,
                message: Some(format!("Unknown command: {}", command)),
            },
        }
    }

    /// Get next sequence number
    fn next_seq(&self) -> i64 {
        let mut seq = self.seq.lock().unwrap();
        *seq += 1;
        *seq
    }

    /// Send an event to the client
    fn send_event(&self, event: &str, body: Option<Value>) {
        if let Some(ref sender) = self.event_sender {
            let seq = self.next_seq();
            let msg = DapMessage::Event {
                seq,
                event: event.to_string(),
                body,
            };
            let _ = sender.send(msg);
        }
    }

    /// Handle initialize request
    fn handle_initialize(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        let capabilities = json!({
            "supportsConfigurationDoneRequest": true,
            "supportsFunctionBreakpoints": false,
            "supportsConditionalBreakpoints": true,
            "supportsHitConditionalBreakpoints": false,
            "supportsEvaluateForHovers": true,
            "supportsStepBack": false,
            "supportsSetVariable": true,
            "supportsRestartFrame": false,
            "supportsGotoTargetsRequest": false,
            "supportsStepInTargetsRequest": false,
            "supportsCompletionsRequest": false,
            "supportsModulesRequest": false,
            "supportsRestartRequest": false,
            "supportsExceptionOptions": false,
            "supportsValueFormattingOptions": true,
            "supportsExceptionInfoRequest": false,
            "supportTerminateDebuggee": true,
            "supportsDelayedStackTraceLoading": false,
            "supportsLoadedSourcesRequest": false,
            "supportsLogPoints": false,
            "supportsTerminateThreadsRequest": false,
            "supportsSetExpression": false,
            "supportsTerminateRequest": true,
            "supportsDataBreakpoints": false,
            "supportsReadMemoryRequest": false,
            "supportsDisassembleRequest": false,
            "supportsCancelRequest": false,
            "supportsBreakpointLocationsRequest": false,
            "supportsClipboardContext": false,
            "supportsSteppingGranularity": false,
            "supportsInstructionBreakpoints": false,
            "supportsExceptionFilterOptions": false
        });

        // Send initialized event
        self.send_event("initialized", None);

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "initialize".to_string(),
            body: Some(capabilities),
            message: None,
        }
    }

    /// Handle launch request
    fn handle_launch(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        if let Some(args) = arguments {
            let program = args.get("program").and_then(|p| p.as_str()).unwrap_or("");

            let perl_args = args
                .get("args")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let stop_on_entry = args
                .get("stopOnEntry")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);

            // Launch Perl debugger
            match self.launch_debugger(program, perl_args, stop_on_entry) {
                Ok(thread_id) => {
                    // Send stopped event if stop on entry
                    if stop_on_entry {
                        self.send_event(
                            "stopped",
                            Some(json!({
                                "reason": "entry",
                                "threadId": thread_id,
                                "allThreadsStopped": true
                            })),
                        );
                    }

                    DapMessage::Response {
                        seq,
                        request_seq,
                        success: true,
                        command: "launch".to_string(),
                        body: None,
                        message: None,
                    }
                }
                Err(e) => DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "launch".to_string(),
                    body: None,
                    message: Some(format!("Failed to launch debugger: {}", e)),
                },
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "launch".to_string(),
                body: None,
                message: Some("Missing launch arguments".to_string()),
            }
        }
    }

    /// Launch the Perl debugger
    fn launch_debugger(
        &mut self,
        program: &str,
        args: Vec<String>,
        stop_on_entry: bool,
    ) -> Result<i32, String> {
        let mut cmd = Command::new("perl");
        cmd.arg("-d");

        // Add debugger initialization
        if stop_on_entry {
            cmd.arg("-e").arg("$DB::single=1");
        }

        cmd.arg(program);
        cmd.args(&args);

        // Set up pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                let thread_id = {
                    let mut counter = self.thread_counter.lock().unwrap();
                    *counter += 1;
                    *counter
                };

                let session = DebugSession {
                    process: child,
                    state: DebugState::Running,
                    stack_frames: Vec::new(),
                    variables: HashMap::new(),
                    thread_id,
                };

                *self.session.lock().unwrap() = Some(session);

                // Start output reader thread
                self.start_output_reader();

                Ok(thread_id)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Start thread to read debugger output
    fn start_output_reader(&self) {
        // TODO: Implement output reader that parses Perl debugger output
        // and sends appropriate events (stopped, output, etc.)
    }

    /// Handle attach request
    fn handle_attach(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // Not implemented yet
        DapMessage::Response {
            seq,
            request_seq,
            success: false,
            command: "attach".to_string(),
            body: None,
            message: Some("Attach not yet implemented".to_string()),
        }
    }

    /// Handle disconnect request
    fn handle_disconnect(
        &mut self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        // Terminate the debug session
        if let Some(mut session) = self.session.lock().unwrap().take() {
            let _ = session.process.kill();
            session.state = DebugState::Terminated;
        }

        // Send terminated event
        self.send_event("terminated", None);

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "disconnect".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle setBreakpoints request
    fn handle_set_breakpoints(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        if let Some(args) = arguments {
            let source_path = args
                .get("source")
                .and_then(|s| s.get("path"))
                .and_then(|p| p.as_str())
                .unwrap_or("");

            let empty_vec = Vec::new();
            let breakpoint_requests = args
                .get("breakpoints")
                .and_then(|b| b.as_array())
                .unwrap_or(&empty_vec);

            let mut verified_breakpoints = Vec::new();
            let mut bp_id = 1;

            for bp_req in breakpoint_requests {
                let line = bp_req.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as i32;

                let condition = bp_req
                    .get("condition")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());

                // TODO: Actually set breakpoint in Perl debugger
                let breakpoint = Breakpoint {
                    id: bp_id,
                    verified: true,
                    line,
                    column: None,
                    message: condition,
                };

                verified_breakpoints.push(breakpoint.clone());
                bp_id += 1;
            }

            // Store breakpoints
            self.breakpoints
                .lock()
                .unwrap()
                .insert(source_path.to_string(), verified_breakpoints.clone());

            DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "setBreakpoints".to_string(),
                body: Some(json!({
                    "breakpoints": verified_breakpoints
                })),
                message: None,
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setBreakpoints".to_string(),
                body: None,
                message: Some("Missing arguments".to_string()),
            }
        }
    }

    /// Handle configurationDone request
    fn handle_configuration_done(&self, seq: i64, request_seq: i64) -> DapMessage {
        // Continue execution after configuration
        // TODO: Send continue command to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "configurationDone".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle threads request
    fn handle_threads(&self, seq: i64, request_seq: i64) -> DapMessage {
        let threads = if let Some(ref session) = *self.session.lock().unwrap() {
            vec![json!({
                "id": session.thread_id,
                "name": "Main Thread"
            })]
        } else {
            vec![]
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "threads".to_string(),
            body: Some(json!({
                "threads": threads
            })),
            message: None,
        }
    }

    /// Handle stackTrace request
    fn handle_stack_trace(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        let stack_frames = if let Some(ref session) = *self.session.lock().unwrap() {
            session.stack_frames.clone()
        } else {
            vec![]
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stackTrace".to_string(),
            body: Some(json!({
                "stackFrames": stack_frames,
                "totalFrames": stack_frames.len()
            })),
            message: None,
        }
    }

    /// Handle scopes request
    fn handle_scopes(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        if let Some(args) = arguments {
            let frame_id = args.get("frameId").and_then(|f| f.as_i64()).unwrap_or(0) as i32;

            // Return local scope
            let scopes = vec![json!({
                "name": "Local",
                "presentationHint": "locals",
                "variablesReference": frame_id,
                "expensive": false
            })];

            DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "scopes".to_string(),
                body: Some(json!({
                    "scopes": scopes
                })),
                message: None,
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "scopes".to_string(),
                body: None,
                message: Some("Missing frameId".to_string()),
            }
        }
    }

    /// Handle variables request
    fn handle_variables(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        if let Some(args) = arguments {
            let variables_ref = args
                .get("variablesReference")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let variables = if let Some(ref session) = *self.session.lock().unwrap() {
                session
                    .variables
                    .get(&variables_ref)
                    .cloned()
                    .unwrap_or_default()
            } else {
                vec![]
            };

            DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "variables".to_string(),
                body: Some(json!({
                    "variables": variables
                })),
                message: None,
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "variables".to_string(),
                body: None,
                message: Some("Missing variablesReference".to_string()),
            }
        }
    }

    /// Handle continue request
    fn handle_continue(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // TODO: Send continue command to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "continue".to_string(),
            body: Some(json!({
                "allThreadsContinued": true
            })),
            message: None,
        }
    }

    /// Handle next request
    fn handle_next(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // TODO: Send next command to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "next".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle stepIn request
    fn handle_step_in(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // TODO: Send step command to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stepIn".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle stepOut request
    fn handle_step_out(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // TODO: Send return command to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stepOut".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle pause request
    fn handle_pause(&self, seq: i64, request_seq: i64, _arguments: Option<Value>) -> DapMessage {
        // TODO: Send interrupt signal to Perl debugger

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "pause".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle evaluate request
    fn handle_evaluate(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        if let Some(args) = arguments {
            let expression = args
                .get("expression")
                .and_then(|e| e.as_str())
                .unwrap_or("");

            // TODO: Evaluate expression in Perl debugger
            let result = format!("({})", expression);

            DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "evaluate".to_string(),
                body: Some(json!({
                    "result": result,
                    "type": "string",
                    "variablesReference": 0
                })),
                message: None,
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some("Missing expression".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_adapter_creation() {
        let adapter = DebugAdapter::new();
        assert!(adapter.session.lock().unwrap().is_none());
        assert!(adapter.breakpoints.lock().unwrap().is_empty());
    }

    #[test]
    fn test_sequence_numbers() {
        let adapter = DebugAdapter::new();
        assert_eq!(adapter.next_seq(), 1);
        assert_eq!(adapter.next_seq(), 2);
        assert_eq!(adapter.next_seq(), 3);
    }

    #[test]
    fn test_initialize_response() {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(1, "initialize", None);

        match response {
            DapMessage::Response {
                success,
                command,
                body,
                ..
            } => {
                assert!(success);
                assert_eq!(command, "initialize");
                assert!(body.is_some());
            }
            _ => panic!("Expected response"),
        }
    }
}
