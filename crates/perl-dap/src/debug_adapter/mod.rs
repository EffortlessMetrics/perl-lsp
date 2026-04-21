//! Debug Adapter Protocol (DAP) implementation for Perl debugging
//!
//! This module provides a DAP server that integrates with Perl's built-in debugger
//! to enable debugging support in VSCode and other DAP-compatible editors.

mod breakpoints;
mod evaluation;
mod execution;
mod frames;
mod output;
mod process;
mod variables;

mod dispatch;
mod parsing;
pub(crate) mod safe_eval;
mod transport;

use crate::breakpoint::{AstBreakpointValidator, BreakpointValidator};
use crate::eval::SafeEvaluator;
use crate::feature_catalog::has_feature as catalog_has_feature;
use crate::inline_values::{collect_inline_values_with_runtime, extract_variable_names};
use crate::protocol::{
    BreakpointLocation, BreakpointLocationsArguments, BreakpointLocationsResponseBody,
    CompletionItem, CompletionsArguments, CompletionsResponseBody, ContinueArguments,
    ContinueResponseBody, DataBreakpointInfoArguments, DataBreakpointInfoResponseBody,
    DisconnectArguments, EvaluateArguments, EvaluateResponseBody, ExceptionDetails,
    ExceptionInfoArguments, ExceptionInfoResponseBody, GotoArguments, GotoTarget,
    GotoTargetsArguments, GotoTargetsResponseBody, InlineValuesArguments, InlineValuesResponseBody,
    LoadedSourcesResponseBody, Module, ModulesArguments, ModulesResponseBody, NextArguments,
    PauseArguments, RestartArguments, Scope, ScopesArguments, ScopesResponseBody,
    SetDataBreakpointsArguments, SetDataBreakpointsResponseBody, SetExceptionBreakpointsArguments,
    SetExpressionArguments, SetExpressionResponseBody, SetFunctionBreakpointsArguments,
    SetVariableArguments, SetVariableResponseBody, SourceArguments, SourceResponseBody,
    StackTraceArguments, StepInArguments, StepInTarget, StepInTargetsArguments,
    StepInTargetsResponseBody, StepOutArguments, TerminateArguments, VariablesArguments,
};
use crate::stack::{PerlStackParser, is_internal_frame_name_and_path};
use crate::tcp_attach::{DapEvent, TcpAttachConfig, TcpAttachSession};
use crate::types::{Source, StackFrame, Variable};
use crate::variables::{PerlVariableRenderer, RenderedVariable, VariableParser, VariableRenderer};
use perl_lexer::DAP_COMPLETION_KEYWORDS;
use perl_lsp_rs_core::transport::framing::{ContentLengthFramer, frame};
use perl_module::path::module_path_to_name;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::breakpoints::{BreakpointHitOutcome, BreakpointStore};
use crate::security;
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use regex::Regex;
use safe_eval::validate_safe_expression;

/// Poison-safe mutex lock that recovers from poisoned state
fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, ctx: &'static str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(ctx, "Poisoned mutex recovered");
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
static WARNING_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static INC_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
const RECENT_OUTPUT_MAX_LINES: usize = 2048;
const DEBUG_SESSION_TERMINATE_WAIT_MS: u64 = 250;
const DEBUGGER_QUERY_WAIT_MS: u64 = 75;
const DEBUGGER_FRAME_POLL_MS: u64 = 10;

fn context_re() -> Option<&'static Regex> {
    CONTEXT_RE
        .get_or_init(|| {
            // Match Perl debugger context lines of the form:
            //   Func::Name(/path/to/file.pl:42):
            //   main::(/path/to/file.pl:42):
            //   main::(C:\Windows\path\file.pl:42):
            //
            // File paths may contain `:` on Windows (drive letter prefix such as `C:\`).
            // The path-capturing groups allow a `:` only when it is immediately followed
            // by a non-digit, non-space character — matching `C:\path` but not the `:42`
            // line-number separator.
            Regex::new(r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)\s]+(?::[^:)\d\s][^:)\s]*)*):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+(?::[^:)\d\s][^:)\s]*)*)(?:\))?:(?P<line2>\d+):?)")
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

fn warning_re() -> Option<&'static Regex> {
    WARNING_RE
        .get_or_init(|| {
            // Perl `warn`, `Carp::carp`, and `Carp::cluck` emit warning messages.
            // Common patterns:
            //  - "Something went wrong at script.pl line 42."
            //  - "Use of uninitialized value..."
            //  - Explicit warn/carp/cluck output
            Regex::new(
                r"(?i)\b(?:warn(?:ing)?|carp|cluck)\b.*\bat\s+\S+?\s+line\s+\d+|^.+\bat\s+\S+?\s+line\s+\d+\.?\s*$",
            )
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
                "each",
                "keys",
                "values",
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
                "eof",
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

fn inc_re() -> Option<&'static Regex> {
    INC_RE.get_or_init(|| Regex::new(r"'([^']+)'\s*=>\s*'([^']+)'")).as_ref().ok()
}

/// Stored data breakpoint record for watchpoint management
#[derive(Debug, Clone)]
struct DataBreakpointRecord {
    #[allow(dead_code)]
    data_id: String,
    #[allow(dead_code)]
    access_type: Option<String>,
    #[allow(dead_code)]
    condition: Option<String>,
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
    /// Exception breakpoint policy: break on `warn`/carp/cluck output.
    exception_break_on_warn: Arc<Mutex<bool>>,
    /// Unique marker IDs used to frame debugger output per command.
    debugger_output_marker: Arc<AtomicU64>,
    /// Cancellation flag for in-progress requests.
    cancel_requested: Arc<AtomicBool>,
    /// Data breakpoints (watchpoints) stored with REPLACE semantics
    data_breakpoints: Arc<Mutex<Vec<DataBreakpointRecord>>>,
    /// Last exception message captured by the output reader (for exceptionInfo)
    last_exception_message: Arc<Mutex<Option<String>>>,
    /// Stored launch arguments for restart support
    last_launch_args: Arc<Mutex<Option<Value>>>,
    /// Goto target ID → (file_path, line) mapping for cross-file goto
    goto_targets: Arc<Mutex<HashMap<i64, (String, i64)>>>,
    /// Monotonic goto target ID counter
    next_goto_target_id: Arc<Mutex<i64>>,
    /// Workspace root for path validation (set during launch)
    workspace_root: Arc<Mutex<Option<PathBuf>>>,
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
    /// Like `Continue` but auto-continues past any non-breakpoint stop.
    /// Used when `configurationDone` runs with `stopOnEntry: false` to
    /// silently skip the debugger's implicit first-line stop and run to
    /// the first user-set breakpoint.
    RunToBreakpoint,
    Goto,
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
            exception_break_on_warn: Arc::new(Mutex::new(false)),
            debugger_output_marker: Arc::new(AtomicU64::new(1)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            data_breakpoints: Arc::new(Mutex::new(Vec::new())),
            last_exception_message: Arc::new(Mutex::new(None)),
            last_launch_args: Arc::new(Mutex::new(None)),
            goto_targets: Arc::new(Mutex::new(HashMap::new())),
            next_goto_target_id: Arc::new(Mutex::new(1)),
            workspace_root: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the event sender (primarily for testing)
    pub fn set_event_sender(&mut self, sender: Sender<DapMessage>) {
        self.event_sender = Some(sender);
    }

    /// Validate a client-provided source path against the workspace root.
    ///
    /// Returns the validated `PathBuf` on success, or an error message on failure.
    /// If no workspace root is set (pre-launch), the path is allowed through with a
    /// warning — defense-in-depth only blocks when a workspace boundary is known.
    fn validate_source_path(&self, path: &str) -> Result<PathBuf, String> {
        let ws = lock_or_recover(&self.workspace_root, "debug_adapter.workspace_root");
        match ws.as_ref() {
            Some(root) => security::validate_path(Path::new(path), root)
                .map_err(|e| format!("Path validation failed: {e}")),
            None => {
                // No workspace set (pre-launch) — allow reads but accept the risk
                Ok(PathBuf::from(path))
            }
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

    /// Snapshot debugger output history for parsing without holding locks.
    fn snapshot_recent_output_lines(&self) -> Vec<String> {
        let output = lock_or_recover(&self.recent_output, "debug_adapter.recent_output");
        output.iter().cloned().collect()
    }

    /// Allocate a unique marker id used for framed debugger output capture.
    fn next_debugger_marker_id(&self) -> u64 {
        self.debugger_output_marker.fetch_add(1, Ordering::Relaxed)
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
            Instant::now() + Duration::from_millis(Self::debugger_timeout_budget_ms(timeout_ms));

        loop {
            // Check for cancellation before each poll iteration
            if self.cancel_requested.load(Ordering::Acquire) {
                self.cancel_requested.store(false, Ordering::Release);
                return None;
            }

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

    /// Expand debugger query budgets in heavily instrumented environments.
    ///
    /// `cargo llvm-cov` adds noticeable overhead to framed debugger queries against a
    /// real `perl -d` subprocess. Keep the normal fast path unchanged, but allow a
    /// larger budget when coverage profiling is active.
    fn debugger_timeout_budget_ms(timeout_ms: u64) -> u64 {
        let base = timeout_ms.max(DEBUGGER_QUERY_WAIT_MS);
        if std::env::var_os("LLVM_PROFILE_FILE").is_some()
            || std::env::var_os("CARGO_LLVM_COV").is_some()
        {
            base.clamp(15_000, 30_000)
        } else {
            base
        }
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
            (
                "supportsExceptionOptions",
                crate::feature_catalog::has_feature("dap.exceptions.die")
                    || crate::feature_catalog::has_feature("dap.exceptions.warn"),
            ),
            (
                "supportsExceptionFilterOptions",
                crate::feature_catalog::has_feature("dap.exceptions.die")
                    || crate::feature_catalog::has_feature("dap.exceptions.warn"),
            ),
            ("supportsInlineValues", crate::feature_catalog::has_feature("dap.inline_values")),
            ("supportsTerminateRequest", crate::feature_catalog::has_feature("dap.core")),
            ("supportsCompletionsRequest", crate::feature_catalog::has_feature("dap.completions")),
            ("supportsModulesRequest", crate::feature_catalog::has_feature("dap.modules")),
            ("supportsDataBreakpoints", crate::feature_catalog::has_feature("dap.watchpoints")),
            ("supportsTerminateThreadsRequest", false),
            ("supportsGotoTargetsRequest", crate::feature_catalog::has_feature("dap.core")),
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

        let has_filter = |id: &str| -> bool {
            exception_filters.iter().any(|f| f.get("filter").and_then(Value::as_str) == Some(id))
        };

        let die_enabled = crate::feature_catalog::has_feature("dap.exceptions.die");
        let warn_enabled = crate::feature_catalog::has_feature("dap.exceptions.warn");

        assert_eq!(
            has_filter("die"),
            die_enabled,
            "die filter presence must match dap.exceptions.die"
        );
        assert_eq!(
            has_filter("all"),
            die_enabled,
            "all filter presence must match dap.exceptions.die"
        );
        assert_eq!(
            has_filter("warn"),
            warn_enabled,
            "warn filter presence must match dap.exceptions.warn"
        );

        if !die_enabled && !warn_enabled {
            assert!(
                exception_filters.is_empty(),
                "exceptionBreakpointFilters must be empty when no exception features are enabled"
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
            ("supportsCompletionsRequest", "completions"),
            ("supportsModulesRequest", "modules"),
            ("supportsRestartRequest", "restart"),
            ("supportsExceptionInfoRequest", "exceptionInfo"),
            ("supportsBreakpointLocationsRequest", "breakpointLocations"),
            ("supportsSetExpression", "setExpression"),
            ("supportsDataBreakpoints", "setDataBreakpoints"),
            ("supportsLoadedSourcesRequest", "loadedSources"),
            ("supportsCancelRequest", "cancel"),
            ("supportsStepInTargetsRequest", "stepInTargets"),
            ("supportsGotoTargetsRequest", "gotoTargets"),
            ("supportsTerminateThreadsRequest", "terminateThreads"),
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
                "completions" => Some(json!({"text": "pr", "column": 2})),
                "modules" => Some(json!({})),
                "restart" => Some(json!({})),
                "exceptionInfo" => Some(json!({"threadId": 1})),
                "breakpointLocations" => Some(json!({
                    "source": { "path": "/tmp/capability_honesty.pl" },
                    "line": 1
                })),
                "setExpression" => Some(json!({"expression": "$x", "value": "1"})),
                "setDataBreakpoints" => Some(json!({"breakpoints": []})),
                "loadedSources" => Some(json!({})),
                "cancel" => Some(json!({})),
                "stepInTargets" => Some(json!({"frameId": 1})),
                "gotoTargets" => Some(json!({
                    "source": { "path": "/tmp/capability_honesty.pl" },
                    "line": 1
                })),
                "terminateThreads" => Some(json!({})),
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

        // supportsTerminateThreadsRequest must be false (Perl limitation)
        assert_eq!(
            capability_map.get("supportsTerminateThreadsRequest").and_then(|v| v.as_bool()),
            Some(false),
            "supportsTerminateThreadsRequest must be false — Perl has no thread termination"
        );

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
    fn test_tcp_session_threads_non_empty() -> Result<(), Box<dyn std::error::Error>> {
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
                let threads = body["threads"].as_array().ok_or("threads must be array")?;
                assert!(!threads.is_empty(), "TCP attach should return non-empty threads");
                assert_eq!(threads[0]["id"], 1);
                assert_eq!(threads[0]["name"], "TCP Attached Thread");
            }
            _ => return Err("Expected successful response with body".into()),
        }
        Ok(())
    }

    #[test]
    fn test_attach_port_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
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
                _ => return Err(format!("Expected error response for port {port}").into()),
            }
        }
        Ok(())
    }

    #[test]
    fn test_attach_port_valid_boundary() {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", None);

        // Port 1 and 65535 should pass port validation (may fail later at TCP connect)
        for port in [1_u64, 65535] {
            let args = json!({ "port": port });
            let response = adapter.handle_request(2, "attach", Some(args));
            if let DapMessage::Response { message, .. } = response {
                // Should NOT contain "out of range" — it passed validation
                assert!(
                    !message.as_ref().is_some_and(|m| m.contains("out of range")),
                    "port {port} should pass range validation, got: {message:?}"
                );
            }
        }
    }

    #[test]
    fn test_goto_missing_arguments() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(1, "goto", None);
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "goto");
                assert_eq!(message.as_deref(), Some("Missing or invalid arguments"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_goto_invalid_target() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let response =
            adapter.handle_request(1, "goto", Some(json!({"threadId": 1, "targetId": -1})));
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "goto");
                // With target mapping, unknown IDs produce "Unknown goto target id"
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("Unknown goto target"),
                    "expected unknown target message, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_goto_no_session() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        // First store a mapping so goto gets past the lookup
        {
            let mut goto_map = lock_or_recover(&adapter.goto_targets, "test.goto_targets");
            goto_map.insert(10, ("/test/file.pl".to_string(), 10));
        }
        let response =
            adapter.handle_request(1, "goto", Some(json!({"threadId": 1, "targetId": 10})));
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success);
                assert_eq!(command, "goto");
                assert_eq!(message.as_deref(), Some("No active debug session"));
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_terminate_threads_capability_is_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        let init = adapter.handle_request(1, "initialize", None);
        let capabilities = match init {
            DapMessage::Response { success: true, body: Some(body), .. } => body,
            _ => return Err("Expected successful initialize response".into()),
        };
        let cap_map = capabilities.as_object().ok_or("body must be object")?;
        assert_eq!(
            cap_map.get("supportsTerminateThreadsRequest").and_then(|v| v.as_bool()),
            Some(false),
            "supportsTerminateThreadsRequest must be false"
        );
        Ok(())
    }

    #[test]
    fn test_goto_targets_then_goto_flow() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();

        // gotoTargets should succeed (even with no file — returns empty targets)
        let gt_response = adapter.handle_request(
            1,
            "gotoTargets",
            Some(json!({"source": {"path": "/tmp/nonexistent.pl"}, "line": 1})),
        );
        match gt_response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(success, "gotoTargets should succeed");
                assert_eq!(command, "gotoTargets");
                // Must NOT say "does not support"
                assert!(
                    !message.as_deref().unwrap_or("").contains("does not support"),
                    "gotoTargets must not claim lack of support"
                );
            }
            _ => return Err("Expected response".into()),
        }

        // goto should fail gracefully with unknown target (no stored mapping)
        let goto_response =
            adapter.handle_request(2, "goto", Some(json!({"threadId": 1, "targetId": 999})));
        match goto_response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "goto with unknown target should fail");
                assert_eq!(command, "goto");
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("Unknown goto target"),
                    "goto must report unknown target, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_goto_targets_stores_mapping() -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let mut adapter = DebugAdapter::new();
        adapter.handle_request(1, "initialize", None);

        // Create a temp file with executable content
        let dir = tempfile::tempdir()?;
        let file_path = dir.path().join("test_goto.pl");
        {
            let mut f = std::fs::File::create(&file_path)?;
            writeln!(f, "my $x = 1;")?;
            writeln!(f, "my $y = 2;")?;
            writeln!(f, "print $x + $y;")?;
        }

        let path_str = file_path.to_string_lossy().to_string();
        let response = adapter.handle_request(
            2,
            "gotoTargets",
            Some(json!({
                "source": {"path": path_str},
                "line": 2
            })),
        );

        // Verify the response contains targets with monotonic IDs (not line numbers)
        match response {
            DapMessage::Response { success, body: Some(body), .. } => {
                assert!(success, "gotoTargets should succeed");
                let targets = body
                    .get("targets")
                    .and_then(|t| t.as_array())
                    .ok_or("should have targets array")?;
                assert!(!targets.is_empty(), "should find executable lines");

                // Verify IDs are monotonic starting from 1, NOT equal to line numbers
                let first_id = targets[0].get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                assert!(first_id >= 1, "IDs should start at 1 or higher");

                // Verify the mapping was stored internally
                let goto_map = lock_or_recover(&adapter.goto_targets, "test.goto_targets");
                assert!(!goto_map.is_empty(), "goto_targets map should be populated");
                // Each stored entry should reference our temp file
                for (_id, (stored_path, _line)) in goto_map.iter() {
                    assert_eq!(stored_path, &path_str, "stored path should match source");
                }
            }
            _ => return Err("Expected successful response".into()),
        }

        let _ = std::fs::remove_file(&file_path);
        Ok(())
    }

    #[test]
    fn test_goto_uses_stored_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        adapter.handle_request(1, "initialize", None);

        // Manually populate the goto_targets map to simulate handle_goto_targets
        {
            let mut goto_map = lock_or_recover(&adapter.goto_targets, "test.goto_targets");
            goto_map.insert(42, ("/some/file.pl".to_string(), 10));
        }

        // Without a debug session, goto should fail with "No active debug session"
        // but only after successfully looking up the target
        let response =
            adapter.handle_request(2, "goto", Some(json!({"threadId": 1, "targetId": 42})));
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "goto without session should fail");
                assert_eq!(command, "goto");
                // It should NOT say "Unknown goto target" — the mapping was found
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("No active debug session"),
                    "goto should report no session, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }

        // Verify the consumed entry was removed from the map
        let goto_map = lock_or_recover(&adapter.goto_targets, "test.goto_targets");
        assert!(!goto_map.contains_key(&42), "consumed goto target should be removed from map");
        Ok(())
    }

    #[test]
    fn test_source_rejects_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        adapter.handle_request(1, "initialize", None);

        // Set workspace root to a temp directory
        let dir = tempfile::tempdir()?;
        *lock_or_recover(&adapter.workspace_root, "test.workspace_root") =
            Some(dir.path().to_path_buf());

        let response = adapter.handle_request(
            2,
            "source",
            Some(json!({
                "source": {"path": "../../../etc/passwd"},
                "sourceReference": 0
            })),
        );
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "source with traversal path should fail");
                assert_eq!(command, "source");
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("Path validation failed"),
                    "should report path validation failure, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_breakpoint_locations_rejects_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        adapter.handle_request(1, "initialize", None);

        let dir = tempfile::tempdir()?;
        *lock_or_recover(&adapter.workspace_root, "test.workspace_root") =
            Some(dir.path().to_path_buf());

        let response = adapter.handle_request(
            2,
            "breakpointLocations",
            Some(json!({
                "source": {"path": "../../../etc/passwd"},
                "line": 1
            })),
        );
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "breakpointLocations with traversal path should fail");
                assert_eq!(command, "breakpointLocations");
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("Path validation failed"),
                    "should report path validation failure, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    #[test]
    fn test_goto_targets_rejects_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = DebugAdapter::new();
        adapter.handle_request(1, "initialize", None);

        let dir = tempfile::tempdir()?;
        *lock_or_recover(&adapter.workspace_root, "test.workspace_root") =
            Some(dir.path().to_path_buf());

        let response = adapter.handle_request(
            2,
            "gotoTargets",
            Some(json!({
                "source": {"path": "../../../etc/passwd"},
                "line": 1
            })),
        );
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "gotoTargets with traversal path should fail");
                assert_eq!(command, "gotoTargets");
                let msg = message.as_deref().unwrap_or("");
                assert!(
                    msg.contains("Path validation failed"),
                    "should report path validation failure, got: {msg}"
                );
            }
            _ => return Err("Expected response".into()),
        }
        Ok(())
    }

    // --- Signal handling tests (#3028) ---

    #[test]
    fn test_send_continue_signal_does_not_panic_on_pid_1() {
        // PID 1 on Unix is init (EPERM), on Windows GenerateConsoleCtrlEvent returns 0.
        // Must not panic on any platform.
        let adapter = DebugAdapter::new();
        let _ = adapter.send_continue_signal(1);
    }

    #[test]
    fn test_send_interrupt_signal_does_not_panic_on_pid_1() {
        let adapter = DebugAdapter::new();
        let _ = adapter.send_interrupt_signal(1);
    }

    #[test]
    fn test_send_continue_signal_pid_zero_returns_false() {
        let adapter = DebugAdapter::new();
        assert!(!adapter.send_continue_signal(0));
    }

    #[test]
    fn test_send_interrupt_signal_pid_zero_returns_false() {
        let adapter = DebugAdapter::new();
        assert!(!adapter.send_interrupt_signal(0));
    }

    #[test]
    #[cfg(unix)]
    fn test_send_continue_signal_nonexistent_pid_returns_false() {
        let adapter = DebugAdapter::new();
        assert!(!adapter.send_continue_signal(999_999));
    }

    #[test]
    #[cfg(unix)]
    fn test_send_interrupt_signal_nonexistent_pid_returns_false() {
        let adapter = DebugAdapter::new();
        assert!(!adapter.send_interrupt_signal(999_999));
    }

    // ── context_re unit tests ─────────────────────────────────────────────────

    /// Helper: apply context_re to `line` and return (file, line_num) if matched.
    fn apply_context_re(line: &str) -> Option<(String, String)> {
        let re = context_re()?;
        let caps = re.captures(line)?;
        let file =
            caps.name("file").or_else(|| caps.name("file2")).map(|m| m.as_str().to_string())?;
        let line_num =
            caps.name("line").or_else(|| caps.name("line2")).map(|m| m.as_str().to_string())?;
        Some((file, line_num))
    }

    #[test]
    fn test_context_re_unix_path() {
        let result = apply_context_re("main::(/path/to/file.pl:42):");
        assert_eq!(result, Some(("/path/to/file.pl".to_string(), "42".to_string())));
    }

    #[test]
    fn test_context_re_windows_drive_letter_backslash() {
        // Windows drive-letter path: colon followed by backslash must be captured
        // as part of the file path, not treated as a line-number separator.
        let result = apply_context_re(r"main::(C:\Users\name\file.pl:42):");
        assert_eq!(result, Some((r"C:\Users\name\file.pl".to_string(), "42".to_string())));
    }

    #[test]
    fn test_context_re_windows_drive_letter_forward_slash() {
        // Forward-slash Windows path from Git Bash / cross-platform tools.
        let result = apply_context_re("main::(C:/Users/file.pl:7):");
        assert_eq!(result, Some(("C:/Users/file.pl".to_string(), "7".to_string())));
    }

    #[test]
    fn test_context_re_unc_path() {
        // UNC path (Windows network share).
        let result = apply_context_re(r"main::(\\server\share\file.pl:5):");
        assert_eq!(result, Some((r"\\server\share\file.pl".to_string(), "5".to_string())));
    }

    #[test]
    fn test_context_re_named_function() {
        // Func::Name context line.
        let result = apply_context_re("Foo::Bar::(/path/script.pl:10):");
        assert_eq!(result, Some(("/path/script.pl".to_string(), "10".to_string())));
    }

    #[test]
    fn test_context_re_windows_path_named_function() {
        // Func::Name context line with Windows path.
        let result = apply_context_re(r"Foo::Bar::(C:\path\script.pl:10):");
        assert_eq!(result, Some((r"C:\path\script.pl".to_string(), "10".to_string())));
    }

    #[test]
    fn test_context_re_no_match_path_with_spaces() {
        // Paths with spaces do not match — the character class excludes \s.
        let result = apply_context_re("main::(/path with spaces/file.pl:5):");
        assert!(result.is_none(), "paths with spaces should not match");
    }

    #[test]
    fn test_context_re_colon_digit_is_line_separator() {
        // Colon followed by digit is the line-number separator, not a path component.
        // "/path/file.pl:42" should yield file="/path/file.pl", line="42".
        let result = apply_context_re("main::(/path/file.pl:42):");
        assert_eq!(result, Some(("/path/file.pl".to_string(), "42".to_string())));
    }
}
