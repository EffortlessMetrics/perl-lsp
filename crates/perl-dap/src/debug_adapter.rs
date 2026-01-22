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
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;

#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use regex::Regex;

/// Poison-safe mutex lock that recovers from poisoned state
fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, ctx: &'static str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Warning: poisoned mutex recovered: {ctx}");
            poisoned.into_inner()
        }
    }
}

/// Compiled regex patterns for debugger output parsing
static CONTEXT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static PROMPT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static STACK_FRAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
#[allow(dead_code)] // Reserved for future variable parsing enhancements
static VARIABLE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ERROR_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

fn context_re() -> Option<&'static Regex> {
    CONTEXT_RE
        .get_or_init(|| {
            Regex::new(r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)]+):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+)(?:\))?:(?P<line2>\d+):?)")
        })
        .as_ref()
        .ok()
}

fn prompt_re() -> Option<&'static Regex> {
    PROMPT_RE.get_or_init(|| Regex::new(r"^\s*DB<?\d*>?\s*$")).as_ref().ok()
}

fn stack_frame_re() -> Option<&'static Regex> {
    STACK_FRAME_RE
        .get_or_init(|| {
            Regex::new(r"^\s*#?\s*(?P<frame>\d+)?\s+(?P<func>[A-Za-z_][\w:]*+?)(?:\s+called)?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)")
        })
        .as_ref()
        .ok()
}

#[allow(dead_code)] // Reserved for future variable parsing enhancements
fn variable_re() -> Option<&'static Regex> {
    VARIABLE_RE
        .get_or_init(|| Regex::new(r"^\s*(?P<name>[\$\@\%][\w:]+)\s*=\s*(?P<value>.*?)$"))
        .as_ref()
        .ok()
}

fn error_re() -> Option<&'static Regex> {
    ERROR_RE
        .get_or_init(|| {
            Regex::new(r"^(?:.*?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)|Syntax error|Can't locate|Global symbol).*$")
        })
        .as_ref()
        .ok()
}

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

/// Represents a DAP message, which can be a request, response, or event.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DapMessage {
    /// A request from the client to the debug adapter.
    #[serde(rename = "request")]
    Request {
        /// Sequence number of the request.
        seq: i64,
        /// The command to execute.
        command: String,
        /// Arguments for the command.
        arguments: Option<Value>,
    },
    /// A response from the debug adapter to a client request.
    #[serde(rename = "response")]
    Response {
        /// Sequence number of the response.
        seq: i64,
        /// Sequence number of the corresponding request.
        request_seq: i64,
        /// Indicates whether the request was successful.
        success: bool,
        /// The command that was executed.
        command: String,
        /// The body of the response.
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
        /// An optional message providing additional information.
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// An event from the debug adapter to the client.
    #[serde(rename = "event")]
    Event {
        /// Sequence number of the event.
        seq: i64,
        /// The type of event.
        event: String,
        /// The body of the event.
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
        // Create a shared stdout writer to prevent interleaving between the main loop
        // and the event handler thread. This is critical for DAP protocol correctness:
        // both response frames and event frames must be written atomically to avoid
        // corrupting Content-Length framing.
        let stdout_writer: Arc<Mutex<io::Stdout>> = Arc::new(Mutex::new(io::stdout()));
        let event_stdout = Arc::clone(&stdout_writer);

        // Create channel for events
        let (tx, rx) = channel::<DapMessage>();
        self.event_sender = Some(tx.clone());

        // Start event handler thread with enhanced error handling
        // Uses shared stdout writer to serialize output with main loop
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match serde_json::to_string(&msg) {
                    Ok(json) => {
                        let content_length = json.len();
                        let frame = format!("Content-Length: {}\r\n\r\n{}", content_length, json);
                        // Lock shared stdout for atomic frame write
                        match event_stdout.lock() {
                            Ok(mut stdout) => {
                                if let Err(e) = stdout.write_all(frame.as_bytes()) {
                                    eprintln!("Failed to write DAP frame in event handler: {}", e);
                                }
                                if let Err(e) = stdout.flush() {
                                    eprintln!("Failed to flush stdout in event handler: {}", e);
                                }
                            }
                            Err(poisoned) => {
                                // Recover from poisoned mutex
                                eprintln!(
                                    "Warning: stdout mutex poisoned in event handler, recovering"
                                );
                                let mut stdout = poisoned.into_inner();
                                let _ = stdout.write_all(frame.as_bytes());
                                let _ = stdout.flush();
                            }
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
                                let frame =
                                    format!("Content-Length: {}\r\n\r\n{}", content_length, json);
                                // Lock shared stdout for atomic frame write
                                let mut stdout = lock_or_recover(&stdout_writer, "response_writer");
                                stdout.write_all(frame.as_bytes())?;
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

    /// Get next sequence number (monotonically increasing, poison-safe)
    fn next_seq(&self) -> i64 {
        let mut seq = lock_or_recover(&self.seq, "next_seq");
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
        // Security: Validate program path before any process spawning
        // This prevents command injection via flag arguments (e.g., "-e malicious_code")
        // and ensures we're launching a real Perl script file.

        let program = program.trim();

        // Reject empty or whitespace-only paths
        if program.is_empty() {
            return Err("Program path cannot be empty".to_string());
        }

        // Validate that the program is a regular file (not a directory, device, etc.)
        // Using metadata().is_file() is more robust than exists() because:
        // - exists() returns true for directories
        // - exists() returns true for symlinks to non-files
        // - is_file() specifically checks for regular files
        use std::path::Path;
        let path = Path::new(program);
        match std::fs::metadata(path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(format!("Program path is not a regular file: {}", program));
                }
            }
            Err(e) => {
                return Err(format!("Could not access program file '{}': {}", program, e));
            }
        }

        let mut cmd = Command::new("perl");
        cmd.arg("-d");

        // Perl debugger stops on the first line by default
        let _ = stop_on_entry; // currently unused

        // Use -- to separate flags from script name, preventing argument injection
        // if program starts with -
        cmd.arg("--");
        cmd.arg(program);
        cmd.args(&args);

        // Set up pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                let thread_id = {
                    if let Ok(mut counter) = self.thread_counter.lock() {
                        *counter += 1;
                        *counter
                    } else {
                        eprintln!("Failed to lock thread counter, using 1");
                        1
                    }
                };

                let session = DebugSession {
                    process: child,
                    state: DebugState::Running,
                    stack_frames: Vec::new(),
                    variables: HashMap::new(),
                    thread_id,
                };

                if let Ok(mut guard) = self.session.lock() {
                    *guard = Some(session);
                } else {
                    return Err("Failed to lock session".to_string());
                }

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
                if let Ok(mut guard) = session.lock() {
                    guard.as_mut().and_then(|s| s.process.stdout.take())
                } else {
                    eprintln!("Failed to lock session in output reader");
                    None
                }
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
                        if let Some(re) = context_re()
                            && let Some(caps) = re.captures(&text)
                        {
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
                        if !context_updated
                            && let Some(re) = stack_frame_re()
                            && let Some(caps) = re.captures(&text)
                        {
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

                        // Check for errors that might provide location info
                        if !context_updated
                            && let Some(re) = error_re()
                            && let Some(caps) = re.captures(&text)
                        {
                            if let Some(file) = caps.name("file") {
                                current_file = file.as_str().to_string();
                            }
                            if let Some(line_num) = caps.name("line") {
                                current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                            }
                            context_updated = true;

                            // Send error event to client
                            if let Some(ref sender) = sender
                                && let Ok(mut seq_lock) = seq.lock()
                            {
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

                        if context_updated {
                            continue;
                        }

                        // Detect debugger prompt (stopped state) with enhanced pattern matching
                        if prompt_re().is_some_and(|re| re.is_match(&text))
                            || text.trim().starts_with("DB<")
                            || text.trim().starts_with("  DB<")
                        {
                            _debugger_ready = true;
                            let thread_id = {
                                let Ok(mut guard) = session.lock() else {
                                    eprintln!(
                                        "Failed to lock session when processing debugger prompt"
                                    );
                                    continue;
                                };
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
    ///
    /// Attaches to a running Perl process. Supports two modes:
    /// 1. TCP attachment - Connect to Perl::LanguageServer DAP via host:port
    /// 2. Process ID attachment - Attach to local Perl process (future implementation)
    ///
    /// For TCP attachment, the arguments should contain:
    /// - `host`: Hostname or IP address (default: "localhost")
    /// - `port`: Port number (default: 13603)
    /// - `timeout`: Connection timeout in milliseconds (optional)
    ///
    /// # Current Implementation
    ///
    /// TCP attachment is not yet fully implemented. This is a placeholder that:
    /// - Validates attach arguments
    /// - Returns appropriate error messages
    /// - Provides foundation for future TCP socket implementation
    ///
    /// Process ID attachment will be added in Phase 2.
    fn handle_attach(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        // Parse attach arguments
        if let Some(args) = arguments {
            // Extract host and port for TCP attachment
            let host = args.get("host").and_then(|h| h.as_str()).unwrap_or("localhost");
            let port = args.get("port").and_then(|p| p.as_u64()).unwrap_or(13603) as u16;
            let timeout = args.get("timeout").and_then(|t| t.as_u64()).map(|t| t as u32);
            let process_id = args.get("processId").and_then(|p| p.as_u64()).map(|p| p as u32);

            // Validate arguments
            if host.trim().is_empty() {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "attach".to_string(),
                    body: None,
                    message: Some("Host cannot be empty".to_string()),
                };
            }

            if port == 0 {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "attach".to_string(),
                    body: None,
                    message: Some("Port must be in range 1-65535".to_string()),
                };
            }

            if let Some(t) = timeout {
                if t == 0 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some("Timeout must be greater than 0 milliseconds".to_string()),
                    };
                }
                if t > 300_000 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(
                            "Timeout cannot exceed 300000 milliseconds (5 minutes)".to_string(),
                        ),
                    };
                }
            }

            // Determine attachment mode
            if let Some(pid) = process_id {
                // Process ID attachment mode (future implementation)
                eprintln!(
                    "Attach request: Process ID attachment to PID {} (not yet implemented)",
                    pid
                );
                DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "attach".to_string(),
                    body: None,
                    message: Some(format!(
                        "Process ID attachment not yet implemented (PID: {}). \
                         Use TCP attachment with host/port for Perl::LanguageServer compatibility.",
                        pid
                    )),
                }
            } else {
                // TCP attachment mode (future implementation)
                let timeout_msg = if let Some(t) = timeout {
                    format!(" with {}ms timeout", t)
                } else {
                    String::new()
                };
                eprintln!("Attach request: TCP attachment to {}:{}{}", host, port, timeout_msg);

                // TODO: Implement TCP socket connection to Perl::LanguageServer DAP
                // This will require:
                // 1. Establishing TCP connection to host:port
                // 2. Setting up bidirectional message proxying
                // 3. Handling connection errors gracefully
                // 4. Managing timeout during connection attempt
                // 5. Sending appropriate DAP events (attached, initialized)

                DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "attach".to_string(),
                    body: None,
                    message: Some(format!(
                        "TCP attachment not yet fully implemented. \
                         Would connect to {}:{}{} for Perl::LanguageServer DAP. \
                         Use BridgeAdapter for current Perl::LanguageServer integration.",
                        host, port, timeout_msg
                    )),
                }
            }
        } else {
            // No arguments provided
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "attach".to_string(),
                body: None,
                message: Some(
                    "Missing attach arguments. Provide either 'processId' for process attachment \
                     or 'host' and 'port' for TCP attachment."
                        .to_string(),
                ),
            }
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
        if let Ok(mut guard) = self.session.lock()
            && let Some(mut session) = guard.take()
        {
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
            if let Ok(mut guard) = self.session.lock()
                && let Some(ref mut session) = *guard
            {
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

                if let Some(ref mut session) =
                    *lock_or_recover(&self.session, "debug_adapter.session")
                    && let Some(stdin) = session.process.stdin.as_mut()
                {
                    let cmd = if let Some(cond) = condition {
                        format!("b {} {}\n", line, cond)
                    } else {
                        format!("b {}\n", line)
                    };

                    success = stdin.write_all(cmd.as_bytes()).is_ok() && stdin.flush().is_ok();
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
            lock_or_recover(&self.breakpoints, "debug_adapter.breakpoints")
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
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            // Send initial 'l' command to list current location
            let _ = stdin.write_all(b"l\n");
            let _ = stdin.flush();
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
        let threads =
            if let Some(ref session) = *lock_or_recover(&self.session, "debug_adapter.session") {
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
        let stack_frames =
            if let Some(ref session) = *lock_or_recover(&self.session, "debug_adapter.session") {
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

            let variables = if let Some(ref session) =
                *lock_or_recover(&self.session, "debug_adapter.session")
            {
                // Try to get cached variables first
                if let Some(vars) = session.variables.get(&variables_ref) {
                    vars.clone()
                } else {
                    // Generate some basic variables for the local scope
                    if variables_ref == 1 {
                        // Frame 1 local variables - send command to get them
                        if let Some(ref mut session) =
                            *lock_or_recover(&self.session, "debug_adapter.session")
                            && let Some(stdin) = session.process.stdin.as_mut()
                        {
                            let _ = stdin.write_all(b"V\n"); // Show local variables
                            let _ = stdin.flush();
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
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"c\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
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
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"n\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
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
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"s\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
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
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"r\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
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
        let success =
            if let Some(ref session) = *lock_or_recover(&self.session, "debug_adapter.session") {
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
    #[allow(unused_variables)] // pid unused on non-unix/non-windows platforms (e.g., wasm32)
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
            if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            {
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

            // Security: Reject expressions with newlines to prevent command injection
            if expression.contains('\n') || expression.contains('\r') {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Expression cannot contain newlines".to_string()),
                };
            }

            // Send evaluation command to debugger
            if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            {
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
        assert!(adapter.session.lock().ok().is_some_and(|guard| guard.is_none()));
        assert!(adapter.breakpoints.lock().ok().is_some_and(|guard| guard.is_empty()));
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

    #[test]
    fn test_attach_missing_arguments() {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(1, "attach", None);

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Missing attach arguments"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_tcp_valid_arguments() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "localhost",
            "port": 13603,
            "timeout": 5000
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success); // Not yet implemented, but validates correctly
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("localhost:13603"));
                assert!(msg.contains("5000ms timeout"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_process_id_mode() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "processId": 12345
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success); // Not yet implemented
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Process ID attachment"));
                assert!(msg.contains("12345"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_empty_host() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "",
            "port": 13603
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Host cannot be empty"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_whitespace_host() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "   ",
            "port": 13603
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Host cannot be empty"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_zero_port() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "localhost",
            "port": 0
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Port must be in range"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_zero_timeout() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "localhost",
            "port": 13603,
            "timeout": 0
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Timeout must be greater than 0"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_excessive_timeout() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "localhost",
            "port": 13603,
            "timeout": 400000
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("Timeout cannot exceed"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_default_values() {
        let mut adapter = DebugAdapter::new();
        // Empty args should use defaults and fail with missing arguments message
        let args = json!({});
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                // Should use default host/port but still not be implemented
                let msg = message.unwrap();
                assert!(msg.contains("localhost:13603"));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_attach_custom_port() {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "host": "192.168.1.100",
            "port": 9000
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success); // Not yet implemented
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.unwrap();
                assert!(msg.contains("192.168.1.100:9000"));
            }
            _ => panic!("Expected response"),
        }
    }
}
