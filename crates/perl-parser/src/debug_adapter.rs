//! Debug Adapter Protocol (DAP) implementation for Perl debugging
//!
//! This module provides a DAP server that integrates with Perl's built-in debugger
//! to enable debugging support in VSCode and other DAP-compatible editors.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use lazy_static::lazy_static;
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use regex::Regex;

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
    Request { seq: i64, command: String, arguments: Option<Value> },
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

    /// Set the event sender (primarily for testing)
    pub fn set_event_sender(&mut self, sender: Sender<DapMessage>) {
        self.event_sender = Some(sender);
    }

    /// Run the debug adapter server
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        // Create channel for events
        let (tx, rx) = channel::<DapMessage>();
        self.event_sender = Some(tx.clone());

        // Start event handler thread with enhanced error handling
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match serde_json::to_string(&msg) {
                    Ok(json) => {
                        let content_length = json.len();
                        let output = format!("Content-Length: {}\r\n\r\n{}", content_length, json);
                        print!("{}", output);
                        if let Err(e) = io::stdout().flush() {
                            eprintln!("Failed to flush stdout in event handler: {}", e);
                            // Continue trying to process more events
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to serialize DAP message: {} - {:#?}", e, msg);
                        // Continue processing other messages
                    }
                }
            }
            eprintln!("Event handler thread terminating - channel closed");
        });

        // Read messages from stdin with proper DAP protocol handling
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            // Read headers
            let mut headers = HashMap::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => return Ok(()), // EOF
                    Ok(_) => {
                        let line = line.trim_end();
                        if line.is_empty() {
                            break; // End of headers
                        }
                        if let Some(colon_pos) = line.find(':') {
                            let key = line[..colon_pos].trim();
                            let value = line[colon_pos + 1..].trim();
                            headers.insert(key.to_string(), value.to_string());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            // Read content body based on Content-Length
            if let Some(content_length) = headers.get("Content-Length") {
                if let Ok(length) = content_length.parse::<usize>() {
                    let mut buffer = vec![0u8; length];
                    reader.read_exact(&mut buffer)?;

                    // Parse and handle the message
                    if let Ok(msg) = serde_json::from_slice::<DapMessage>(&buffer) {
                        if let DapMessage::Request { seq, command, arguments } = msg {
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
                    } else {
                        eprintln!(
                            "Failed to parse DAP message: {}",
                            String::from_utf8_lossy(&buffer)
                        );
                    }
                } else {
                    eprintln!("Invalid Content-Length header: {}", content_length);
                }
            }
        }
    }

    /// Handle a DAP request
    pub fn handle_request(
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
            let msg = DapMessage::Event { seq, event: event.to_string(), body };
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
                    arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let stop_on_entry = args.get("stopOnEntry").and_then(|s| s.as_bool()).unwrap_or(false);

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

        // Perl debugger stops on the first line by default
        let _ = stop_on_entry; // currently unused
        cmd.arg(program);
        cmd.args(&args);

        // Set up pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                // Validate that the program file exists before proceeding
                use std::path::Path;
                if !Path::new(program).exists() {
                    // Kill the child process and return error
                    let _ = child.kill();
                    return Err(format!("Program file does not exist: {}", program));
                }

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

    /// Start thread to read debugger output with enhanced error recovery
    fn start_output_reader(&self) {
        let session = self.session.clone();
        let seq = self.seq.clone();
        let sender = self.event_sender.clone();

        thread::spawn(move || {
            // Take stdout handle
            let stdout = {
                let mut guard = session.lock().unwrap();
                guard.as_mut().and_then(|s| s.process.stdout.take())
            };

            let Some(stdout) = stdout else {
                eprintln!(
                    "No stdout handle available for Perl debugger - output reader thread exiting"
                );
                // Send termination event
                if let Some(ref sender) = sender {
                    let mut seq_lock = match seq.lock() {
                        Ok(lock) => lock,
                        Err(poisoned) => {
                            eprintln!("Sequence lock poisoned, recovering");
                            poisoned.into_inner()
                        }
                    };
                    *seq_lock += 1;
                    let _ = sender.send(DapMessage::Event {
                        seq: *seq_lock,
                        event: "terminated".to_string(),
                        body: Some(json!({"reason": "no_stdout"})),
                    });
                }
                return;
            };

            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            lazy_static! {
                // Enhanced regex patterns for more robust Perl debugger output parsing
                static ref CONTEXT_RE: Regex = Regex::new(
                    r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)]+):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+)(?:\))?:(?P<line2>\d+):?)"
                ).unwrap();
                static ref PROMPT_RE: Regex = Regex::new(r"^\s*DB<?\d*>?\s*$").unwrap();
                static ref STACK_FRAME_RE: Regex = Regex::new(
                    r"^\s*#?\s*(?P<frame>\d+)?\s+(?P<func>[A-Za-z_][\w:]*+?)(?:\s+called)?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)"
                ).unwrap();
                static ref VARIABLE_RE: Regex = Regex::new(
                    r"^\s*(?P<name>[\$\@\%][\w:]+)\s*=\s*(?P<value>.*?)$"
                ).unwrap();
                static ref ERROR_RE: Regex = Regex::new(
                    r"^(?:.*?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)|Syntax error|Can't locate|Global symbol).*$"
                ).unwrap();
            }

            let mut current_file = String::new();
            let mut current_func = String::new();
            let mut current_line = 0;
            let mut _debugger_ready = false;

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        eprintln!("Perl debugger process terminated");
                        break;
                    }
                    Ok(_) => {
                        let text = line.trim_end().to_string();
                        eprintln!("Debugger output: {}", text); // Debug logging

                        // Send all output to client with error handling
                        if let Some(ref sender) = sender {
                            match seq.lock() {
                                Ok(mut seq_lock) => {
                                    *seq_lock += 1;
                                    if sender
                                        .send(DapMessage::Event {
                                            seq: *seq_lock,
                                            event: "output".to_string(),
                                            body: Some(json!({
                                                "category": "stdout",
                                                "output": format!("{}\n", text)
                                            })),
                                        })
                                        .is_err()
                                    {
                                        eprintln!(
                                            "Failed to send output event - client may have disconnected"
                                        );
                                        break; // Exit the loop if client is gone
                                    }
                                }
                                Err(poisoned) => {
                                    eprintln!(
                                        "Sequence lock poisoned in output reader, attempting recovery"
                                    );
                                    let mut seq_lock = poisoned.into_inner();
                                    *seq_lock += 1;
                                    let _ = sender.send(DapMessage::Event {
                                        seq: *seq_lock,
                                        event: "output".to_string(),
                                        body: Some(json!({
                                            "category": "stdout",
                                            "output": format!("{}\n", text)
                                        })),
                                    });
                                }
                            }
                        }

                        // Enhanced context information parsing with multiple patterns
                        let mut context_updated = false;

                        // Try main context pattern
                        if let Some(caps) = CONTEXT_RE.captures(&text) {
                            if let Some(func) = caps.name("func") {
                                current_func = func.as_str().to_string();
                                context_updated = true;
                            }
                            if let Some(file) = caps.name("file").or_else(|| caps.name("file2")) {
                                current_file = file.as_str().to_string();
                                context_updated = true;
                            }
                            if let Some(line_num) = caps.name("line").or_else(|| caps.name("line2"))
                            {
                                current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                                context_updated = true;
                            }
                        }

                        // Try stack frame pattern as fallback
                        if !context_updated {
                            if let Some(caps) = STACK_FRAME_RE.captures(&text) {
                                if let Some(func) = caps.name("func") {
                                    current_func = func.as_str().to_string();
                                }
                                if let Some(file) = caps.name("file") {
                                    current_file = file.as_str().to_string();
                                }
                                if let Some(line_num) = caps.name("line") {
                                    current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                                }
                                context_updated = true;
                            }
                        }

                        // Check for errors that might provide location info
                        if !context_updated {
                            if let Some(caps) = ERROR_RE.captures(&text) {
                                if let Some(file) = caps.name("file") {
                                    current_file = file.as_str().to_string();
                                }
                                if let Some(line_num) = caps.name("line") {
                                    current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                                }
                                context_updated = true;

                                // Send error event to client
                                if let Some(ref sender) = sender {
                                    let mut seq_lock = seq.lock().unwrap();
                                    *seq_lock += 1;
                                    let _ = sender.send(DapMessage::Event {
                                        seq: *seq_lock,
                                        event: "output".to_string(),
                                        body: Some(json!({
                                            "category": "stderr",
                                            "output": format!("Error: {}\n", text)
                                        })),
                                    });
                                }
                            }
                        }

                        if context_updated {
                            continue;
                        }

                        // Detect debugger prompt (stopped state) with enhanced pattern matching
                        if PROMPT_RE.is_match(&text)
                            || text.trim().starts_with("DB<")
                            || text.trim().starts_with("  DB<")
                        {
                            _debugger_ready = true;
                            let thread_id = {
                                let mut guard = session.lock().unwrap();
                                if let Some(ref mut s) = *guard {
                                    // Create stack frame with enhanced context validation
                                    if !current_file.is_empty() && current_line > 0 {
                                        let frame = StackFrame {
                                            id: 1,
                                            name: if current_func.is_empty() {
                                                "main".to_string()
                                            } else {
                                                current_func.clone()
                                            },
                                            source: Source {
                                                name: Some(
                                                    std::path::Path::new(&current_file)
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or(&current_file)
                                                        .to_string(),
                                                ),
                                                path: current_file.clone(),
                                                source_reference: None,
                                            },
                                            line: current_line,
                                            column: 1,
                                            end_line: None,
                                            end_column: None,
                                        };
                                        s.stack_frames = vec![frame];
                                    } else {
                                        // Provide a fallback frame for when we don't have perfect context
                                        let frame = StackFrame {
                                            id: 1,
                                            name: "main".to_string(),
                                            source: Source {
                                                name: Some("<unknown>".to_string()),
                                                path: "<unknown>".to_string(),
                                                source_reference: None,
                                            },
                                            line: 1,
                                            column: 1,
                                            end_line: None,
                                            end_column: None,
                                        };
                                        s.stack_frames = vec![frame];
                                    }
                                    s.state = DebugState::Stopped;
                                    s.thread_id
                                } else {
                                    continue;
                                }
                            };

                            // Send stopped event with robust error handling
                            if let Some(ref sender) = sender {
                                match seq.lock() {
                                    Ok(mut seq_lock) => {
                                        *seq_lock += 1;
                                        if sender
                                            .send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "stopped".to_string(),
                                                body: Some(json!({
                                                    "reason": "step",
                                                    "threadId": thread_id,
                                                    "allThreadsStopped": true
                                                })),
                                            })
                                            .is_err()
                                        {
                                            eprintln!(
                                                "Failed to send stopped event - client disconnected"
                                            );
                                            return; // Exit thread
                                        }
                                    }
                                    Err(poisoned) => {
                                        eprintln!(
                                            "Sequence lock poisoned when sending stopped event, recovering"
                                        );
                                        let mut seq_lock = poisoned.into_inner();
                                        *seq_lock += 1;
                                        let _ = sender.send(DapMessage::Event {
                                            seq: *seq_lock,
                                            event: "stopped".to_string(),
                                            body: Some(json!({
                                                "reason": "step",
                                                "threadId": thread_id,
                                                "allThreadsStopped": true
                                            })),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from debugger: {}", e);
                        // Send termination event before exiting
                        if let Some(ref sender) = sender {
                            match seq.lock() {
                                Ok(mut seq_lock) => {
                                    *seq_lock += 1;
                                    let _ = sender.send(DapMessage::Event {
                                        seq: *seq_lock,
                                        event: "terminated".to_string(),
                                        body: Some(
                                            json!({"reason": "read_error", "error": e.to_string()}),
                                        ),
                                    });
                                }
                                Err(poisoned) => {
                                    let mut seq_lock = poisoned.into_inner();
                                    *seq_lock += 1;
                                    let _ = sender.send(DapMessage::Event {
                                        seq: *seq_lock,
                                        event: "terminated".to_string(),
                                        body: Some(
                                            json!({"reason": "read_error", "error": e.to_string()}),
                                        ),
                                    });
                                }
                            }
                        }
                        break;
                    }
                }
            }
        });
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

            if source_path.is_empty() {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "setBreakpoints".to_string(),
                    body: None,
                    message: Some("Missing source path".to_string()),
                };
            }

            let empty_vec = Vec::new();
            let breakpoint_requests =
                args.get("breakpoints").and_then(|b| b.as_array()).unwrap_or(&empty_vec);

            let mut verified_breakpoints = Vec::new();
            let mut bp_id = 1;
            let mut has_session = false;

            // First, clear existing breakpoints for this file
            if let Some(ref mut session) = *self.session.lock().unwrap() {
                has_session = true;
                if let Some(stdin) = session.process.stdin.as_mut() {
                    // Clear breakpoints in file (Perl debugger 'B' command)
                    let _ = stdin.write_all(b"B\n");
                    let _ = stdin.flush();
                }
            }

            // Set new breakpoints
            for bp_req in breakpoint_requests {
                let line = bp_req.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as i32;

                if line <= 0 {
                    // Invalid line number
                    let breakpoint = Breakpoint {
                        id: bp_id,
                        verified: false,
                        line,
                        column: None,
                        message: Some("Invalid line number".to_string()),
                    };
                    verified_breakpoints.push(breakpoint);
                    bp_id += 1;
                    continue;
                }

                let condition = bp_req.get("condition").and_then(|c| c.as_str());
                let mut success = false;

                if let Some(ref mut session) = *self.session.lock().unwrap() {
                    if let Some(stdin) = session.process.stdin.as_mut() {
                        let cmd = if let Some(cond) = condition {
                            format!("b {} {}\n", line, cond)
                        } else {
                            format!("b {}\n", line)
                        };

                        success = stdin.write_all(cmd.as_bytes()).is_ok() && stdin.flush().is_ok();
                    }
                }

                let breakpoint = Breakpoint {
                    id: bp_id,
                    verified: success && has_session,
                    line,
                    column: None,
                    message: if !success && has_session {
                        Some("Failed to set breakpoint".to_string())
                    } else if !has_session {
                        Some("No active debug session".to_string())
                    } else {
                        condition.map(|c| c.to_string())
                    },
                };

                verified_breakpoints.push(breakpoint);
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
        // Send initial command to get the debugger started
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Send initial 'l' command to list current location
                let _ = stdin.write_all(b"l\n");
                let _ = stdin.flush();
            }
        }

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
            let variables_ref =
                args.get("variablesReference").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            let variables = if let Some(ref session) = *self.session.lock().unwrap() {
                // Try to get cached variables first
                if let Some(vars) = session.variables.get(&variables_ref) {
                    vars.clone()
                } else {
                    // Generate some basic variables for the local scope
                    if variables_ref == 1 {
                        // Frame 1 local variables - send command to get them
                        if let Some(ref mut session) = *self.session.lock().unwrap() {
                            if let Some(stdin) = session.process.stdin.as_mut() {
                                let _ = stdin.write_all(b"V\n"); // Show local variables
                                let _ = stdin.flush();
                            }
                        }

                        // Return placeholder variables for now
                        vec![
                            Variable {
                                name: "@_".to_string(),
                                value: "()".to_string(),
                                type_: Some("array".to_string()),
                                variables_reference: 0,
                                named_variables: None,
                                indexed_variables: None,
                            },
                            Variable {
                                name: "$_".to_string(),
                                value: "undef".to_string(),
                                type_: Some("scalar".to_string()),
                                variables_reference: 0,
                                named_variables: None,
                                indexed_variables: None,
                            },
                        ]
                    } else {
                        vec![]
                    }
                }
            } else {
                // No session, but still return default variables for local scope (variablesRef == 1)
                if variables_ref == 1 {
                    vec![
                        Variable {
                            name: "@_".to_string(),
                            value: "()".to_string(),
                            type_: Some("array".to_string()),
                            variables_reference: 0,
                            named_variables: None,
                            indexed_variables: None,
                        },
                        Variable {
                            name: "$_".to_string(),
                            value: "undef".to_string(),
                            type_: Some("scalar".to_string()),
                            variables_reference: 0,
                            named_variables: None,
                            indexed_variables: None,
                        },
                    ]
                } else {
                    vec![]
                }
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
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let _ = stdin.write_all(b"c\n");
                let _ = stdin.flush();
                session.state = DebugState::Running;
            }
        }

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
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let _ = stdin.write_all(b"n\n");
                let _ = stdin.flush();
                session.state = DebugState::Running;
            }
        }

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
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let _ = stdin.write_all(b"s\n");
                let _ = stdin.flush();
                session.state = DebugState::Running;
            }
        }

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
        if let Some(ref mut session) = *self.session.lock().unwrap() {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let _ = stdin.write_all(b"r\n");
                let _ = stdin.flush();
                session.state = DebugState::Running;
            }
        }

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
        let success = if let Some(ref session) = *self.session.lock().unwrap() {
            let pid = session.process.id();
            self.send_interrupt_signal(pid)
        } else {
            eprintln!("No active debug session to pause");
            false
        };

        DapMessage::Response {
            seq,
            request_seq,
            success,
            command: "pause".to_string(),
            body: None,
            message: if !success { Some("Failed to pause debugger".to_string()) } else { None },
        }
    }

    /// Send interrupt signal to process (cross-platform)
    fn send_interrupt_signal(&self, pid: u32) -> bool {
        #[cfg(unix)]
        {
            let pid = pid as i32;
            match signal::kill(Pid::from_raw(pid), Signal::SIGINT) {
                Ok(()) => {
                    eprintln!("Sent SIGINT to process {}", pid);
                    true
                }
                Err(e) => {
                    eprintln!("Failed to send SIGINT to process {}: {}", pid, e);
                    false
                }
            }
        }
        #[cfg(windows)]
        {
            // On Windows, we use TerminateProcess or send Ctrl+C event
            // For Perl debugger, we can try sending input directly
            if let Some(ref mut session) = *self.session.lock().unwrap() {
                if let Some(stdin) = session.process.stdin.as_mut() {
                    // Send interrupt character (Ctrl+C equivalent in Perl debugger)
                    match stdin.write_all(b"\x03\n") {
                        Ok(()) => {
                            let _ = stdin.flush();
                            eprintln!("Sent interrupt signal to Perl debugger on process {}", pid);
                            true
                        }
                        Err(e) => {
                            eprintln!("Failed to send interrupt to process {}: {}", pid, e);
                            // Fallback: try to kill the process
                            match session.process.kill() {
                                Ok(()) => {
                                    eprintln!("Terminated process {} as fallback", pid);
                                    true
                                }
                                Err(kill_e) => {
                                    eprintln!("Failed to terminate process {}: {}", pid, kill_e);
                                    false
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("No stdin handle for process {}", pid);
                    false
                }
            } else {
                false
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            eprintln!("Interrupt signal not supported on this platform");
            false
        }
    }

    /// Handle evaluate request
    fn handle_evaluate(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        if let Some(args) = arguments {
            let expression = args.get("expression").and_then(|e| e.as_str()).unwrap_or("");

            if expression.is_empty() {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Empty expression".to_string()),
                };
            }

            // Send evaluation command to debugger
            if let Some(ref mut session) = *self.session.lock().unwrap() {
                if let Some(stdin) = session.process.stdin.as_mut() {
                    // Use 'x' command for better evaluation output
                    let cmd = format!("x {}\n", expression);
                    let _ = stdin.write_all(cmd.as_bytes());
                    let _ = stdin.flush();
                } else {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some("No debugger session active".to_string()),
                    };
                }
            } else {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("No debugger session".to_string()),
                };
            }

            // For now, return a placeholder result
            // In a full implementation, we'd capture the debugger's response
            let result = format!("<evaluating: {}>", expression);

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
                message: Some("Missing arguments".to_string()),
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
            DapMessage::Response { success, command, body, .. } => {
                assert!(success);
                assert_eq!(command, "initialize");
                assert!(body.is_some());
            }
            _ => panic!("Expected response"),
        }
    }
}
