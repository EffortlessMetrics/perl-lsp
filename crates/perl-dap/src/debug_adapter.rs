//! Debug Adapter Protocol (DAP) implementation for Perl debugging
//!
//! This module provides a DAP server that integrates with Perl's built-in debugger
//! to enable debugging support in VSCode and other DAP-compatible editors.

use crate::feature_catalog::has_feature as catalog_has_feature;
use crate::inline_values::collect_inline_values;
use crate::protocol::{
    ContinueArguments, ContinueResponseBody, DisconnectArguments, EvaluateArguments,
    EvaluateResponseBody, InlineValuesArguments, InlineValuesResponseBody, NextArguments,
    PauseArguments, Scope, ScopesArguments, ScopesResponseBody, SetExceptionBreakpointsArguments,
    SetFunctionBreakpointsArguments, SetVariableArguments, SetVariableResponseBody,
    StackTraceArguments, StepInArguments, StepOutArguments, TerminateArguments, VariablesArguments,
};
use crate::tcp_attach::{DapEvent, TcpAttachConfig, TcpAttachSession};
use perl_dap_eval::SafeEvaluator;
use perl_dap_stack::PerlStackParser;
use perl_dap_variables::{
    PerlVariableRenderer, RenderedVariable, VariableParser, VariableRenderer,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::breakpoints::{BreakpointHitOutcome, BreakpointStore};
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

/// Send a DAP event through the event channel with poison-safe sequence numbering.
///
/// Returns `true` if the event was successfully sent, `false` otherwise.
fn emit_event_safe(
    sender: &Sender<DapMessage>,
    seq: &Mutex<i64>,
    event: &str,
    body: Option<Value>,
) -> bool {
    let mut seq_lock = lock_or_recover(seq, "emit_event_safe.seq");
    *seq_lock += 1;
    sender.send(DapMessage::Event { seq: *seq_lock, event: event.to_string(), body }).is_ok()
}

/// Compiled regex patterns for debugger output parsing
static CONTEXT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static PROMPT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static STACK_FRAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
#[allow(dead_code)] // Reserved for future variable parsing enhancements
static VARIABLE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ERROR_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static EXCEPTION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static DANGEROUS_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static REGEX_MUTATION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ASSIGNMENT_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static DEREF_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static GLOB_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ANSI_ESCAPE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static SET_VARIABLE_NAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static FUNCTION_BREAKPOINT_NAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
const RECENT_OUTPUT_MAX_LINES: usize = 2048;
const DEBUGGER_QUERY_WAIT_MS: u64 = 75;
const DEBUGGER_FRAME_POLL_MS: u64 = 10;

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

fn exception_re() -> Option<&'static Regex> {
    EXCEPTION_RE
        .get_or_init(|| {
            // Perl `die` often emits two lines:
            //  - message text
            //  - `at /path/file.pl line N.`
            Regex::new(r"(?i)\b(?:died|uncaught exception|panic)\b|^\s*at\s+\S+?\s+line\s+\d+\.?$")
        })
        .as_ref()
        .ok()
}

fn dangerous_ops_re() -> Option<&'static Regex> {
    DANGEROUS_OPS_RE
        .get_or_init(|| {
            // Dangerous operations that can mutate state, perform I/O, or execute code
            // Categories:
            //   - State mutation: push, pop, shift, unshift, splice, delete, undef, srand
            //   - Process control: system, exec, fork, exit, dump, kill, alarm, sleep, wait, waitpid
            //   - I/O: qx, readpipe, syscall, open, close, print, say, printf, sysread, syswrite, glob, readline, ioctl, fcntl, flock, select, dbmopen, dbmclose
            //   - Filesystem: mkdir, rmdir, unlink, rename, chdir, chmod, chown, chroot, truncate, symlink, link
            //   - Code loading: eval, require, do (file)
            //   - Tie/untie: can execute arbitrary code via FETCH/STORE
            //   - Network: socket, connect, bind, listen, accept, send, recv, shutdown
            //   - IPC: msg*, sem*, shm*
            // Note: s/tr/y regex mutation operators handled separately via regex_mutation_re()
            let ops = [
                // State mutation
                "push",
                "pop",
                "shift",
                "unshift",
                "splice",
                "delete",
                "undef",
                "srand",
                "bless",
                "reset", // Process control
                "system",
                "exec",
                "fork",
                "exit",
                "dump",
                "kill",
                "alarm",
                "sleep",
                "wait",
                "waitpid",
                "setpgrp",
                "setpriority",
                "umask",
                "lock", // I/O
                "qx",
                "readpipe",
                "syscall",
                "open",
                "close",
                "print",
                "say",
                "printf",
                "sysread",
                "syswrite",
                "glob",
                "readline",
                "ioctl",
                "fcntl",
                "flock",
                "select",
                "dbmopen",
                "dbmclose",
                "binmode",
                "opendir",
                "closedir",
                "readdir",
                "rewinddir",
                "seekdir",
                "telldir",
                "seek",
                "sysseek",
                "formline",
                "write",
                "pipe",
                "socketpair", // Filesystem
                "mkdir",
                "rmdir",
                "unlink",
                "rename",
                "chdir",
                "chmod",
                "chown",
                "chroot",
                "truncate",
                "utime",
                "symlink",
                "link", // Code loading/execution
                "eval",
                "require",
                "do", // Tie mechanism (can execute arbitrary code)
                "tie",
                "untie", // Network
                "socket",
                "connect",
                "bind",
                "listen",
                "accept",
                "send",
                "recv",
                "shutdown",
                "setsockopt",
                // IPC
                "msgget",
                "msgsnd",
                "msgrcv",
                "msgctl",
                "semget",
                "semop",
                "semctl",
                "shmget",
                "shmat",
                "shmdt",
                "shmctl",
            ];
            // Build pattern: \b(op1|op2|...)\b
            let pattern = format!(r"\b(?:{})\b", ops.join("|"));
            Regex::new(&pattern)
        })
        .as_ref()
        .ok()
}

/// Regex to match mutating regex operators (s///, tr///, y///)
/// Matches s, tr, y followed by a delimiter character
fn regex_mutation_re() -> Option<&'static Regex> {
    REGEX_MUTATION_RE
        .get_or_init(|| {
            // Match s, tr, y followed by a delimiter character (not alphanumeric/underscore/whitespace)
            // Common delimiters: / # | ! { [ ( ' "
            // Note: We filter out escape sequences like \s manually after matching
            Regex::new(r"\b(?:s|tr|y)[^\w\s]")
        })
        .as_ref()
        .ok()
}

/// Regex to match potential assignment operators (any sequence of operator chars)
fn assignment_ops_re() -> Option<&'static Regex> {
    ASSIGNMENT_OPS_RE
        .get_or_init(|| {
            // Match any sequence of operator characters to tokenize operators
            Regex::new(r"([!~^&|+\-*/%=<>]+)")
        })
        .as_ref()
        .ok()
}

/// Regex to match dynamic subroutine dereferencing: &{...}
fn deref_re() -> Option<&'static Regex> {
    DEREF_RE.get_or_init(|| Regex::new(r"&[\s]*\{")).as_ref().ok()
}

/// Regex to match glob operations: <*...>
fn glob_re() -> Option<&'static Regex> {
    GLOB_RE.get_or_init(|| Regex::new(r"<\*[^>]*>")).as_ref().ok()
}

/// Regex for matching ANSI escape sequences in debugger output.
fn ansi_escape_re() -> Option<&'static Regex> {
    ANSI_ESCAPE_RE.get_or_init(|| Regex::new(r"\x1B\[[0-9;]*[A-Za-z]")).as_ref().ok()
}

/// Regex for validating setVariable variable names to avoid debugger command injection.
fn set_variable_name_re() -> Option<&'static Regex> {
    SET_VARIABLE_NAME_RE
        .get_or_init(|| {
            Regex::new(r"^[\$\@\%](?:[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*|\d+|_)$")
        })
        .as_ref()
        .ok()
}

/// Validate DAP setVariable names (e.g. `$x`, `%ENV`, `$Package::value`) for safe passthrough.
fn is_valid_set_variable_name(name: &str) -> bool {
    set_variable_name_re().is_some_and(|re| re.is_match(name))
}

fn function_breakpoint_name_re() -> Option<&'static Regex> {
    FUNCTION_BREAKPOINT_NAME_RE
        .get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$"))
        .as_ref()
        .ok()
}

fn is_valid_function_breakpoint_name(name: &str) -> bool {
    function_breakpoint_name_re().is_some_and(|re| re.is_match(name))
}

/// Check if the match is an escape sequence (preceded by backslash)
fn is_escape_sequence(s: &str, match_start: usize) -> bool {
    if match_start == 0 {
        return false;
    }
    s.as_bytes()[match_start - 1] == b'\\'
}

/// DAP server that handles debug sessions
pub struct DebugAdapter {
    /// Sequence number for messages
    seq: Arc<Mutex<i64>>,
    /// Active debug session (process-based)
    session: Arc<Mutex<Option<DebugSession>>>,
    /// Attached process ID for PID-based attach mode
    attached_pid: Arc<Mutex<Option<u32>>>,
    /// TCP attach session (for connecting to running debugger)
    tcp_session: Arc<Mutex<Option<TcpAttachSession>>>,
    /// Breakpoints store
    breakpoints: BreakpointStore,
    /// Thread ID counter
    thread_counter: Arc<Mutex<i32>>,
    /// Output channel for sending events to client
    event_sender: Option<Sender<DapMessage>>,
    /// Bounded history of debugger output for stack/variable/evaluate parsing
    recent_output: Arc<Mutex<VecDeque<String>>>,
    /// Function breakpoints (`setFunctionBreakpoints`) stored with REPLACE semantics
    function_breakpoints: Arc<Mutex<Vec<String>>>,
    /// Monotonic IDs for function breakpoints
    next_function_breakpoint_id: Arc<Mutex<i64>>,
    /// Exception breakpoint policy: break on `die`/uncaught exception output.
    exception_break_on_die: Arc<Mutex<bool>>,
    /// Unique marker IDs used to frame debugger output per command.
    debugger_output_marker: Arc<AtomicU64>,
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
    /// Last resume command issued while running.
    last_resume_mode: ResumeMode,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum DebugState {
    Running,
    Stopped,
    Terminated,
}

#[derive(Debug, Clone, PartialEq)]
enum ResumeMode {
    Continue,
    Next,
    StepIn,
    StepOut,
    Unknown,
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
            attached_pid: Arc::new(Mutex::new(None)),
            tcp_session: Arc::new(Mutex::new(None)),
            breakpoints: BreakpointStore::new(),
            thread_counter: Arc::new(Mutex::new(0)),
            event_sender: None,
            recent_output: Arc::new(Mutex::new(VecDeque::with_capacity(RECENT_OUTPUT_MAX_LINES))),
            function_breakpoints: Arc::new(Mutex::new(Vec::new())),
            next_function_breakpoint_id: Arc::new(Mutex::new(1)),
            exception_break_on_die: Arc::new(Mutex::new(false)),
            debugger_output_marker: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Set the event sender (primarily for testing)
    pub fn set_event_sender(&mut self, sender: Sender<DapMessage>) {
        self.event_sender = Some(sender);
    }

    /// Run the debug adapter server
    pub fn run(&mut self) -> io::Result<()> {
        self.run_with_io(io::stdin(), io::stdout())
    }

    /// Run the debug adapter over a TCP socket transport.
    ///
    /// This binds to `127.0.0.1:<port>`, accepts one client connection, and
    /// serves the DAP session on that stream.
    pub fn run_socket(&mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        eprintln!("DAP socket transport listening on 127.0.0.1:{port}");

        let (stream, peer_addr) = listener.accept()?;
        eprintln!("DAP socket client connected: {peer_addr}");

        let reader_stream = stream.try_clone()?;
        self.run_with_io(reader_stream, stream)
    }

    /// Shared DAP transport loop used by stdio and socket modes.
    fn run_with_io<R, W>(&mut self, input: R, output: W) -> io::Result<()>
    where
        R: Read,
        W: Write + Send + 'static,
    {
        // Create a shared writer to prevent interleaving between the main loop
        // and the event handler thread.
        let shared_writer: Arc<Mutex<W>> = Arc::new(Mutex::new(output));
        let event_writer = Arc::clone(&shared_writer);

        // Create channel for asynchronous events.
        let (tx, rx) = channel::<DapMessage>();
        self.event_sender = Some(tx.clone());

        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let frame = match serde_json::to_string(&msg) {
                    Ok(payload) => {
                        format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload)
                    }
                    Err(e) => {
                        eprintln!("Failed to serialize DAP message: {} - {:#?}", e, msg);
                        continue;
                    }
                };

                let mut writer = lock_or_recover(&event_writer, "event_writer");
                if let Err(e) = writer.write_all(frame.as_bytes()) {
                    eprintln!("Failed to write DAP frame in event handler: {}", e);
                    continue;
                }
                if let Err(e) = writer.flush() {
                    eprintln!("Failed to flush DAP frame in event handler: {}", e);
                }
            }
            eprintln!("Event handler thread terminating - channel closed");
        });

        let mut reader = BufReader::new(input);
        let mut line = String::new();

        loop {
            // Read headers.
            let mut headers = HashMap::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => return Ok(()), // EOF
                    Ok(_) => {
                        let trimmed = line.trim_end();
                        if trimmed.is_empty() {
                            break;
                        }
                        if let Some(colon_pos) = trimmed.find(':') {
                            let key = trimmed[..colon_pos].trim();
                            let value = trimmed[colon_pos + 1..].trim();
                            headers.insert(key.to_string(), value.to_string());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            // Read content body based on Content-Length.
            let Some(content_length) = headers.get("Content-Length") else {
                continue;
            };
            let Ok(length) = content_length.parse::<usize>() else {
                eprintln!("Invalid Content-Length header: {content_length}");
                continue;
            };

            let mut buffer = vec![0u8; length];
            reader.read_exact(&mut buffer)?;

            let msg = match serde_json::from_slice::<DapMessage>(&buffer) {
                Ok(msg) => msg,
                Err(_) => {
                    eprintln!("Failed to parse DAP message: {}", String::from_utf8_lossy(&buffer));
                    continue;
                }
            };

            let DapMessage::Request { seq, command, arguments } = msg else {
                continue;
            };

            let response = self.dispatch_request(seq, &command, arguments);
            let payload = match serde_json::to_string(&response) {
                Ok(payload) => payload,
                Err(e) => {
                    eprintln!("Failed to serialize DAP response: {}", e);
                    continue;
                }
            };

            let frame = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
            let mut writer = lock_or_recover(&shared_writer, "response_writer");
            writer.write_all(frame.as_bytes())?;
            writer.flush()?;

            // DAP requires this event only after initialize response is sent.
            if command == "initialize"
                && Self::response_succeeded_for_command(&response, "initialize")
            {
                self.send_event("initialized", None);
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

        let response = self.dispatch_request(request_seq, command, arguments);

        // Preserve existing direct-call behavior for tests and in-memory usage.
        if command == "initialize" && Self::response_succeeded_for_command(&response, "initialize")
        {
            self.send_event("initialized", None);
        }

        response
    }

    /// Handle a DAP request (mock version for testing)
    pub fn handle_request_mock(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        eprintln!("DAP request (mock): {} {:?}", command, arguments);

        if command == "attach" {
            let seq = self.next_seq();
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "attach".to_string(),
                body: None,
                message: Some("Attach not yet fully implemented".to_string()),
            };
        }

        let response = self.dispatch_request(request_seq, command, arguments);
        if command == "initialize" && Self::response_succeeded_for_command(&response, "initialize")
        {
            self.send_event("initialized", None);
        }
        response
    }

    fn dispatch_request(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        let seq = self.next_seq();

        match command {
            "initialize" => self.handle_initialize(seq, request_seq, arguments),
            "launch" => self.handle_launch(seq, request_seq, arguments),
            "attach" => self.handle_attach(seq, request_seq, arguments),
            "disconnect" => self.handle_disconnect(seq, request_seq, arguments),
            "terminate" => self.handle_terminate(seq, request_seq, arguments),
            "setBreakpoints" => self.handle_set_breakpoints(seq, request_seq, arguments),
            "setFunctionBreakpoints" => {
                self.handle_set_function_breakpoints(seq, request_seq, arguments)
            }
            "setExceptionBreakpoints" => {
                self.handle_set_exception_breakpoints(seq, request_seq, arguments)
            }
            "configurationDone" => self.handle_configuration_done(seq, request_seq),
            "threads" => self.handle_threads(seq, request_seq),
            "stackTrace" => self.handle_stack_trace(seq, request_seq, arguments),
            "scopes" => self.handle_scopes(seq, request_seq, arguments),
            "variables" => self.handle_variables(seq, request_seq, arguments),
            "setVariable" => self.handle_set_variable(seq, request_seq, arguments),
            "continue" => self.handle_continue(seq, request_seq, arguments),
            "next" => self.handle_next(seq, request_seq, arguments),
            "stepIn" => self.handle_step_in(seq, request_seq, arguments),
            "stepOut" => self.handle_step_out(seq, request_seq, arguments),
            "pause" => self.handle_pause(seq, request_seq, arguments),
            "evaluate" => self.handle_evaluate(seq, request_seq, arguments),
            "inlineValues" => self.handle_inline_values(seq, request_seq, arguments),
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

    fn response_succeeded_for_command(response: &DapMessage, expected_command: &str) -> bool {
        matches!(
            response,
            DapMessage::Response {
                success: true,
                command,
                ..
            } if command == expected_command
        )
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

    /// Snapshot debugger output history for parsing without holding locks.
    fn snapshot_recent_output_lines(&self) -> Vec<String> {
        let output = lock_or_recover(&self.recent_output, "debug_adapter.recent_output");
        output.iter().cloned().collect()
    }

    /// Allocate a unique marker id used for framed debugger output capture.
    fn next_debugger_marker_id(&self) -> u64 {
        self.debugger_output_marker.fetch_add(1, Ordering::Relaxed)
    }

    /// Normalize debugger output lines for deterministic parsing by:
    /// - removing ANSI escape sequences
    /// - stripping debugger prompt prefixes (e.g. `DB<1>`)
    fn normalize_debugger_output_line(line: &str) -> String {
        let mut normalized = if let Some(re) = ansi_escape_re() {
            re.replace_all(line, "").into_owned()
        } else {
            line.to_string()
        };

        if let Some(prompt_start) = normalized.find("DB<")
            && let Some(prompt_end) = normalized[prompt_start..].find('>')
        {
            let content_start = prompt_start + prompt_end + 1;
            normalized = normalized[content_start..].to_string();
        }

        normalized.trim().to_string()
    }

    /// Infer a coarse DAP value type from literal-like debugger output.
    fn infer_debugger_value_type(text: &str) -> String {
        if text == "undef" {
            "undef".to_string()
        } else if text.parse::<i64>().is_ok() {
            "integer".to_string()
        } else if text.parse::<f64>().is_ok() {
            "number".to_string()
        } else if text.starts_with('[') && text.ends_with(']') {
            "array".to_string()
        } else if text.starts_with('{') && text.ends_with('}') {
            "hash".to_string()
        } else {
            "string".to_string()
        }
    }

    /// Write a debugger command and flush immediately so output framing remains ordered.
    fn write_debugger_command(stdin: &mut impl Write, command: &str) -> Result<(), String> {
        stdin.write_all(command.as_bytes()).map_err(|e| format!("write debugger command: {e}"))?;
        stdin.flush().map_err(|e| format!("flush debugger command: {e}"))?;
        Ok(())
    }

    /// Send commands wrapped with unique begin/end markers.
    ///
    /// Returns `(begin_marker, end_marker)` so callers can wait for framed output.
    fn send_framed_debugger_commands(
        &self,
        stdin: &mut impl Write,
        commands: &[String],
    ) -> Result<(String, String), String> {
        let marker_id = self.next_debugger_marker_id();
        let begin_marker = format!("DAP_BEGIN_{marker_id}");
        let end_marker = format!("DAP_END_{marker_id}");

        Self::write_debugger_command(stdin, &format!("p \"{begin_marker}\"\n"))?;
        for command in commands {
            if command.ends_with('\n') {
                Self::write_debugger_command(stdin, command)?;
            } else {
                Self::write_debugger_command(stdin, &format!("{command}\n"))?;
            }
        }
        Self::write_debugger_command(stdin, &format!("p \"{end_marker}\"\n"))?;

        Ok((begin_marker, end_marker))
    }

    /// Capture debugger output lines between begin/end markers.
    fn capture_framed_debugger_output(
        &self,
        begin_marker: &str,
        end_marker: &str,
        timeout_ms: u64,
    ) -> Option<Vec<String>> {
        let deadline =
            Instant::now() + Duration::from_millis(timeout_ms.max(DEBUGGER_QUERY_WAIT_MS));

        loop {
            let lines = self.snapshot_recent_output_lines();
            let normalized_lines: Vec<String> =
                lines.iter().map(|line| Self::normalize_debugger_output_line(line)).collect();

            if let Some(begin_idx) =
                normalized_lines.iter().rposition(|line| line.contains(begin_marker))
                && let Some(end_rel) = normalized_lines[begin_idx + 1..]
                    .iter()
                    .position(|line| line.contains(end_marker))
            {
                let end_idx = begin_idx + 1 + end_rel;
                let framed = normalized_lines[begin_idx + 1..end_idx]
                    .iter()
                    .filter(|line| !line.trim().is_empty())
                    .cloned()
                    .collect::<Vec<_>>();
                return Some(framed);
            }

            if Instant::now() >= deadline {
                return None;
            }

            thread::sleep(Duration::from_millis(DEBUGGER_FRAME_POLL_MS));
        }
    }

    /// Wait briefly for debugger command responses to arrive in the output buffer.
    fn wait_for_debugger_output_window(timeout_ms: u32) {
        let wait_ms = u64::from(timeout_ms.min(250)).max(DEBUGGER_QUERY_WAIT_MS);
        thread::sleep(Duration::from_millis(wait_ms));
    }

    /// Convert i64 values in protocol payloads to i32 with saturation.
    fn i64_to_i32_saturating(value: i64) -> i32 {
        match i32::try_from(value) {
            Ok(v) => v,
            Err(_) => {
                if value.is_negative() {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }
        }
    }

    /// Convert microcrate rendered variables into adapter-local protocol values.
    fn rendered_to_variable(rendered: RenderedVariable) -> Variable {
        Variable {
            name: rendered.name,
            value: rendered.value,
            type_: rendered.type_name,
            variables_reference: Self::i64_to_i32_saturating(rendered.variables_reference),
            named_variables: rendered.named_variables.map(Self::i64_to_i32_saturating),
            indexed_variables: rendered.indexed_variables.map(Self::i64_to_i32_saturating),
        }
    }

    /// Determine if a variable name should appear in a given scope.
    fn scope_allows_variable_name(scope_type: i32, name: &str) -> bool {
        match scope_type {
            // Locals
            1 => !name.contains("::"),
            // Package variables (qualified)
            2 => name.contains("::"),
            // Globals/specials
            3 => {
                matches!(name, "$_" | "@ARGV" | "%ENV" | "$!" | "$@" | "$/" | "$|" | "$0" | "$^W")
                    || name.starts_with("$^")
            }
            _ => true,
        }
    }

    /// Convert parsed stack frames from `perl-dap-stack` into local DAP response frames.
    fn parse_stack_frames_from_text(output: &str) -> Vec<StackFrame> {
        let mut parser = PerlStackParser::new();
        parser
            .parse_stack_trace(output)
            .into_iter()
            .map(|frame| {
                let source = frame.source.unwrap_or_default();
                let path = source.path.unwrap_or_else(|| "<unknown>".to_string());
                let name = source.name.or_else(|| {
                    std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(ToString::to_string)
                });
                StackFrame {
                    id: Self::i64_to_i32_saturating(frame.id),
                    name: frame.name,
                    source: Source { name, path, source_reference: None },
                    line: Self::i64_to_i32_saturating(frame.line),
                    column: Self::i64_to_i32_saturating(frame.column),
                    end_line: frame.end_line.map(Self::i64_to_i32_saturating),
                    end_column: frame.end_column.map(Self::i64_to_i32_saturating),
                }
            })
            .collect()
    }

    /// Filter out internal debugger and shim frames from user-visible stack traces.
    fn filter_user_visible_frames(frames: Vec<StackFrame>) -> Vec<StackFrame> {
        frames
            .into_iter()
            .filter(|f| {
                !f.name.starts_with("Devel::TSPerlDAP::")
                    && !f.name.starts_with("DB::")
                    && !f.source.path.contains("perl5db.pl")
            })
            .collect()
    }

    /// Parse variables from debugger output lines using microcrate parser/renderer.
    fn parse_scope_variables_from_lines(
        lines: &[String],
        variables_ref: i32,
        start: usize,
        count: usize,
    ) -> (Vec<Variable>, HashMap<i32, Vec<Variable>>) {
        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();
        let scope_type = variables_ref % 10;
        let mut seen = HashSet::new();
        let mut parsed = Vec::new();

        for line in lines.iter().rev() {
            let normalized = Self::normalize_debugger_output_line(line);
            let text = normalized.trim();
            if text.is_empty() {
                continue;
            }
            if let Ok((name, value)) = parser.parse_assignment(text) {
                if !Self::scope_allows_variable_name(scope_type, &name) {
                    continue;
                }
                if seen.insert(name.clone()) {
                    parsed.push((name, value));
                }
                if parsed.len() >= 256 {
                    break;
                }
            }
        }

        parsed.reverse();
        parsed.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

        let mut top_level = Vec::new();
        let mut child_cache = HashMap::new();
        for (idx, (name, value)) in parsed.into_iter().skip(start).take(count).enumerate() {
            let child_ref = variables_ref.saturating_mul(1000).saturating_add(
                Self::i64_to_i32_saturating(i64::try_from(idx + 1).unwrap_or(i64::from(i32::MAX))),
            );
            let rendered = if value.is_expandable() {
                renderer.render_with_reference(&name, &value, i64::from(child_ref))
            } else {
                renderer.render(&name, &value)
            };
            top_level.push(Self::rendered_to_variable(rendered));

            if value.is_expandable() {
                let children = renderer
                    .render_children(&value, 0, 256)
                    .into_iter()
                    .map(Self::rendered_to_variable)
                    .collect::<Vec<_>>();
                if !children.is_empty() {
                    child_cache.insert(child_ref, children);
                }
            }
        }

        (top_level, child_cache)
    }

    /// Parse variables from recent debugger output using microcrate parser/renderer.
    fn parse_scope_variables_from_output(
        &self,
        variables_ref: i32,
        start: usize,
        count: usize,
    ) -> (Vec<Variable>, HashMap<i32, Vec<Variable>>) {
        let lines = self.snapshot_recent_output_lines();
        Self::parse_scope_variables_from_lines(&lines, variables_ref, start, count)
    }

    /// Parse evaluate output from debugger lines into a DAP result payload.
    fn parse_evaluate_result_from_lines(
        lines: &[String],
        expression: &str,
        allow_fallback_line: bool,
    ) -> Option<(String, String)> {
        if lines.is_empty() {
            return None;
        }

        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();

        for line in lines.iter().rev() {
            let normalized = Self::normalize_debugger_output_line(line);
            let text = normalized.trim();
            if text.is_empty() || prompt_re().is_some_and(|re| re.is_match(text)) {
                continue;
            }

            if let Ok((name, value)) = parser.parse_assignment(text) {
                let rendered = renderer.render(&name, &value);
                let type_name = rendered.type_name.unwrap_or_else(|| "string".to_string());
                // Prefer direct matches for the evaluated expression, but allow fallback assignment.
                if name == expression || text.starts_with(expression) || text.contains(expression) {
                    return Some((rendered.value, type_name));
                }
                if !allow_fallback_line {
                    continue;
                }
                return Some((rendered.value, type_name));
            }

            if allow_fallback_line {
                return Some((text.to_string(), Self::infer_debugger_value_type(text)));
            }
        }

        None
    }

    /// Parse evaluate output from recent debugger lines into a DAP result payload.
    fn parse_evaluate_result_from_output(&self, expression: &str) -> Option<(String, String)> {
        let lines = self.snapshot_recent_output_lines();
        Self::parse_evaluate_result_from_lines(&lines, expression, true)
    }

    /// Build deterministic placeholder variables used when debugger output is unavailable.
    fn fallback_scope_variables(variables_ref: i32) -> Vec<Variable> {
        match variables_ref % 10 {
            1 => vec![
                Variable {
                    name: "$self".to_string(),
                    value: "blessed(My::Module)".to_string(),
                    type_: Some("hash".to_string()),
                    variables_reference: variables_ref.saturating_mul(100) + 2,
                    named_variables: Some(5),
                    indexed_variables: None,
                },
                Variable {
                    name: "@_".to_string(),
                    value: "array(size=0)".to_string(),
                    type_: Some("array".to_string()),
                    variables_reference: variables_ref.saturating_mul(100) + 1,
                    named_variables: None,
                    indexed_variables: Some(0),
                },
            ],
            2 => vec![Variable {
                name: "$VERSION".to_string(),
                value: "\"1.0.0\"".to_string(),
                type_: Some("scalar".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            }],
            3 => vec![Variable {
                name: "$_".to_string(),
                value: "undef".to_string(),
                type_: Some("scalar".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            }],
            _ => Vec::new(),
        }
    }

    /// Handle initialize request
    fn handle_initialize(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        let supports_core = catalog_has_feature("dap.core");
        let supports_basic_breakpoints = catalog_has_feature("dap.breakpoints.basic");
        let supports_hit_conditions = catalog_has_feature("dap.breakpoints.hit_condition");
        let supports_log_points = catalog_has_feature("dap.breakpoints.logpoints");
        let supports_exceptions = catalog_has_feature("dap.exceptions.die");
        let supports_inline_values = catalog_has_feature("dap.inline_values");

        let exception_breakpoint_filters = if supports_exceptions {
            json!([
                {
                    "filter": "die",
                    "label": "Perl die() and uncaught exceptions",
                    "default": true
                },
                {
                    "filter": "all",
                    "label": "All Perl exception events",
                    "default": false
                }
            ])
        } else {
            json!([])
        };

        let capabilities = json!({
            "supportsConfigurationDoneRequest": supports_core,
            "supportsFunctionBreakpoints": supports_core,
            "supportsConditionalBreakpoints": supports_basic_breakpoints,
            "supportsHitConditionalBreakpoints": supports_hit_conditions,
            "supportsEvaluateForHovers": supports_core,
            "supportsStepBack": false,
            "supportsSetVariable": supports_core,
            "supportsRestartFrame": false,
            "supportsGotoTargetsRequest": false,
            "supportsStepInTargetsRequest": false,
            "supportsCompletionsRequest": false,
            "supportsModulesRequest": false,
            "supportsRestartRequest": false,
            "supportsExceptionOptions": supports_exceptions,
            "supportsValueFormattingOptions": supports_core,
            "supportsExceptionInfoRequest": false,
            "supportTerminateDebuggee": supports_core,
            "supportsDelayedStackTraceLoading": false,
            "supportsLoadedSourcesRequest": false,
            "supportsLogPoints": supports_log_points,
            "supportsTerminateThreadsRequest": false,
            "supportsSetExpression": false,
            "supportsTerminateRequest": supports_core,
            "supportsDataBreakpoints": false,
            "supportsReadMemoryRequest": false,
            "supportsDisassembleRequest": false,
            "supportsCancelRequest": false,
            "supportsBreakpointLocationsRequest": false,
            "supportsClipboardContext": false,
            "supportsSteppingGranularity": false,
            "supportsInstructionBreakpoints": false,
            "supportsExceptionFilterOptions": supports_exceptions,
            "supportsInlineValues": supports_inline_values,
            "exceptionBreakpointFilters": exception_breakpoint_filters
        });

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

            let env_overrides = args
                .get("env")
                .and_then(Value::as_object)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key.clone(), value.to_string()))
                        })
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default();

            // Launch Perl debugger
            match self.launch_debugger(program, perl_args, stop_on_entry, env_overrides) {
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
        env_overrides: HashMap<String, String>,
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
        cmd.envs(env_overrides);

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
                    last_resume_mode: ResumeMode::Unknown,
                };

                if let Ok(mut guard) = self.session.lock() {
                    *guard = Some(session);
                } else {
                    return Err("Failed to lock session".to_string());
                }

                // Apply any function breakpoints configured before launch.
                self.apply_stored_function_breakpoints();

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
        let recent_output = self.recent_output.clone();
        let breakpoints = self.breakpoints.clone();
        let exception_break_on_die = self.exception_break_on_die.clone();

        thread::spawn(move || {
            // Perl's debugger prompt and evaluation output are emitted on stderr.
            // Prefer stderr as the control stream, with stdout as a fallback.
            let control_stream: Option<Box<dyn Read + Send>> = {
                if let Ok(mut guard) = session.lock() {
                    guard.as_mut().and_then(|s| {
                        if let Some(stderr) = s.process.stderr.take() {
                            Some(Box::new(stderr) as Box<dyn Read + Send>)
                        } else {
                            s.process
                                .stdout
                                .take()
                                .map(|stdout| Box::new(stdout) as Box<dyn Read + Send>)
                        }
                    })
                } else {
                    eprintln!("Failed to lock session in output reader");
                    None
                }
            };

            let Some(control_stream) = control_stream else {
                eprintln!("No debugger output stream available - output reader thread exiting");
                // Send termination event
                if let Some(ref sender) = sender {
                    emit_event_safe(
                        sender,
                        &seq,
                        "terminated",
                        Some(json!({"reason": "no_debugger_stream"})),
                    );
                }
                return;
            };

            let mut reader = BufReader::new(control_stream);
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
                        {
                            let mut output = lock_or_recover(
                                &recent_output,
                                "debug_adapter.recent_output_reader",
                            );
                            if output.len() >= RECENT_OUTPUT_MAX_LINES {
                                let _ = output.pop_front();
                            }
                            output.push_back(text.clone());
                        }

                        // Send all output to client with error handling
                        if let Some(ref sender) = sender
                            && !emit_event_safe(
                                sender,
                                &seq,
                                "output",
                                Some(json!({
                                    "category": "stdout",
                                    "output": format!("{}\n", text)
                                })),
                            )
                        {
                            eprintln!("Failed to send output event - client may have disconnected");
                            break; // Exit the loop if client is gone
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
                            if let Some(ref sender) = sender {
                                emit_event_safe(
                                    sender,
                                    &seq,
                                    "output",
                                    Some(json!({
                                        "category": "stderr",
                                        "output": format!("Error: {}\n", text)
                                    })),
                                );
                            }
                        }

                        if context_updated {
                            let break_on_die =
                                exception_break_on_die.lock().map(|guard| *guard).unwrap_or(false);
                            let exception_match =
                                break_on_die && exception_re().is_some_and(|re| re.is_match(&text));

                            let mut should_emit_stopped = false;
                            let mut should_auto_continue = false;
                            let mut stop_reason = "step".to_string();
                            let mut logpoint_messages: Vec<String> = Vec::new();

                            let thread_id = {
                                let Ok(mut guard) = session.lock() else {
                                    eprintln!(
                                        "Failed to lock session when processing debugger context"
                                    );
                                    continue;
                                };

                                if let Some(ref mut s) = *guard {
                                    if !current_file.is_empty() && current_line > 0 {
                                        s.stack_frames = vec![StackFrame {
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
                                        }];
                                    }

                                    if matches!(s.state, DebugState::Running) {
                                        should_emit_stopped = true;
                                        let resume_mode = s.last_resume_mode.clone();

                                        let breakpoint_outcome =
                                            if matches!(resume_mode, ResumeMode::Continue)
                                                && !current_file.is_empty()
                                                && current_line > 0
                                            {
                                                breakpoints.register_breakpoint_hit(
                                                    &current_file,
                                                    i64::from(current_line),
                                                )
                                            } else {
                                                BreakpointHitOutcome::default()
                                            };

                                        if exception_match {
                                            stop_reason = "exception".to_string();
                                            s.state = DebugState::Stopped;
                                        } else if breakpoint_outcome.matched {
                                            logpoint_messages = breakpoint_outcome.log_messages;
                                            if breakpoint_outcome.should_stop {
                                                stop_reason = "breakpoint".to_string();
                                                s.state = DebugState::Stopped;
                                            } else {
                                                if let Some(stdin) = s.process.stdin.as_mut() {
                                                    let _ = stdin.write_all(b"c\n");
                                                    let _ = stdin.flush();
                                                }
                                                s.state = DebugState::Running;
                                                s.last_resume_mode = ResumeMode::Continue;
                                                should_auto_continue = true;
                                            }
                                        } else {
                                            s.state = DebugState::Stopped;
                                        }

                                        if !should_auto_continue {
                                            s.last_resume_mode = ResumeMode::Unknown;
                                        }
                                    }

                                    s.thread_id
                                } else {
                                    continue;
                                }
                            };

                            if let Some(ref sender) = sender {
                                for message in logpoint_messages {
                                    emit_event_safe(
                                        sender,
                                        &seq,
                                        "output",
                                        Some(json!({
                                            "category": "console",
                                            "output": format!("{message}\n")
                                        })),
                                    );
                                }
                            }

                            if should_auto_continue {
                                continue;
                            }

                            if should_emit_stopped
                                && let Some(ref sender) = sender
                                && !emit_event_safe(
                                    sender,
                                    &seq,
                                    "stopped",
                                    Some(json!({
                                        "reason": stop_reason,
                                        "threadId": thread_id,
                                        "allThreadsStopped": true
                                    })),
                                )
                            {
                                eprintln!("Failed to send stopped event - client disconnected");
                                return;
                            }
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
                            if let Some(ref sender) = sender
                                && !emit_event_safe(
                                    sender,
                                    &seq,
                                    "stopped",
                                    Some(json!({
                                        "reason": "step",
                                        "threadId": thread_id,
                                        "allThreadsStopped": true
                                    })),
                                )
                            {
                                eprintln!("Failed to send stopped event - client disconnected");
                                return; // Exit thread
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from debugger: {}", e);
                        // Send termination event before exiting
                        if let Some(ref sender) = sender {
                            emit_event_safe(
                                sender,
                                &seq,
                                "terminated",
                                Some(json!({"reason": "read_error", "error": e.to_string()})),
                            );
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
    /// 2. Process ID attachment - Signal-control mode for local Perl process
    ///
    /// For TCP attachment, the arguments should contain:
    /// - `host`: Hostname or IP address (default: "localhost")
    /// - `port`: Port number (default: 13603)
    /// - `timeout`: Connection timeout in milliseconds (optional)
    ///
    /// # Current Implementation
    ///
    /// TCP attachment is implemented with socket support.
    /// Process ID attachment is implemented in signal-control mode (pause/continue
    /// signaling and thread identity), with limited stack/evaluate capabilities
    /// unless a debugger transport is active.
    fn handle_attach(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        // Parse attach arguments
        if let Some(args) = arguments {
            let process_id = args.get("processId").and_then(|p| p.as_u64()).map(|p| p as u32);

            // PID attachment mode: best-effort process control without requiring TCP shim transport.
            if let Some(pid) = process_id {
                if pid == 0 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some("processId must be greater than zero".to_string()),
                    };
                }

                // Reset existing process/tcp attachment state before switching to PID mode.
                if let Ok(mut guard) = self.session.lock()
                    && let Some(mut existing) = guard.take()
                {
                    let _ = existing.process.kill();
                }
                if let Ok(mut guard) = self.tcp_session.lock()
                    && let Some(ref mut tcp_session) = *guard
                {
                    let _ = tcp_session.disconnect();
                }
                if let Ok(mut guard) = self.tcp_session.lock() {
                    *guard = None;
                }

                if let Ok(mut guard) = self.attached_pid.lock() {
                    *guard = Some(pid);
                }

                let thread_id = Self::i64_to_i32_saturating(i64::from(pid));
                self.send_event(
                    "stopped",
                    Some(json!({
                        "reason": "attach",
                        "threadId": thread_id,
                        "allThreadsStopped": true
                    })),
                );

                eprintln!(
                    "Attach request: Process ID attachment to PID {} (signal-control mode)",
                    pid
                );

                DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "attach".to_string(),
                    body: Some(json!({
                        "threadId": thread_id,
                        "processId": pid,
                        "mode": "processId"
                    })),
                    message: Some(
                        "Attached in signal-control mode. Stack/evaluate are limited without a \
                         debugger transport."
                            .to_string(),
                    ),
                }
            } else {
                // Extract host and port for TCP attachment.
                let host = args.get("host").and_then(|h| h.as_str()).unwrap_or("localhost");
                let raw_port = args.get("port").and_then(|p| p.as_u64()).unwrap_or(13603);
                if raw_port > 65535 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!("Port {raw_port} out of range (must be 1-65535)")),
                    };
                }
                let port = raw_port as u16;
                let timeout = args.get("timeout").and_then(|t| t.as_u64()).map(|t| t as u32);

                // Validate arguments.
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
                            message: Some(
                                "Timeout must be greater than 0 milliseconds".to_string(),
                            ),
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

                // TCP attachment mode (IMPLEMENTED)
                let mut config = TcpAttachConfig::new(host.to_string(), port);
                if let Some(t) = timeout {
                    config = config.with_timeout(t);
                }

                // Validate configuration
                if let Err(e) = config.validate() {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!("Invalid attach configuration: {}", e)),
                    };
                }

                // Create TCP attach session
                let mut session = TcpAttachSession::new();

                // Set up event channel for TCP events
                let (tx, rx) = channel::<DapEvent>();
                session.set_event_sender(tx);

                // Attempt to connect
                match session.connect(&config) {
                    Ok(()) => {
                        // Store session
                        if let Ok(mut guard) = self.tcp_session.lock() {
                            *guard = Some(session);
                        }

                        // Start reader thread
                        if let Ok(mut guard) = self.tcp_session.lock() {
                            if let Some(ref mut s) = *guard {
                                if let Err(e) = s.start_reader() {
                                    eprintln!("Failed to start TCP reader: {}", e);
                                    return DapMessage::Response {
                                        seq,
                                        request_seq,
                                        success: false,
                                        command: "attach".to_string(),
                                        body: None,
                                        message: Some(format!("Failed to start TCP reader: {}", e)),
                                    };
                                }
                            }
                        }

                        // Start event handler thread for TCP events
                        let seq_counter = self.seq.clone();
                        let event_sender = self.event_sender.clone();
                        thread::spawn(move || {
                            while let Ok(event) = rx.recv() {
                                match event {
                                    DapEvent::Output { category, output } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "output".to_string(),
                                                body: Some(json!({
                                                    "category": category,
                                                    "output": output
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Stopped { reason, thread_id } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "stopped".to_string(),
                                                body: Some(json!({
                                                    "reason": reason,
                                                    "threadId": thread_id,
                                                    "allThreadsStopped": true
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Continued { thread_id } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "continued".to_string(),
                                                body: Some(json!({
                                                    "threadId": thread_id,
                                                    "allThreadsContinued": true
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Terminated { reason } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "terminated".to_string(),
                                                body: Some(json!({
                                                    "reason": reason
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Error { message } => {
                                        eprintln!("TCP attach error: {}", message);
                                    }
                                }
                            }
                        });

                        DapMessage::Response {
                            seq,
                            request_seq,
                            success: true,
                            command: "attach".to_string(),
                            body: None,
                            message: None,
                        }
                    }
                    Err(e) => DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!(
                            "Failed to connect to {}:{} ({}ms timeout): {}",
                            config.host,
                            config.port,
                            config.timeout_ms.unwrap_or(30000),
                            e
                        )),
                    },
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

    /// Clear active process session, TCP session, and PID-attach mode state.
    fn clear_active_session_state(&self) {
        // Terminate the debug session
        if let Ok(mut guard) = self.session.lock()
            && let Some(mut session) = guard.take()
        {
            let _ = session.process.kill();
            session.state = DebugState::Terminated;
        }

        // Disconnect TCP session if active
        if let Ok(mut guard) = self.tcp_session.lock()
            && let Some(ref mut tcp_session) = *guard
        {
            let _ = tcp_session.disconnect();
        }
        if let Ok(mut guard) = self.tcp_session.lock() {
            *guard = None;
        }

        // Clear PID attach mode.
        if let Ok(mut guard) = self.attached_pid.lock() {
            *guard = None;
        }
    }

    /// Handle disconnect request
    fn handle_disconnect(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<DisconnectArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        self.clear_active_session_state();

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

    /// Handle terminate request
    fn handle_terminate(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: Option<TerminateArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        let restart = args.and_then(|a| a.restart);

        self.clear_active_session_state();

        let terminated_body = restart.map(|restart| json!({ "restart": restart }));
        self.send_event("terminated", terminated_body);

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "terminate".to_string(),
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
        let Some(args_value) = arguments else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setBreakpoints".to_string(),
                body: None,
                message: Some("Missing arguments".to_string()),
            };
        };

        let args: crate::protocol::SetBreakpointsArguments =
            match serde_json::from_value(args_value) {
                Ok(a) => a,
                Err(e) => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setBreakpoints".to_string(),
                        body: None,
                        message: Some(format!("Invalid arguments: {}", e)),
                    };
                }
            };

        // Snapshot old breakpoints for this file before replacing them,
        // so we can clear only per-file breakpoints instead of global `B *`.
        let old_breakpoints = if let Some(ref source_path) = args.source.path {
            self.breakpoints.get_breakpoints(source_path)
        } else {
            Vec::new()
        };

        // AC7: AST-based breakpoint validation via BreakpointStore
        let verified_breakpoints = self.breakpoints.set_breakpoints(&args);

        // If a session is active, also sync the breakpoints to the Perl debugger
        if let Ok(mut guard) = self.session.lock()
            && let Some(ref mut session) = *guard
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Clear only the old breakpoints for this specific file
                for old_bp in &old_breakpoints {
                    if old_bp.verified {
                        let cmd = format!("B {}\n", old_bp.line);
                        let _ = stdin.write_all(cmd.as_bytes());
                        let _ = stdin.flush();
                    }
                }

                // Set new breakpoints that were successfully verified
                for bp in &verified_breakpoints {
                    if bp.verified {
                        // Retrieve the record to get the original condition
                        let cmd = if let Some(record) = self.breakpoints.get_breakpoint_by_id(bp.id)
                            && let Some(cond) = record.condition
                        {
                            format!("b {} {}\n", bp.line, cond)
                        } else {
                            format!("b {}\n", bp.line)
                        };
                        let _ = stdin.write_all(cmd.as_bytes());
                        let _ = stdin.flush();
                    }
                }
            }
        }

        // Keep function breakpoints active after line-breakpoint synchronization.
        self.apply_stored_function_breakpoints();

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
    }

    /// Handle setFunctionBreakpoints request.
    ///
    /// Uses replace semantics and best-effort synchronization to the running debugger.
    fn handle_set_function_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetFunctionBreakpointsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setFunctionBreakpoints".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let requested = args.breakpoints;

        let mut validated_names = Vec::with_capacity(requested.len());
        let mut response_breakpoints = Vec::with_capacity(requested.len());

        for entry in requested {
            let name = entry.name.trim().to_string();

            let id = {
                let mut next = lock_or_recover(
                    &self.next_function_breakpoint_id,
                    "debug_adapter.next_function_breakpoint_id",
                );
                let id = *next;
                *next += 1;
                id
            };

            let invalid_reason = if name.is_empty() {
                Some("Function breakpoint name is required".to_string())
            } else if name.contains('\n') || name.contains('\r') {
                Some("Function breakpoint name cannot contain newlines".to_string())
            } else if !is_valid_function_breakpoint_name(&name) {
                Some(format!(
                    "Invalid function breakpoint name `{name}` (expected package-qualified Perl symbol)"
                ))
            } else {
                None
            };

            if let Some(reason) = invalid_reason {
                response_breakpoints.push(json!({
                    "id": id,
                    "verified": false,
                    "message": reason
                }));
                continue;
            }

            validated_names.push(name.clone());
            response_breakpoints.push(json!({
                "id": id,
                "verified": true
            }));
        }

        // DAP replace semantics: overwrite existing function breakpoints.
        if let Ok(mut stored) = self.function_breakpoints.lock() {
            *stored = validated_names;
        }

        // Best-effort apply to currently running session as well.
        self.apply_stored_function_breakpoints();

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setFunctionBreakpoints".to_string(),
            body: Some(json!({ "breakpoints": response_breakpoints })),
            message: None,
        }
    }

    /// Handle setExceptionBreakpoints request.
    ///
    /// Supports `die`/uncaught exception breaks via output classification in the
    /// debugger reader thread.
    fn handle_set_exception_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let mut break_on_die = false;

        if let Some(args) = arguments
            .and_then(|v| serde_json::from_value::<SetExceptionBreakpointsArguments>(v).ok())
        {
            break_on_die = args.filters.iter().any(|filter| {
                filter.eq_ignore_ascii_case("die") || filter.eq_ignore_ascii_case("all")
            });

            if !break_on_die {
                if let Some(filter_options) = args.filter_options {
                    break_on_die = filter_options.iter().any(|entry| {
                        entry.filter_id.eq_ignore_ascii_case("die")
                            || entry.filter_id.eq_ignore_ascii_case("all")
                    });
                }
            }
        }

        if let Ok(mut guard) = self.exception_break_on_die.lock() {
            *guard = break_on_die;
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setExceptionBreakpoints".to_string(),
            body: Some(json!({ "breakpoints": [] })),
            message: None,
        }
    }

    /// Apply stored function breakpoints to the active debugger session.
    fn apply_stored_function_breakpoints(&self) {
        let names =
            self.function_breakpoints.lock().map(|stored| stored.clone()).unwrap_or_default();
        if names.is_empty() {
            return;
        }

        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            for name in names {
                let cmd = format!("b {name}\n");
                let _ = stdin.write_all(cmd.as_bytes());
            }
            let _ = stdin.flush();
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
        let threads = if let Some(ref session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            vec![json!({
                "id": session.thread_id,
                "name": "Main Thread"
            })]
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            vec![json!({
                "id": Self::i64_to_i32_saturating(i64::from(pid)),
                "name": format!("Attached Process ({pid})")
            })]
        } else if lock_or_recover(&self.tcp_session, "debug_adapter.tcp_session").is_some() {
            vec![json!({ "id": 1, "name": "TCP Attached Thread" })]
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
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<StackTraceArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());
        let mut framed_output_lines = None;

        // Ask the debugger for an explicit stack snapshot when a live session is present.
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let commands = vec!["T".to_string()];
            match self.send_framed_debugger_commands(stdin, &commands) {
                Ok((begin, end)) => {
                    framed_output_lines = self.capture_framed_debugger_output(
                        &begin,
                        &end,
                        DEBUGGER_QUERY_WAIT_MS * 8,
                    );
                }
                Err(error) => {
                    eprintln!("Failed to send framed stackTrace command, falling back: {error}");
                    let _ = stdin.write_all(b"T\n");
                    let _ = stdin.flush();
                    Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                }
            }
        }

        let parsed_frames = if let Some(lines) = framed_output_lines.as_ref() {
            let output = lines.join("\n");
            let framed_frames =
                Self::filter_user_visible_frames(Self::parse_stack_frames_from_text(&output));
            if framed_frames.is_empty() {
                let output_lines = self.snapshot_recent_output_lines();
                if output_lines.is_empty() {
                    Vec::new()
                } else {
                    let output = output_lines.join("\n");
                    Self::filter_user_visible_frames(Self::parse_stack_frames_from_text(&output))
                }
            } else {
                framed_frames
            }
        } else {
            let output_lines = self.snapshot_recent_output_lines();
            if output_lines.is_empty() {
                Vec::new()
            } else {
                let output = output_lines.join("\n");
                Self::filter_user_visible_frames(Self::parse_stack_frames_from_text(&output))
            }
        };

        let stack_frames = if !parsed_frames.is_empty() {
            // Keep parsed frames as best-effort latest snapshot.
            if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            {
                session.stack_frames = parsed_frames.clone();
            }
            parsed_frames
        } else if let Some(ref session) = *lock_or_recover(&self.session, "debug_adapter.session") {
            Self::filter_user_visible_frames(session.stack_frames.clone())
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            vec![StackFrame {
                id: Self::i64_to_i32_saturating(i64::from(pid)),
                name: format!("attached::process::{pid}"),
                source: Source {
                    name: Some(format!("pid:{pid}")),
                    path: format!("pid://{pid}"),
                    source_reference: None,
                },
                line: 1,
                column: 1,
                end_line: None,
                end_column: None,
            }]
        } else {
            // No session - return placeholder frame for testing
            vec![StackFrame {
                id: 1,
                name: "main::hello".to_string(),
                source: Source {
                    name: Some("hello.pl".to_string()),
                    path: "/tmp/hello.pl".to_string(),
                    source_reference: None,
                },
                line: 10,
                column: 1,
                end_line: None,
                end_column: None,
            }]
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
        let args: ScopesArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "scopes".to_string(),
                    body: None,
                    message: Some("Missing frameId".to_string()),
                };
            }
        };

        let frame_id = args.frame_id as i32;

        // AC8.3: Hierarchical scope inspection
        // Use bit-shifting or offsets to distinguish between scope types for the same frame
        let locals_ref = frame_id * 10 + 1;
        let package_ref = frame_id * 10 + 2;
        let globals_ref = frame_id * 10 + 3;

        let scopes_body = ScopesResponseBody {
            scopes: vec![
                Scope {
                    name: "Locals".to_string(),
                    presentation_hint: Some("locals".to_string()),
                    variables_reference: i64::from(locals_ref),
                    expensive: false,
                },
                Scope {
                    name: "Package".to_string(),
                    presentation_hint: None,
                    variables_reference: i64::from(package_ref),
                    expensive: true,
                },
                Scope {
                    name: "Globals".to_string(),
                    presentation_hint: None,
                    variables_reference: i64::from(globals_ref),
                    expensive: true,
                },
            ],
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "scopes".to_string(),
            body: serde_json::to_value(&scopes_body).ok(),
            message: None,
        }
    }

    /// Handle variables request
    fn handle_variables(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let args: VariablesArguments = match arguments.and_then(|v| serde_json::from_value(v).ok())
        {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "variables".to_string(),
                    body: None,
                    message: Some("Missing arguments".to_string()),
                };
            }
        };

        let variables_ref = args.variables_reference as i32;
        let start = args.start.unwrap_or(0) as usize;
        let count = args.count.map(|v| v as usize).unwrap_or(256).clamp(1, 1024);

        if variables_ref == 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "variables".to_string(),
                body: None,
                message: Some("Missing variablesReference".to_string()),
            };
        }

        // AC8.4: Render scalars/arrays/hashes with lazy child expansion.
        let parsed_from_output;
        let mut parsed_child_cache = HashMap::new();
        let mut used_session_cache = false;

        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session") {
            // Return cached variables first for stable references and fast repeated expansion.
            if let Some(vars) = session.variables.get(&variables_ref) {
                used_session_cache = true;
                parsed_from_output = vars.clone();
            } else {
                let mut framed_scope_lines = None;

                // Request fresh scope output from Perl debugger for scope roots only.
                let frame_id = variables_ref / 10;
                match variables_ref % 10 {
                    1 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} .", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to send framed variables command, falling back: {error}"
                                    );
                                    let cmd = format!("V {} .\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    2 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} ::", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to send framed variables command, falling back: {error}"
                                    );
                                    let cmd = format!("V {} ::\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    3 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} *", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to send framed variables command, falling back: {error}"
                                    );
                                    let cmd = format!("V {} *\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    _ => {}
                }

                let (vars, child_cache) = if let Some(lines) = framed_scope_lines.as_ref() {
                    let (framed_vars, framed_child_cache) =
                        Self::parse_scope_variables_from_lines(lines, variables_ref, start, count);
                    if framed_vars.is_empty() {
                        Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                        self.parse_scope_variables_from_output(variables_ref, start, count)
                    } else {
                        (framed_vars, framed_child_cache)
                    }
                } else {
                    Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                    self.parse_scope_variables_from_output(variables_ref, start, count)
                };

                parsed_from_output = vars;
                parsed_child_cache = child_cache;
            }
        } else {
            let (vars, _child_cache) =
                self.parse_scope_variables_from_output(variables_ref, start, count);
            parsed_from_output = vars;
        }

        let variables = if parsed_from_output.is_empty() {
            Self::fallback_scope_variables(variables_ref)
        } else {
            parsed_from_output
        };

        // Cache parsed variables and generated child references for expansion requests.
        if !used_session_cache
            && let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
        {
            session.variables.insert(variables_ref, variables.clone());
            for (reference, children) in parsed_child_cache {
                session.variables.insert(reference, children);
            }
        }

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
    }

    /// Handle setVariable request
    fn handle_set_variable(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetVariableArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setVariable".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let variables_ref = args.variables_reference;
        if variables_ref <= 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variablesReference".to_string()),
            };
        }

        let name = args.name.trim().to_string();
        let value = args.value.trim().to_string();
        let name = name.as_str();
        let value = value.as_str();

        if name.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variable name".to_string()),
            };
        }

        if value.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variable value".to_string()),
            };
        }

        if name.contains('\n')
            || name.contains('\r')
            || value.contains('\n')
            || value.contains('\r')
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Variable name/value cannot contain newlines".to_string()),
            };
        }

        if !is_valid_set_variable_name(name) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "Invalid variable name `{name}` for setVariable (expected Perl sigil-prefixed variable)"
                )),
            };
        }

        let output_frame_markers = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Frame assignment + read-back so output parsing is deterministic.
                let commands = vec![format!("p {name} = {value}"), format!("p {name}")];
                match self.send_framed_debugger_commands(stdin, &commands) {
                    Ok(markers) => Some(markers),
                    Err(error) => {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "setVariable".to_string(),
                            body: None,
                            message: Some(format!("Failed to send setVariable command: {error}")),
                        };
                    }
                }
            } else {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "setVariable".to_string(),
                    body: None,
                    message: Some("No debugger session active".to_string()),
                };
            }
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "setVariable is unavailable for processId attach (PID {pid}) without an active debugger transport"
                )),
            };
        } else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("No debugger session".to_string()),
            };
        };

        let parsed = output_frame_markers
            .as_ref()
            .and_then(|(begin, end)| {
                self.capture_framed_debugger_output(begin, end, DEBUGGER_QUERY_WAIT_MS * 8)
            })
            .and_then(|lines| Self::parse_evaluate_result_from_lines(&lines, "", true));

        let Some((rendered_value, rendered_type)) = parsed else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "setVariable read-back for `{name}` produced no parseable output"
                )),
            };
        };

        let set_var_body = SetVariableResponseBody {
            value: rendered_value,
            type_: Some(rendered_type),
            variables_reference: 0,
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setVariable".to_string(),
            body: serde_json::to_value(&set_var_body).ok(),
            message: None,
        }
    }

    /// Handle continue request
    fn handle_continue(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let _args: Option<ContinueArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        let mut thread_id = 1;
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"c\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::Continue;
            session.variables.clear();
            thread_id = session.thread_id;
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            let _ = self.send_continue_signal(pid);
            thread_id = Self::i64_to_i32_saturating(i64::from(pid));
        }

        // AC9.4: Proper DAP event emission: continued
        self.send_event(
            "continued",
            Some(json!({
                "threadId": thread_id,
                "allThreadsContinued": true
            })),
        );

        let continue_body = ContinueResponseBody { all_threads_continued: true };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "continue".to_string(),
            body: serde_json::to_value(&continue_body).ok(),
            message: None,
        }
    }

    /// Handle next request
    fn handle_next(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let _args: Option<NextArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"n\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::Next;
            session.variables.clear();
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
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
    fn handle_step_in(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let _args: Option<StepInArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"s\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::StepIn;
            session.variables.clear();
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
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
    fn handle_step_out(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let _args: Option<StepOutArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"r\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::StepOut;
            session.variables.clear();
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
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
    fn handle_pause(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let _args: Option<PauseArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        let success = if let Some(ref session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            let pid = session.process.id();
            self.send_interrupt_signal(pid)
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
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

    /// Parse stack trace output from Perl debugger "T" command
    ///
    /// AC8.2: Parse caller() + %DB::sub data from Perl debugger
    ///
    /// The Perl debugger "T" command outputs stack traces in formats like:
    /// ```text
    /// $ = main::compute_sum() called from file /app/main.pl line 15
    /// $ = main::process_data() called from file /app/main.pl line 10
    /// ```
    ///
    /// Or with frame numbers:
    /// ```text
    /// # 0 main::helper at /app/script.pl line 20
    /// # 1 Foo::bar called at /app/lib/Foo.pm line 15
    /// # 2 main::start at /app/script.pl line 5
    /// ```
    ///
    /// Returns a vector of StackFrame structs with accurate line numbers,
    /// source paths, and package-qualified function names.
    #[cfg(test)]
    fn parse_stack_trace(output: &str) -> Vec<StackFrame> {
        let mut frames = Vec::new();
        let mut frame_id = 1;

        for line in output.lines() {
            // Try to match stack frame format
            if let Some(re) = stack_frame_re() {
                if let Some(caps) = re.captures(line) {
                    let func = caps.name("func").map(|m| m.as_str()).unwrap_or("main");
                    let file = caps.name("file").map(|m| m.as_str()).unwrap_or("<unknown>");
                    let line_num =
                        caps.name("line").and_then(|m| m.as_str().parse::<i32>().ok()).unwrap_or(1);

                    // Extract file name from path for display
                    let file_name = file.split(['/', '\\'].as_ref()).next_back().unwrap_or(file);

                    frames.push(StackFrame {
                        id: frame_id,
                        name: func.to_string(),
                        source: Source {
                            name: Some(file_name.to_string()),
                            path: file.to_string(),
                            source_reference: None,
                        },
                        line: line_num,
                        column: 1, // Perl debugger doesn't provide column info by default
                        end_line: None,
                        end_column: None,
                    });

                    frame_id += 1;
                }
            }
        }

        frames
    }
}

/// Check if a position in a string is inside single quotes
/// (conservative: only tracks single-quoted string literals)
fn is_in_single_quotes(s: &str, idx: usize) -> bool {
    let mut in_sq = false;
    let mut escaped = false;

    for (i, ch) in s.char_indices() {
        if i >= idx {
            break;
        }
        if in_sq {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_sq = false;
            }
        } else if ch == '\'' {
            in_sq = true;
        }
    }

    in_sq
}

/// Check if the match is CORE:: or CORE::GLOBAL:: qualified (must block these)
fn is_core_qualified(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();

    // Must have :: immediately before op
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }

    // Extract the identifier right before that ::
    let end = op_start - 2;
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    let seg = &s[start..end];
    if seg == "CORE" {
        return true;
    }
    if seg != "GLOBAL" {
        return false;
    }

    // If GLOBAL, require CORE::GLOBAL::op
    if start < 2 || bytes[start - 1] != b':' || bytes[start - 2] != b':' {
        return false;
    }
    let end2 = start - 2;
    let mut start2 = end2;
    while start2 > 0 {
        let b = bytes[start2 - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start2 -= 1;
        } else {
            break;
        }
    }
    &s[start2..end2] == "CORE"
}

/// Check if the match is a sigil-prefixed identifier ($print, @say, %exit, *dump)
/// BUT NOT if it's a dereference call (&$print) or method call (->$print)
fn is_sigil_prefixed_identifier(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start == 0 {
        return false;
    }

    // Must be preceded by a sigil
    if !matches!(bytes[op_start - 1], b'$' | b'@' | b'%' | b'*') {
        return false;
    }

    // Security: If it's a sigil, we must ensure it's not being used in a way
    // that triggers execution (like &$sub or ->$method).
    // We scan backwards from the sigil (op_start - 1) skipping whitespace.
    let mut i = op_start - 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }

    if i > 0 {
        let prev = bytes[i - 1];

        // Block dereference execution (&$sub)
        if prev == b'&' {
            return false;
        }

        // Block method call (->$method)
        if prev == b'>' && i > 1 && bytes[i - 2] == b'-' {
            return false;
        }

        // Handle braced dereference &{ $sub }
        if prev == b'{' {
            i -= 1;
            while i > 0 && bytes[i - 1].is_ascii_whitespace() {
                i -= 1;
            }
            if i > 0 && bytes[i - 1] == b'&' {
                return false;
            }
        }
    }

    true
}

/// Check if the match is a simple braced scalar variable ${print}
/// Does NOT skip ${print()} or ${print + 1}
fn is_simple_braced_scalar_var(s: &str, op_start: usize, op_end: usize) -> bool {
    let bytes = s.as_bytes();

    // Scan left for `${` (allow whitespace between)
    let mut i = op_start;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i < 1 || bytes[i - 1] != b'{' {
        return false;
    }
    i -= 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i < 1 || bytes[i - 1] != b'$' {
        return false;
    }

    // Scan right for `}` (allow whitespace between)
    let mut j = op_end;
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    j < bytes.len() && bytes[j] == b'}'
}

/// Check if the match is package-qualified (Foo::print) but not CORE::
fn is_package_qualified_not_core(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }
    // It's qualified, but we need to check it's not CORE::
    !is_core_qualified(s, op_start)
}

/// Validate that an expression is safe for evaluation (non-mutating)
///
/// AC10.2: Safe evaluation mode validates expressions don't have side effects
///
/// This function uses a pre-compiled regex for performance and includes
/// context-aware filtering to reduce false positives for:
/// - Sigil-prefixed identifiers ($print, @say, %exit)
/// - Simple braced scalar variables ${print}
/// - Package-qualified names (Foo::print) unless CORE::
/// - Single-quoted string literals ('print')
///
/// Note: Method calls ($obj->print) are intentionally NOT exempted because
/// dangerous operations remain dangerous regardless of invocation syntax.
fn validate_safe_expression(expression: &str) -> Option<String> {
    // Check for assignment operators using regex to properly handle multi-char ops
    // This avoids false positives for comparison operators (e.g., == contains =)
    if let Some(re) = assignment_ops_re() {
        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();

            // Allow harmless occurrences in single-quoted literals
            if is_in_single_quotes(expression, start) {
                continue;
            }

            // Check if it's strictly an assignment operator
            match op {
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | ".=" | "&=" | "|=" | "^="
                | "<<=" | ">>=" | "&&=" | "||=" | "//=" | "x=" => {
                    return Some(format!(
                        "Safe evaluation mode: assignment operator '{}' not allowed (use allowSideEffects: true)",
                        op
                    ));
                }
                _ => {}
            }
        }
    }

    // Check for dynamic subroutine calls &{...}
    // This blocks tricks like &{"sys"."tem"}("ls")
    if let Some(re) = deref_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: dynamic subroutine calls (&{...}) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    // Check for glob operations <*...>
    // This blocks filesystem access via globs
    if let Some(re) = glob_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: glob operations (<*...>) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    // Check for mutating operations using pre-compiled regex
    if let Some(re) = dangerous_ops_re() {
        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();
            let end = mat.end();

            // Allow harmless occurrences in single-quoted literals
            if is_in_single_quotes(expression, start) {
                continue;
            }

            // Allow sigil-prefixed identifiers ($print, @say, %exit, *printf)
            if is_sigil_prefixed_identifier(expression, start) {
                continue;
            }

            // Allow ${print} (simple scalar braced variable form)
            if is_simple_braced_scalar_var(expression, start, end) {
                continue;
            }

            // Allow package-qualified names unless it's CORE::
            if is_package_qualified_not_core(expression, start) {
                continue;
            }

            // Block: either bare op or CORE:: qualified
            return Some(format!(
                "Safe evaluation mode: potentially mutating operation '{}' not allowed (use allowSideEffects: true)",
                op
            ));
        }
    }

    // Check for regex mutation operators (s///, tr///, y///)
    // Handled separately to avoid false positives with escape sequences like \s in /\s+/
    if let Some(re) = regex_mutation_re() {
        if let Some(mat) = re.find(expression) {
            let op = mat.as_str();
            let start = mat.start();

            // Allow sigil-prefixed identifiers ($s, $tr, $y)
            if is_sigil_prefixed_identifier(expression, start) {
                // It's a variable, allow it
            } else if is_escape_sequence(expression, start) {
                // It's an escape sequence like \s or \y, allow it
            } else {
                return Some(format!(
                    "Safe evaluation mode: regex mutation operator '{}' not allowed (use allowSideEffects: true)",
                    op.trim()
                ));
            }
        }
    }

    // Check for increment/decrement operators
    if expression.contains("++") || expression.contains("--") {
        return Some(
            "Safe evaluation mode: increment/decrement operators not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    // Check for backticks (shell execution)
    if expression.contains('`') {
        return Some(
            "Safe evaluation mode: backticks (shell execution) not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    None
}

impl DebugAdapter {
    /// Send continue/resume signal to process (Unix only)
    #[allow(unused_variables)]
    fn send_continue_signal(&self, pid: u32) -> bool {
        #[cfg(unix)]
        {
            let pid = pid as i32;
            match signal::kill(Pid::from_raw(pid), Signal::SIGCONT) {
                Ok(()) => {
                    eprintln!("Sent SIGCONT to process {}", pid);
                    true
                }
                Err(e) => {
                    eprintln!("Failed to send SIGCONT to process {}: {}", pid, e);
                    false
                }
            }
        }
        #[cfg(not(unix))]
        {
            eprintln!("Continue signal not supported on this platform");
            false
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

    /// Handle evaluate request with safe evaluation mode and timeout enforcement
    ///
    /// AC10.1: Evaluates expressions in stack frame context
    /// AC10.2: Safe evaluation mode (non-mutating) by default
    /// AC10.3: Timeout enforcement (5s default, 30s hard limit)
    fn handle_evaluate(&self, seq: i64, request_seq: i64, arguments: Option<Value>) -> DapMessage {
        let args: EvaluateArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Missing arguments".to_string()),
                };
            }
        };

        {
            let expression = &args.expression;

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

            // AC10.2: Safe evaluation mode (non-mutating) by default
            let allow_side_effects = args.allow_side_effects.unwrap_or(false);

            // Validate expression safety if side effects are not allowed
            if !allow_side_effects {
                if let Some(error) = validate_safe_expression(expression) {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some(error),
                    };
                }

                // Re-run through microcrate validator to keep evaluation policy aligned
                // with shared DAP security logic.
                let evaluator = SafeEvaluator::new();
                if let Err(error) = evaluator.validate(expression) {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some(error.to_string()),
                    };
                }
            }
        }

        let expression = &args.expression;

        // AC10.3: Get timeout configuration (5s default, 30s hard limit)
        let timeout_ms = 5000u32;
        let timeout_ms = timeout_ms.min(30000); // Enforce 30s hard limit

        // Send evaluation command to debugger
        let output_frame_markers = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Frame debugger output so evaluate parsing only considers this request's output.
                let commands = vec![format!("x {expression}")];
                match self.send_framed_debugger_commands(stdin, &commands) {
                    Ok(markers) => Some(markers),
                    Err(error) => {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "evaluate".to_string(),
                            body: None,
                            message: Some(format!("Failed to send evaluate command: {error}")),
                        };
                    }
                }
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
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some(format!(
                    "Evaluate is unavailable for processId attach (PID {pid}) without an active debugger transport"
                )),
            };
        } else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some("No debugger session".to_string()),
            };
        };

        let framed_lines = output_frame_markers.as_ref().and_then(|(begin, end)| {
            self.capture_framed_debugger_output(begin, end, u64::from(timeout_ms))
        });

        let parsed = framed_lines
            .as_ref()
            .and_then(|lines| Self::parse_evaluate_result_from_lines(lines, expression, true))
            .or_else(|| self.parse_evaluate_result_from_output(expression));

        let (result, result_type) = parsed.unwrap_or_else(|| {
            (
                format!("<evaluating: {}> (timeout: {}ms)", expression, timeout_ms),
                "string".to_string(),
            )
        });

        let eval_body =
            EvaluateResponseBody { result, type_: Some(result_type), variables_reference: 0 };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "evaluate".to_string(),
            body: serde_json::to_value(&eval_body).ok(),
            message: None,
        }
    }

    /// Handle inlineValues request (custom)
    fn handle_inline_values(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let Some(args) = arguments else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("Missing arguments".to_string()),
            };
        };

        let args: InlineValuesArguments = match serde_json::from_value(args) {
            Ok(parsed) => parsed,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "inlineValues".to_string(),
                    body: None,
                    message: Some(format!("Invalid arguments: {}", e)),
                };
            }
        };

        let Some(source_path) = args.source.path else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("inlineValues requires source.path".to_string()),
            };
        };

        if args.start_line <= 0 || args.end_line <= 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("inlineValues requires positive startLine/endLine".to_string()),
            };
        }

        let start_line = args.start_line.min(args.end_line);
        let end_line = args.end_line.max(args.start_line);
        let content = match std::fs::read_to_string(&source_path) {
            Ok(content) => content,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "inlineValues".to_string(),
                    body: None,
                    message: Some(format!("Failed to read source file: {}", e)),
                };
            }
        };

        let inline_values = collect_inline_values(&content, start_line, end_line);
        let body = InlineValuesResponseBody { inline_values };

        match serde_json::to_value(&body) {
            Ok(body) => DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "inlineValues".to_string(),
                body: Some(body),
                message: None,
            },
            Err(e) => DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some(format!("Failed to serialize inlineValues response: {}", e)),
            },
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
        assert!(adapter.breakpoints.is_empty());
    }

    #[test]
    fn test_sequence_numbers() {
        let adapter = DebugAdapter::new();
        assert_eq!(adapter.next_seq(), 1);
        assert_eq!(adapter.next_seq(), 2);
        assert_eq!(adapter.next_seq(), 3);
    }

    #[test]
    fn test_initialize_response() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(1, "initialize", None);

        match response {
            DapMessage::Response { success, command, body, .. } => {
                assert!(success);
                assert_eq!(command, "initialize");
                assert!(body.is_some());
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_initialize_capabilities_follow_feature_catalog()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let init = adapter.handle_request(1, "initialize", None);

        let capabilities = match init {
            DapMessage::Response { success: true, command, body: Some(body), .. }
                if command == "initialize" =>
            {
                body
            }
            _ => return Err("Expected successful initialize response".into()),
        };

        let capability_map =
            capabilities.as_object().ok_or("Initialize response body must be a JSON object")?;

        let expectations = [
            ("supportsConfigurationDoneRequest", crate::feature_catalog::has_feature("dap.core")),
            ("supportsFunctionBreakpoints", crate::feature_catalog::has_feature("dap.core")),
            (
                "supportsConditionalBreakpoints",
                crate::feature_catalog::has_feature("dap.breakpoints.basic"),
            ),
            (
                "supportsHitConditionalBreakpoints",
                crate::feature_catalog::has_feature("dap.breakpoints.hit_condition"),
            ),
            ("supportsEvaluateForHovers", crate::feature_catalog::has_feature("dap.core")),
            ("supportsSetVariable", crate::feature_catalog::has_feature("dap.core")),
            ("supportsValueFormattingOptions", crate::feature_catalog::has_feature("dap.core")),
            ("supportTerminateDebuggee", crate::feature_catalog::has_feature("dap.core")),
            ("supportsLogPoints", crate::feature_catalog::has_feature("dap.breakpoints.logpoints")),
            ("supportsExceptionOptions", crate::feature_catalog::has_feature("dap.exceptions.die")),
            (
                "supportsExceptionFilterOptions",
                crate::feature_catalog::has_feature("dap.exceptions.die"),
            ),
            ("supportsInlineValues", crate::feature_catalog::has_feature("dap.inline_values")),
            ("supportsTerminateRequest", crate::feature_catalog::has_feature("dap.core")),
        ];

        for (capability, expected) in expectations {
            let actual = capability_map
                .get(capability)
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("Capability `{capability}` must be present as boolean"))?;
            assert_eq!(
                actual, expected,
                "Capability `{capability}` must mirror features.toml advertisement"
            );
        }

        let exception_filters = capability_map
            .get("exceptionBreakpointFilters")
            .and_then(Value::as_array)
            .ok_or("exceptionBreakpointFilters must be present as an array")?;
        if crate::feature_catalog::has_feature("dap.exceptions.die") {
            assert!(
                !exception_filters.is_empty(),
                "Exception filters should be advertised when dap.exceptions.die is enabled"
            );
        } else {
            assert!(
                exception_filters.is_empty(),
                "Exception filters should be empty when dap.exceptions.die is not advertised"
            );
        }

        Ok(())
    }

    #[test]
    fn test_initialize_capabilities_are_backed_by_handlers()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let init = adapter.handle_request(1, "initialize", None);

        let capabilities = match init {
            DapMessage::Response { success: true, command, body: Some(body), .. }
                if command == "initialize" =>
            {
                body
            }
            _ => return Err("Expected successful initialize response".into()),
        };

        let capability_map =
            capabilities.as_object().ok_or("Initialize response body must be a JSON object")?;

        let capability_to_command = [
            ("supportsConfigurationDoneRequest", "configurationDone"),
            ("supportsFunctionBreakpoints", "setFunctionBreakpoints"),
            ("supportsConditionalBreakpoints", "setBreakpoints"),
            ("supportsHitConditionalBreakpoints", "setBreakpoints"),
            ("supportsEvaluateForHovers", "evaluate"),
            ("supportsSetVariable", "setVariable"),
            ("supportsValueFormattingOptions", "variables"),
            ("supportsLogPoints", "setBreakpoints"),
            ("supportsExceptionOptions", "setExceptionBreakpoints"),
            ("supportsExceptionFilterOptions", "setExceptionBreakpoints"),
            ("supportsInlineValues", "inlineValues"),
            ("supportsTerminateRequest", "terminate"),
            ("supportTerminateDebuggee", "terminate"),
        ];

        let mut mapped_commands = HashSet::new();
        for (capability, raw_value) in capability_map {
            let is_support_flag =
                capability.starts_with("supports") || capability == "supportTerminateDebuggee";
            if !is_support_flag || !raw_value.as_bool().unwrap_or(false) {
                continue;
            }

            let command = capability_to_command
                .iter()
                .find_map(|(supported, command)| (*supported == capability).then_some(*command))
                .ok_or_else(|| {
                    format!(
                        "Capability `{capability}` is true but has no handler mapping in this invariant test"
                    )
                })?;

            let _ = mapped_commands.insert(command);
        }

        let mut request_seq = 2;
        for command in mapped_commands {
            let arguments = match command {
                "configurationDone" => Some(json!({})),
                "setFunctionBreakpoints" => {
                    Some(json!({"breakpoints": [{ "name": "main::noop" }]}))
                }
                "setBreakpoints" => Some(json!({
                    "source": { "path": "/tmp/capability_honesty.pl" },
                    "breakpoints": [{ "line": 1, "hitCondition": ">= 1", "logMessage": "breakpoint hit" }]
                })),
                "setExceptionBreakpoints" => Some(json!({"filters": ["die"]})),
                "evaluate" => Some(json!({"expression": "$x", "allowSideEffects": true})),
                "setVariable" => {
                    Some(json!({"variablesReference": 11, "name": "$x", "value": "1"}))
                }
                "variables" => Some(json!({"variablesReference": 11})),
                "inlineValues" => Some(json!({
                    "source": { "path": "/tmp/capability_honesty.pl" },
                    "startLine": 1,
                    "endLine": 1
                })),
                "terminate" => Some(json!({"restart": false})),
                _ => None,
            };

            let response = adapter.handle_request(request_seq, command, arguments);
            request_seq += 1;

            match response {
                DapMessage::Response { command: actual, message, .. } => {
                    assert_eq!(
                        actual, command,
                        "Capability-mapped command `{command}` must route to its handler"
                    );
                    let message_text = message.unwrap_or_default();
                    assert!(
                        !message_text.contains("Unknown command"),
                        "Capability-mapped command `{command}` must not hit unknown-command path"
                    );
                }
                _ => return Err(format!("Expected response for `{command}`").into()),
            }
        }

        Ok(())
    }

    #[test]
    fn test_set_exception_breakpoints_toggles_die_filter() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut adapter = DebugAdapter::new();

        assert!(
            !*lock_or_recover(
                &adapter.exception_break_on_die,
                "test_set_exception_breakpoints.initial"
            ),
            "die filter should default to disabled"
        );

        let response = adapter.handle_request(
            1,
            "setExceptionBreakpoints",
            Some(json!({
                "filters": ["die"]
            })),
        );
        match response {
            DapMessage::Response { success: true, command, .. } => {
                assert_eq!(command, "setExceptionBreakpoints");
            }
            _ => return Err("Expected successful setExceptionBreakpoints response".into()),
        }

        assert!(
            *lock_or_recover(
                &adapter.exception_break_on_die,
                "test_set_exception_breakpoints.enabled"
            ),
            "die filter should be enabled after request"
        );

        let disable = adapter.handle_request(
            2,
            "setExceptionBreakpoints",
            Some(json!({
                "filters": []
            })),
        );
        match disable {
            DapMessage::Response { success: true, command, .. } => {
                assert_eq!(command, "setExceptionBreakpoints");
            }
            _ => return Err("Expected successful setExceptionBreakpoints response".into()),
        }

        assert!(
            !*lock_or_recover(
                &adapter.exception_break_on_die,
                "test_set_exception_breakpoints.disabled"
            ),
            "die filter should be disabled when no matching filters are configured"
        );

        Ok(())
    }

    #[test]
    fn test_attach_missing_arguments() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(1, "attach", None);

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "attach");
                assert!(message.is_some());
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Missing attach arguments"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_tcp_valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("localhost:13603"));
                assert!(msg.contains("5000ms timeout"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_process_id_mode() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let args = json!({
            "processId": 12345
        });
        let response = adapter.handle_request(1, "attach", Some(args));

        match response {
            DapMessage::Response { success, command, body, message, .. } => {
                assert!(success);
                assert_eq!(command, "attach");
                assert!(body.is_some());
                let body = body.ok_or("Expected body")?;
                assert_eq!(body.get("processId").and_then(|v| v.as_u64()), Some(12345));
                assert!(message.is_some());
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("signal-control mode"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_empty_host() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Host cannot be empty"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_whitespace_host() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Host cannot be empty"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_zero_port() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Port must be in range"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_zero_timeout() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Timeout must be greater than 0"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_excessive_timeout() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("Timeout cannot exceed"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_default_values() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("localhost:13603"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_custom_port() -> Result<(), Box<dyn std::error::Error>> {
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
                let msg = message.ok_or("Expected message")?;
                assert!(msg.contains("192.168.1.100:9000"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_parse_scope_variables_from_recent_output() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        {
            let mut output =
                lock_or_recover(&adapter.recent_output, "test_parse_scope_variables.recent_output");
            output.push_back("$foo = 42".to_string());
            output.push_back("@arr = (1, 2, 3)".to_string());
            output.push_back("%hash = {a => 1}".to_string());
        }

        let (vars, child_cache) = adapter.parse_scope_variables_from_output(11, 0, 20);
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"$foo"));
        assert!(names.contains(&"@arr"));
        assert!(names.contains(&"%hash"));
        assert!(!child_cache.is_empty(), "expected child cache entries for expandable values");
        Ok(())
    }

    #[test]
    fn test_parse_scope_variables_are_sorted_for_stability()
    -> Result<(), Box<dyn std::error::Error>> {
        let lines = vec!["$zeta = 1".to_string(), "$alpha = 2".to_string(), "$mid = 3".to_string()];

        let (vars, _child_cache) =
            DebugAdapter::parse_scope_variables_from_lines(&lines, 11, 0, 20);
        let names = vars.iter().map(|v| v.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names, vec!["$alpha", "$mid", "$zeta"]);
        Ok(())
    }

    #[test]
    fn test_capture_framed_debugger_output_isolated_by_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        {
            let mut output = lock_or_recover(
                &adapter.recent_output,
                "test_capture_framed_debugger_output.recent_output",
            );
            output.push_back("noise".to_string());
            output.push_back(r#""DAP_BEGIN_100""#.to_string());
            output.push_back("$a = 1".to_string());
            output.push_back(r#""DAP_END_100""#.to_string());
            output.push_back(r#""DAP_BEGIN_200""#.to_string());
            output.push_back("$b = 2".to_string());
            output.push_back(r#""DAP_END_200""#.to_string());
        }

        let lines = adapter
            .capture_framed_debugger_output("DAP_BEGIN_200", "DAP_END_200", 200)
            .ok_or("expected framed output for marker 200")?;
        assert_eq!(lines, vec!["$b = 2".to_string()]);
        Ok(())
    }

    #[test]
    fn test_stack_trace_uses_recent_output_when_available() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut adapter = DebugAdapter::new();
        {
            let mut output = lock_or_recover(
                &adapter.recent_output,
                "test_stack_trace_recent_output.recent_output",
            );
            output.push_back("# 0 main::compute at /tmp/script.pl line 20".to_string());
            output.push_back("# 1 Foo::process called at /tmp/Foo.pm line 15".to_string());
        }

        let response = adapter.handle_request(1, "stackTrace", Some(json!({"threadId": 1})));
        match response {
            DapMessage::Response { success, body, .. } => {
                assert!(success);
                let body = body.ok_or("missing stackTrace body")?;
                let frames = body
                    .get("stackFrames")
                    .and_then(|v| v.as_array())
                    .ok_or("missing stackFrames")?;
                assert!(
                    frames.len() >= 2,
                    "expected parsed frames from recent output, got {}",
                    frames.len()
                );
            }
            _ => return Err("expected stackTrace response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_parse_evaluate_result_from_recent_output() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        {
            let mut output =
                lock_or_recover(&adapter.recent_output, "test_parse_evaluate_result.recent_output");
            output.push_back("$result = 123".to_string());
        }

        let parsed = adapter.parse_evaluate_result_from_output("$result");
        let (value, ty) = parsed.ok_or("expected parsed evaluate result")?;
        assert_eq!(value, "123");
        assert_eq!(ty, "SCALAR");
        Ok(())
    }

    // Tests for safe_eval false-positive filtering
    #[test]
    fn safe_eval_allows_identifiers_named_like_ops() {
        // These should NOT be blocked - they're identifiers, not builtins
        let allowed = [
            "$print",           // scalar variable
            "@say",             // array variable
            "%exit",            // hash variable
            "*printf",          // glob
            "${print}",         // braced scalar variable
            "${ print }",       // braced with spaces
            "'print'",          // single-quoted string
            "Foo::print",       // package-qualified
            "My::Module::exit", // deeply qualified
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }
    }

    #[test]
    fn safe_eval_still_blocks_real_ops() {
        // These MUST be blocked - they're actual dangerous operations
        let blocked = [
            "print",
            "print $x",
            "say 'hello'",
            "exit",
            "exit 0",
            "eval '$x'",
            "eval { }",
            "system 'ls'",
            "exec '/bin/sh'",
            "fork",
            "kill 9, $$",
            "CORE::print $x",
            "CORE::GLOBAL::exit",
            "$obj->print",
            "$obj->system('ls')",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_mutating_regex_ops() {
        let blocked = [
            "$x =~ s/a/b/",
            "s/a/b/",
            "$x =~ tr/a/b/",
            "tr/a/b/",
            "y/a/b/",
            "$x =~ y/a/b/", // Bound y/// form
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_allows_regex_literals_with_escape_sequences() {
        // These should NOT be blocked - they're regex patterns or identifiers, not mutations
        // Note: Patterns using =~ are blocked by the assignment check (pre-existing behavior)
        // so we test patterns without =~ here
        let allowed = [
            r#"/\s+/"#,    // \s in regex literal (no binding operator)
            r#"/string/"#, // match containing 's'
            r#"/tricky/"#, // match containing 'tr'
            r#"/yay/"#,    // match containing 'y'
            r#"$s"#,       // variable named $s
            r#"$tr"#,      // variable named $tr
            r#"$y"#,       // variable named $y
            r#"qr/\s+/"#,  // compiled regex with \s
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_new_dangerous_ops() {
        // Verify the extended deny-list works
        let blocked = [
            "eval '$code'",
            "kill 9, $pid",
            "exit 1",
            "dump",
            "fork",
            "chroot '/tmp'",
            "print STDERR 'x'",
            "say 'hello'",
            "printf '%s', $x",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_extended_ops_v2() {
        // Verify the even more extended deny-list works (glob, readline, IPC, etc.)
        let blocked = [
            "glob '*'",
            "readline $fh",
            "ioctl $fh, 1, 1",
            "srand",
            "dbmopen %h, 'file', 0666",
            "shmget $key, 10, 0666",
            "select $r, $w, $e, 0",
            "shutdown $socket, 2",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_mutation_and_resource_ops() {
        // Verify newly added mutation and resource management operations are blocked
        let blocked = [
            "bless $ref, 'Class'",
            "reset 'a-z'",
            "umask 0022",
            "binmode $fh",
            "opendir $dh, '.'",
            "closedir $dh",
            "seek $fh, 0, 0",
            "sysseek $fh, 0, 0",
            "setpgrp",
            "setpriority 0, 0, 10",
            "formline",
            "write",
            "lock $ref",
            "pipe $r, $w",
            "socketpair $r, $w, 1, 1, 1",
            "setsockopt $s, 1, 1, 1",
            "utime 1, 1, 'file'",
            "readdir $dh",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_blocks_dereference_execution() {
        // These are variables (safe to access)
        let allowed = ["$system", "@exec", "%fork"];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }

        // These are dereference calls (NOT safe)
        // &$system calls the sub ref in $system
        // ->$system calls the method named in $system
        let blocked = [
            "&$system",
            "& $system",
            "&{$system}", // Braced form
            "$obj->$system",
            "$obj-> $system",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    /// Helper to create a test stack frame
    fn make_test_frame(id: i32, name: &str, path: &str, line: i32) -> StackFrame {
        StackFrame {
            id,
            name: name.to_string(),
            source: Source {
                name: Some(path.split('/').next_back().unwrap_or(path).to_string()),
                path: path.to_string(),
                source_reference: None,
            },
            line,
            column: 1,
            end_line: None,
            end_column: None,
        }
    }

    /// Test helper: Filter frames using the same logic as handle_stack_trace (AC8.2.1)
    fn filter_internal_frames(frames: Vec<StackFrame>) -> Vec<StackFrame> {
        frames
            .into_iter()
            .filter(|f| {
                !f.name.starts_with("Devel::TSPerlDAP::")
                    && !f.name.starts_with("DB::")
                    && !f.source.path.contains("perl5db.pl")
            })
            .collect()
    }

    #[test]
    fn test_stack_frame_filtering_removes_db_frames() {
        // AC8.2.1: Filter internal frames from user-visible stack
        let frames = vec![
            make_test_frame(1, "main::hello", "/app/hello.pl", 10),
            make_test_frame(2, "DB::DB", "/usr/share/perl/5.34/perl5db.pl", 100),
            make_test_frame(3, "Foo::bar", "/app/lib/Foo.pm", 25),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::hello");
        assert_eq!(filtered[1].name, "Foo::bar");
    }

    #[test]
    fn test_stack_frame_filtering_removes_shim_frames() {
        // AC8.2.1: Filter Devel::TSPerlDAP:: shim infrastructure frames
        let frames = vec![
            make_test_frame(1, "Devel::TSPerlDAP::init", "/shim/TSPerlDAP.pm", 50),
            make_test_frame(2, "main::run", "/app/script.pl", 5),
            make_test_frame(3, "Devel::TSPerlDAP::handle_break", "/shim/TSPerlDAP.pm", 200),
            make_test_frame(4, "Utils::process", "/app/lib/Utils.pm", 42),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::run");
        assert_eq!(filtered[1].name, "Utils::process");
    }

    #[test]
    fn test_stack_frame_filtering_removes_perl5db_source() {
        // AC8.2.1: Filter frames from perl5db.pl even with different names
        let frames = vec![
            make_test_frame(1, "main::start", "/app/main.pl", 1),
            make_test_frame(2, "some_internal", "/usr/lib/perl5/perl5db.pl", 999),
            make_test_frame(3, "App::process", "/app/lib/App.pm", 100),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::start");
        assert_eq!(filtered[1].name, "App::process");
    }

    #[test]
    fn test_stack_frame_filtering_mixed_internal_frames() {
        // AC8.2.1: Comprehensive test with all types of internal frames mixed
        let frames = vec![
            // User frame 1
            make_test_frame(1, "main::hello", "/app/hello.pl", 10),
            // DB:: frame (internal)
            make_test_frame(2, "DB::sub", "/usr/share/perl/5.34/perl5db.pl", 2000),
            // User frame 2
            make_test_frame(3, "Foo::bar", "/app/lib/Foo.pm", 25),
            // Shim frame (internal)
            make_test_frame(4, "Devel::TSPerlDAP::step", "/shim/TSPerlDAP.pm", 150),
            // DB:: frame without perl5db.pl path (still filtered)
            make_test_frame(5, "DB::breakpoint", "/some/other/path.pm", 50),
            // User frame 3
            make_test_frame(6, "Baz::qux", "/app/lib/Baz.pm", 75),
            // perl5db.pl source frame (internal)
            make_test_frame(7, "custom_handler", "/usr/lib/perl5/perl5db.pl", 1500),
        ];

        let filtered = filter_internal_frames(frames);

        // Should only have user frames: main::hello, Foo::bar, Baz::qux
        assert_eq!(filtered.len(), 3, "Expected 3 user frames, got {}", filtered.len());
        assert_eq!(filtered[0].name, "main::hello");
        assert_eq!(filtered[1].name, "Foo::bar");
        assert_eq!(filtered[2].name, "Baz::qux");
    }

    #[test]
    fn test_stack_frame_filtering_preserves_order() {
        // Verify frame order is preserved after filtering
        let frames = vec![
            make_test_frame(1, "A::first", "/a.pm", 1),
            make_test_frame(2, "DB::internal", "/perl5db.pl", 100),
            make_test_frame(3, "B::second", "/b.pm", 2),
            make_test_frame(4, "Devel::TSPerlDAP::shim", "/shim.pm", 50),
            make_test_frame(5, "C::third", "/c.pm", 3),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "A::first");
        assert_eq!(filtered[1].name, "B::second");
        assert_eq!(filtered[2].name, "C::third");
    }

    #[test]
    fn test_stack_frame_filtering_all_internal() {
        // Edge case: all frames are internal
        let frames = vec![
            make_test_frame(1, "DB::main", "/perl5db.pl", 1),
            make_test_frame(2, "Devel::TSPerlDAP::init", "/shim.pm", 10),
            make_test_frame(3, "DB::sub", "/perl5db.pl", 50),
        ];

        let filtered = filter_internal_frames(frames);

        assert!(filtered.is_empty(), "Expected empty stack after filtering all internal frames");
    }

    #[test]
    fn test_stack_frame_filtering_no_internal() {
        // Edge case: no internal frames to filter
        let frames = vec![
            make_test_frame(1, "main::start", "/app/main.pl", 1),
            make_test_frame(2, "Lib::helper", "/app/lib/Lib.pm", 50),
            make_test_frame(3, "Utils::format", "/app/lib/Utils.pm", 100),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "main::start");
        assert_eq!(filtered[1].name, "Lib::helper");
        assert_eq!(filtered[2].name, "Utils::format");
    }

    #[test]
    fn test_stack_frame_filtering_empty_input() {
        // Edge case: empty frame list
        let frames: Vec<StackFrame> = vec![];
        let filtered = filter_internal_frames(frames);
        assert!(filtered.is_empty());
    }

    // AC8.2.4: Stack trace parsing tests for simple call chains (A → B → C)
    #[test]
    fn test_parse_stack_trace_simple_call_chain() {
        let output = r#"# 0 main::compute_sum at /app/script.pl line 20
# 1 Foo::process called at /app/lib/Foo.pm line 15
# 2 main::start at /app/script.pl line 5"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 3);

        // Frame 0: main::compute_sum
        assert_eq!(frames[0].id, 1);
        assert_eq!(frames[0].name, "main::compute_sum");
        assert_eq!(frames[0].source.path, "/app/script.pl");
        assert_eq!(frames[0].line, 20);
        assert_eq!(frames[0].source.name, Some("script.pl".to_string()));

        // Frame 1: Foo::process
        assert_eq!(frames[1].id, 2);
        assert_eq!(frames[1].name, "Foo::process");
        assert_eq!(frames[1].source.path, "/app/lib/Foo.pm");
        assert_eq!(frames[1].line, 15);

        // Frame 2: main::start
        assert_eq!(frames[2].id, 3);
        assert_eq!(frames[2].name, "main::start");
        assert_eq!(frames[2].source.path, "/app/script.pl");
        assert_eq!(frames[2].line, 5);
    }

    // AC8.2.4: Stack trace parsing for multi-file call stacks across packages
    #[test]
    fn test_parse_stack_trace_multi_file_packages() {
        let output = r#"# 0 Utils::Helper::validate at /app/lib/Utils/Helper.pm line 42
# 1 Data::Processor::transform called at /app/lib/Data/Processor.pm line 120
# 2 Controller::API::handle_request at /app/controller/API.pm line 78
# 3 main::dispatch called at /app/app.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0].name, "Utils::Helper::validate");
        assert_eq!(frames[1].name, "Data::Processor::transform");
        assert_eq!(frames[2].name, "Controller::API::handle_request");
        assert_eq!(frames[3].name, "main::dispatch");

        // Verify cross-file navigation info is present
        assert!(frames[0].source.path.contains("Utils/Helper.pm"));
        assert!(frames[1].source.path.contains("Data/Processor.pm"));
        assert!(frames[2].source.path.contains("controller/API.pm"));
        assert!(frames[3].source.path.contains("app.pl"));
    }

    // AC8.2.4: Stack trace parsing for recursive calls with depth
    #[test]
    fn test_parse_stack_trace_recursive_calls() {
        let output = r#"# 0 main::factorial at /app/math.pl line 5
# 1 main::factorial called at /app/math.pl line 6
# 2 main::factorial called at /app/math.pl line 6
# 3 main::factorial called at /app/math.pl line 6
# 4 main::compute at /app/math.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 5);

        // Verify recursive frames are all parsed correctly
        assert_eq!(frames[0].name, "main::factorial");
        assert_eq!(frames[1].name, "main::factorial");
        assert_eq!(frames[2].name, "main::factorial");
        assert_eq!(frames[3].name, "main::factorial");
        assert_eq!(frames[4].name, "main::compute");

        // Verify frame IDs are sequential
        assert_eq!(frames[0].id, 1);
        assert_eq!(frames[1].id, 2);
        assert_eq!(frames[2].id, 3);
        assert_eq!(frames[3].id, 4);
        assert_eq!(frames[4].id, 5);
    }

    // AC8.2.4: Stack trace parsing for anonymous subroutines
    #[test]
    fn test_parse_stack_trace_anonymous_subs() {
        let output = r#"# 0 main::__ANON__ at /app/callback.pl line 15
# 1 Utils::map called at /app/lib/Utils.pm line 42
# 2 main::process_items at /app/callback.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 3);

        // Verify anonymous sub is parsed (Perl uses __ANON__ for anonymous subs)
        assert_eq!(frames[0].name, "main::__ANON__");
        assert_eq!(frames[1].name, "Utils::map");
        assert_eq!(frames[2].name, "main::process_items");
    }

    // AC8.2: Stack trace parsing with Windows paths
    #[test]
    fn test_parse_stack_trace_windows_paths() {
        let output = r#"# 0 main::test at C:\workspace\script.pl line 10
# 1 Foo::bar called at C:\workspace\lib\Foo.pm line 25"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].source.path, r"C:\workspace\script.pl");
        assert_eq!(frames[0].source.name, Some("script.pl".to_string()));
        assert_eq!(frames[1].source.path, r"C:\workspace\lib\Foo.pm");
        assert_eq!(frames[1].source.name, Some("Foo.pm".to_string()));
    }

    // AC8.2: Stack trace parsing with empty output
    #[test]
    fn test_parse_stack_trace_empty_output() {
        let output = "";
        let frames = DebugAdapter::parse_stack_trace(output);
        assert!(frames.is_empty());
    }

    // AC8.2: Stack trace parsing with malformed output
    #[test]
    fn test_parse_stack_trace_malformed_output() {
        let output = r#"Random output that doesn't match
Some error message
DB<1>"#;

        let frames = DebugAdapter::parse_stack_trace(output);
        assert!(frames.is_empty());
    }

    // AC8.2.1: Integration test - parse and filter combined
    #[test]
    fn test_parse_and_filter_stack_trace() {
        let output = r#"# 0 main::user_func at /app/script.pl line 10
# 1 DB::DB called at /usr/share/perl/5.34/perl5db.pl line 100
# 2 Foo::process at /app/lib/Foo.pm line 25
# 3 Devel::TSPerlDAP::handle_break called at /shim/TSPerlDAP.pm line 50
# 4 main::start at /app/script.pl line 5"#;

        let frames = DebugAdapter::parse_stack_trace(output);
        assert_eq!(frames.len(), 5);

        // Apply filtering
        let filtered = filter_internal_frames(frames);

        // Should only have user frames after filtering
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "main::user_func");
        assert_eq!(filtered[1].name, "Foo::process");
        assert_eq!(filtered[2].name, "main::start");
    }

    #[test]
    fn test_safe_eval_bypass_prevention() {
        // These patterns attempt to bypass safe evaluation checks
        let bypasses = [
            "&{'sys'.'tem'}('ls')", // Dynamic function name via concatenation
            "& { 'sys' . 'tem' }",  // Dynamic function name with spaces
            "<*.txt>",              // Glob operator for filesystem access
            "CORE::print",          // Explicitly blocked by dangerous ops regex
        ];

        for expr in bypasses {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "Expression '{}' should be blocked but was allowed", expr);
        }
    }

    #[test]
    fn test_safe_eval_assignment_ops_precision() {
        // These are comparison/binding operators (SAFE) but were previously blocked
        // because they contain '='
        let allowed = [
            "$a == $b",
            "$a != $b",
            "$a <= $b",
            "$a >= $b",
            "$a <=> $b",
            "$a =~ /regex/",
            "$a !~ /regex/",
            "$a ~~ $b", // Smart match
            // Logical ops
            "$a && $b",
            "$a || $b",
            "$a // $b",
            // Bitwise ops
            "$a & $b",
            "$a | $b",
            "$a ^ $b",
            "$a << $b",
            "$a >> $b",
            // Range
            "1..10",
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }

        // These are strict assignment operators (UNSAFE) and MUST be blocked
        let blocked = [
            "$a = 1",
            "$a += 1",
            "$a -= 1",
            "$a *= 1",
            "$a /= 1",
            "$a %= 1",
            "$a **= 1",
            "$a .= 's'",
            "$a &= 1",
            "$a |= 1",
            "$a ^= 1",
            "$a <<= 1",
            "$a >>= 1",
            "$a &&= 1",
            "$a ||= 1",
            "$a //= 1",
            "$a x= 3", // Repetition assignment
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_tcp_session_threads_non_empty() {
        let adapter = DebugAdapter::new();
        // Inject a TcpAttachSession so handle_threads sees it
        {
            let mut guard = lock_or_recover(&adapter.tcp_session, "test.tcp_session");
            *guard = Some(TcpAttachSession::new());
        }
        let response = adapter.handle_threads(1, 1);
        match response {
            DapMessage::Response { success, body: Some(body), .. } => {
                assert!(success);
                let threads = body["threads"].as_array().expect("threads must be array");
                assert!(!threads.is_empty(), "TCP attach should return non-empty threads");
                assert_eq!(threads[0]["id"], 1);
                assert_eq!(threads[0]["name"], "TCP Attached Thread");
            }
            _ => panic!("Expected successful response with body"),
        }
    }

    #[test]
    fn test_attach_port_out_of_range() {
        let mut adapter = DebugAdapter::new();
        // Initialize first so attach is allowed
        let _ = adapter.handle_request(1, "initialize", None);

        for port in [65536_u64, 70000, u64::MAX] {
            let args = json!({ "port": port });
            let response = adapter.handle_request(2, "attach", Some(args));
            match response {
                DapMessage::Response { success, message, .. } => {
                    assert!(!success, "port {port} should be rejected");
                    assert!(
                        message.as_ref().is_some_and(|m| m.contains("out of range")),
                        "expected 'out of range' error for port {port}, got: {message:?}"
                    );
                }
                _ => panic!("Expected error response for port {port}"),
            }
        }
    }

    #[test]
    fn test_attach_port_valid_boundary() {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", None);

        // Port 1 and 65535 should pass port validation (may fail later at TCP connect)
        for port in [1_u64, 65535] {
            let args = json!({ "port": port });
            let response = adapter.handle_request(2, "attach", Some(args));
            match response {
                DapMessage::Response { message, .. } => {
                    // Should NOT contain "out of range" — it passed validation
                    assert!(
                        !message.as_ref().is_some_and(|m| m.contains("out of range")),
                        "port {port} should pass range validation, got: {message:?}"
                    );
                }
                _ => {}
            }
        }
    }
}
