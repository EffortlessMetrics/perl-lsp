//! [`DapPeerBridge`] — the DAP **frontend** over a [`DebugBackend`].
//!
//! This is the reachability closure for the external-peer seam: it makes the
//! backend *live-drivable from a real DAP session*. It is the inverse of
//! [`super::native_perldb`]: where the native backend translates the model into
//! the adapter's DAP handlers, this bridge translates incoming **DAP requests →
//! model calls → DAP responses**, and converts backend [`DebugEvent`]s into DAP
//! events for the editor.
//!
//! ```text
//! editor ──DAP──▶ DapPeerBridge ──model──▶ DebugBackend ──peer proto──▶ ptkdb
//!        ◀─DAP───              ◀─events───            ◀────────────────
//! ```
//!
//! The bridge is a **parallel** path: it does not touch the native
//! [`crate::debug_adapter::DebugAdapter`] dispatch funnel (decision DF1 remains
//! deferred). [`run_external_peer_session`] drives it over a socket editor
//! connection; [`DapPeerBridge::dispatch`] / [`DapPeerBridge::poll_events`] are
//! the deterministic, testable core.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{Value, json};

use super::capabilities::{CatalogDapFlags, intersect_dap_capabilities};
use super::{
    AttachBackendParams, DebugBackend, EvaluateContext, EvaluateParams, InitializeBackendParams,
    LaunchBackendParams, SetBackendBreakpointsParams, SetFunctionBreakpointsParams,
    StackTraceParams,
};
use crate::breakpoint_oracle::{AstBreakpointOracle, BreakpointOracle};
use crate::debug_adapter::DapMessage;
use crate::model::{
    DebugBreakpoint, DebugEvent, DebugFunctionBreakpoint, DebugSource, FrameId, OutputCategory,
    StopReason, ThreadId, VariablesRef,
};

/// The DAP frontend over a [`DebugBackend`].
pub struct DapPeerBridge {
    backend: Box<dyn DebugBackend>,
    seq: i64,
    /// Whether a `terminated` DAP event has already been emitted this session,
    /// so a peer's own `debugger/terminated` (which may have been queued before
    /// our `peer/goodbye` on a `terminate`) does not produce a duplicate.
    terminated_emitted: bool,
}

impl DapPeerBridge {
    /// Create a bridge over `backend`.
    #[must_use]
    pub fn new(backend: Box<dyn DebugBackend>) -> Self {
        Self { backend, seq: 0, terminated_emitted: false }
    }

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    fn response(
        &mut self,
        request_seq: i64,
        command: &str,
        success: bool,
        body: Option<Value>,
        message: Option<String>,
    ) -> DapMessage {
        DapMessage::Response {
            seq: self.next_seq(),
            request_seq,
            success,
            command: command.to_string(),
            body,
            message,
        }
    }

    fn event(&mut self, event: &str, body: Option<Value>) -> DapMessage {
        DapMessage::Event { seq: self.next_seq(), event: event.to_string(), body }
    }

    /// Convert and drain any backend events into DAP event messages.
    ///
    /// Non-blocking: returns whatever the backend has queued right now.
    pub fn poll_events(&mut self) -> Vec<DapMessage> {
        let events = self.backend.drain_events();
        let mut out = Vec::new();
        for ev in events {
            self.push_dap_events(ev, &mut out);
        }
        out
    }

    fn push_dap_events(&mut self, ev: DebugEvent, out: &mut Vec<DapMessage>) {
        match ev {
            // The DAP `initialized` event (editor: "you may send configuration
            // now") is a DIFFERENT signal from the peer's `debugger/initialized`
            // (host: "the engine is ready"). The bridge emits the DAP one exactly
            // once, right after the initialize RESPONSE, and treats the peer
            // handshake (`peer/hello`) as the mirror-MVP readiness gate. The peer
            // readiness event is therefore NOT forwarded — emitting a second DAP
            // `initialized` would wrongly re-trigger the client's configuration
            // sequence. (Gating configuration on a peer's `debugger/initialized`
            // is future work for cooperative mode; the v1 integration target does
            // not require peers to send it — see PTKDB_PEER_INTEGRATION_TARGET.md.)
            DebugEvent::Initialized => {}
            DebugEvent::Stopped { reason, thread_id, .. } => {
                let body = json!({
                    "reason": dap_stop_reason(&reason),
                    "threadId": thread_id.0,
                    "allThreadsStopped": true,
                });
                out.push(self.event("stopped", Some(body)));
            }
            DebugEvent::Continued { thread_id } => {
                let body = json!({ "threadId": thread_id.0, "allThreadsContinued": true });
                out.push(self.event("continued", Some(body)));
            }
            DebugEvent::Output { category, output } => {
                let body = json!({ "category": category_str(category), "output": output });
                out.push(self.event("output", Some(body)));
            }
            DebugEvent::Terminated { exit_code } => {
                // Emit at most one `terminated` per session. If a DAP `terminate`
                // already emitted one, the peer's own `debugger/terminated`
                // (possibly queued before our `peer/goodbye`) is swallowed rather
                // than delivered as a duplicate.
                if !self.terminated_emitted {
                    self.terminated_emitted = true;
                    let body = exit_code.map(|c| json!({ "exitCode": c }));
                    out.push(self.event("terminated", body));
                }
            }
            DebugEvent::BreakpointsChanged { breakpoints } => {
                for bp in breakpoints {
                    let body = json!({
                        "reason": "changed",
                        "breakpoint": {
                            "id": bp.id,
                            "verified": bp.verified,
                            "line": bp.actual_position.line,
                            "message": bp.message,
                        },
                    });
                    out.push(self.event("breakpoint", Some(body)));
                }
            }
            // No standard DAP event for source facts; the editor obtains
            // breakable lines via `breakpointLocations`. Intentionally dropped.
            DebugEvent::SourceFacts { .. } => {}
        }
    }

    /// Dispatch a single DAP request. Returns the response message followed by
    /// any backend events that arrived while servicing it (drained after the
    /// call), so a caller can write them in order.
    pub fn dispatch(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> Vec<DapMessage> {
        let mut out = Vec::new();
        match command {
            "initialize" => {
                let params = parse_initialize(arguments.as_ref());
                match self.backend.initialize(params) {
                    Ok(()) => {
                        let body = self.capabilities_body();
                        out.push(self.response(request_seq, command, true, Some(body), None));
                        // DAP contract: emit `initialized` after the response so
                        // the client sends its configuration (breakpoints, etc.).
                        out.push(self.event("initialized", None));
                    }
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "launch" => {
                let params = parse_launch(arguments.as_ref());
                match self.backend.launch(params) {
                    Ok(_) => out.push(self.response(request_seq, command, true, None, None)),
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "attach" => {
                let params = parse_attach(arguments.as_ref());
                match self.backend.attach(params) {
                    Ok(_) => out.push(self.response(request_seq, command, true, None, None)),
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "setBreakpoints" => match self.handle_set_breakpoints(arguments.as_ref()) {
                Ok(body) => out.push(self.response(request_seq, command, true, Some(body), None)),
                Err(e) => out.push(self.error(request_seq, command, e)),
            },
            "setFunctionBreakpoints" => {
                match self.handle_set_function_breakpoints(arguments.as_ref()) {
                    Ok(body) => {
                        out.push(self.response(request_seq, command, true, Some(body), None))
                    }
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "continue" => {
                let tid = thread_id_arg(arguments.as_ref());
                match self.backend.continue_thread(tid) {
                    Ok(r) => {
                        let body = json!({ "allThreadsContinued": r.all_threads_continued });
                        out.push(self.response(request_seq, command, true, Some(body), None));
                    }
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "next" => self.step(request_seq, command, arguments.as_ref(), Step::Next, &mut out),
            "stepIn" => self.step(request_seq, command, arguments.as_ref(), Step::In, &mut out),
            "stepOut" => self.step(request_seq, command, arguments.as_ref(), Step::Out, &mut out),
            "pause" => {
                let tid = thread_id_arg(arguments.as_ref());
                match self.backend.pause(tid) {
                    Ok(()) => out.push(self.response(request_seq, command, true, None, None)),
                    Err(e) => out.push(self.error(request_seq, command, e)),
                }
            }
            "stackTrace" => match self.handle_stack_trace(arguments.as_ref()) {
                Ok(body) => out.push(self.response(request_seq, command, true, Some(body), None)),
                Err(e) => out.push(self.error(request_seq, command, e)),
            },
            "scopes" => match self.handle_scopes(arguments.as_ref()) {
                Ok(body) => out.push(self.response(request_seq, command, true, Some(body), None)),
                Err(e) => out.push(self.error(request_seq, command, e)),
            },
            "variables" => match self.handle_variables(arguments.as_ref()) {
                Ok(body) => out.push(self.response(request_seq, command, true, Some(body), None)),
                Err(e) => out.push(self.error(request_seq, command, e)),
            },
            "evaluate" => match self.handle_evaluate(arguments.as_ref()) {
                Ok(body) => out.push(self.response(request_seq, command, true, Some(body), None)),
                Err(e) => out.push(self.error(request_seq, command, e)),
            },
            "threads" => {
                // Perl's stock debugger is single-threaded; report one thread.
                let body = json!({ "threads": [{ "id": 1, "name": "main" }] });
                out.push(self.response(request_seq, command, true, Some(body), None));
            }
            "configurationDone" => {
                out.push(self.response(request_seq, command, true, None, None));
            }
            "breakpointLocations" => {
                // Answered locally from the AST oracle (the source file is on the
                // same host as perl-dap), independent of the peer.
                let body = handle_breakpoint_locations(arguments.as_ref());
                out.push(self.response(request_seq, command, true, Some(body), None));
            }
            "terminate" => {
                // DAP `terminate` (the editor's Stop button when the adapter
                // advertises `supportsTerminateRequest`): end the debuggee. In
                // mirror mode the peer owns the process, so this is best-effort —
                // ask the backend to disconnect *with* termination, then emit a
                // `terminated` event so the editor tears the session down instead
                // of leaving it running.
                let _ = self.backend.disconnect(true);
                out.push(self.response(request_seq, command, true, None, None));
                // Mark before draining backend events below so a peer-queued
                // `debugger/terminated` does not double up (see push_dap_events).
                self.terminated_emitted = true;
                out.push(self.event("terminated", None));
            }
            "disconnect" => {
                let terminate = arguments
                    .as_ref()
                    .and_then(|a| a.get("terminateDebuggee"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let _ = self.backend.disconnect(terminate);
                out.push(self.response(request_seq, command, true, None, None));
                // `disconnect { terminateDebuggee: true }` carries the same intent
                // as `terminate`, so emit `terminated` (and arm the dedup guard)
                // here too. Otherwise a client that takes this path never gets a
                // `terminated`, and a later peer-queued `debugger/terminated`
                // would reach the editor as an un-deduped event.
                if terminate && !self.terminated_emitted {
                    self.terminated_emitted = true;
                    out.push(self.event("terminated", None));
                }
            }
            other => {
                // Lenient: acknowledge unrecognized requests so a client is not
                // wedged, but carry no body. (mirror-MVP behavior.)
                tracing::warn!(command = other, "peer bridge: unhandled DAP request");
                out.push(self.response(request_seq, other, true, None, None));
            }
        }
        // Surface any events the backend queued while handling the request.
        out.extend(self.poll_events());
        out
    }

    fn step(
        &mut self,
        request_seq: i64,
        command: &str,
        args: Option<&Value>,
        which: Step,
        out: &mut Vec<DapMessage>,
    ) {
        let tid = thread_id_arg(args);
        let result = match which {
            Step::Next => self.backend.next(tid),
            Step::In => self.backend.step_in(tid),
            Step::Out => self.backend.step_out(tid),
        };
        match result {
            Ok(()) => out.push(self.response(request_seq, command, true, None, None)),
            Err(e) => out.push(self.error(request_seq, command, e)),
        }
    }

    fn error(&mut self, request_seq: i64, command: &str, e: super::BackendError) -> DapMessage {
        self.response(request_seq, command, false, None, Some(e.to_string()))
    }

    fn capabilities_body(&self) -> Value {
        let negotiated = intersect_dap_capabilities(
            &CatalogDapFlags::from_catalog(),
            &self.backend.capabilities(),
        );
        json!({
            "supportsConfigurationDoneRequest": true,
            "supportsTerminateRequest": true,
            // Answered locally from the AST oracle, so always available.
            "supportsBreakpointLocationsRequest": true,
            "supportsConditionalBreakpoints": negotiated.supports_conditional_breakpoints,
            "supportsHitConditionalBreakpoints": negotiated.supports_hit_conditional_breakpoints,
            "supportsLogPoints": negotiated.supports_log_points,
            "supportsFunctionBreakpoints": negotiated.supports_function_breakpoints,
            "supportsDataBreakpoints": negotiated.supports_data_breakpoints,
            "supportsEvaluateForHovers": negotiated.supports_evaluate_for_hovers,
            "supportsSetVariable": negotiated.supports_set_variable,
        })
    }

    fn handle_set_breakpoints(&mut self, args: Option<&Value>) -> super::BackendResult<Value> {
        let args = args.ok_or_else(|| super::BackendError::Protocol("missing arguments".into()))?;
        let source = dap_source(args.get("source"));
        let input = args.get("breakpoints").and_then(Value::as_array).cloned().unwrap_or_default();
        // DAP requires the response `breakpoints` array to match the request
        // positionally and in length. An entry missing `line` can't be sent to
        // the backend, but must still occupy its slot as `verified: false` rather
        // than being dropped (which would shift every later entry onto the wrong
        // requested line). Track, per input position, the index of its resolved
        // result — or `None` for the unbuildable entries.
        let mut breakpoints = Vec::new();
        let mut slots: Vec<Option<usize>> = Vec::with_capacity(input.len());
        for b in &input {
            match b.get("line").and_then(Value::as_u64) {
                Some(line) => {
                    slots.push(Some(breakpoints.len()));
                    breakpoints.push(DebugBreakpoint {
                        id: None,
                        source: source.clone(),
                        line: line as u32,
                        column: b.get("column").and_then(Value::as_u64).map(|c| c as u32),
                        condition: str_field(b, "condition"),
                        hit_condition: str_field(b, "hitCondition"),
                        log_message: str_field(b, "logMessage"),
                    });
                }
                None => slots.push(None),
            }
        }
        let resolved =
            self.backend.set_breakpoints(SetBackendBreakpointsParams { source, breakpoints })?;
        let bps: Vec<Value> = slots
            .iter()
            .map(|slot| match slot.and_then(|i| resolved.get(i)) {
                Some(r) => json!({
                    "id": r.id,
                    "verified": r.verified,
                    "line": r.actual_position.line,
                    "column": r.actual_position.column,
                    "message": r.message,
                }),
                // Either the entry lacked a `line`, or the backend returned fewer
                // results than requested — echo an unverified slot to keep the
                // response positionally aligned with the request.
                None => json!({ "verified": false, "message": "line required" }),
            })
            .collect();
        Ok(json!({ "breakpoints": bps }))
    }

    fn handle_set_function_breakpoints(
        &mut self,
        args: Option<&Value>,
    ) -> super::BackendResult<Value> {
        let input = args
            .and_then(|a| a.get("breakpoints"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        // Positional/length contract as in `handle_set_breakpoints`: an entry
        // missing `name` keeps its slot as `verified: false` instead of shifting
        // the array.
        let mut breakpoints = Vec::new();
        let mut slots: Vec<Option<usize>> = Vec::with_capacity(input.len());
        for b in &input {
            match str_field(b, "name") {
                Some(name) => {
                    slots.push(Some(breakpoints.len()));
                    breakpoints.push(DebugFunctionBreakpoint {
                        name,
                        condition: str_field(b, "condition"),
                    });
                }
                None => slots.push(None),
            }
        }
        let resolved =
            self.backend.set_function_breakpoints(SetFunctionBreakpointsParams { breakpoints })?;
        let bps: Vec<Value> = slots
            .iter()
            .map(|slot| match slot.and_then(|i| resolved.get(i)) {
                Some(r) => json!({ "id": r.id, "verified": r.verified }),
                None => json!({ "verified": false, "message": "name required" }),
            })
            .collect();
        Ok(json!({ "breakpoints": bps }))
    }

    fn handle_stack_trace(&mut self, args: Option<&Value>) -> super::BackendResult<Value> {
        let params = StackTraceParams {
            thread_id: thread_id_arg(args),
            start_frame: args
                .and_then(|a| a.get("startFrame"))
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            levels: args.and_then(|a| a.get("levels")).and_then(Value::as_u64).map(|v| v as u32),
        };
        let frames = self.backend.stack_trace(params)?;
        let total = frames.len();
        let out: Vec<Value> = frames
            .into_iter()
            .map(|f| {
                json!({
                    "id": f.id,
                    "name": f.name,
                    "source": { "path": f.source.path.to_string_lossy(), "name": f.source.name },
                    "line": f.line,
                    "column": f.column,
                })
            })
            .collect();
        Ok(json!({ "stackFrames": out, "totalFrames": total }))
    }

    fn handle_scopes(&mut self, args: Option<&Value>) -> super::BackendResult<Value> {
        let frame_id =
            FrameId(args.and_then(|a| a.get("frameId")).and_then(Value::as_i64).unwrap_or(0));
        let scopes = self.backend.scopes(frame_id)?;
        let out: Vec<Value> = scopes
            .into_iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "variablesReference": s.variables_reference.0,
                    "expensive": s.expensive,
                })
            })
            .collect();
        Ok(json!({ "scopes": out }))
    }

    fn handle_variables(&mut self, args: Option<&Value>) -> super::BackendResult<Value> {
        let vref = VariablesRef(
            args.and_then(|a| a.get("variablesReference")).and_then(Value::as_i64).unwrap_or(0),
        );
        let vars = self.backend.variables(vref)?;
        let out: Vec<Value> = vars
            .into_iter()
            .map(|v| {
                json!({
                    "name": v.name,
                    "value": v.value,
                    "type": v.type_name,
                    "variablesReference": v.variables_reference.map(|r| r.0).unwrap_or(0),
                    "namedVariables": v.named_variables,
                    "indexedVariables": v.indexed_variables,
                })
            })
            .collect();
        Ok(json!({ "variables": out }))
    }

    fn handle_evaluate(&mut self, args: Option<&Value>) -> super::BackendResult<Value> {
        let expression = args
            .and_then(|a| a.get("expression"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let frame_id = args.and_then(|a| a.get("frameId")).and_then(Value::as_i64).map(FrameId);
        let context = args
            .and_then(|a| a.get("context"))
            .and_then(Value::as_str)
            .map(evaluate_context)
            .unwrap_or(EvaluateContext::Repl);
        let result = self.backend.evaluate(EvaluateParams { expression, frame_id, context })?;
        Ok(json!({
            "result": result.result,
            "type": result.type_name,
            "variablesReference": result.variables_reference.map(|r| r.0).unwrap_or(0),
        }))
    }
}

enum Step {
    Next,
    In,
    Out,
}

// ---------------------------------------------------------------------------
// Production session driver (socket editor transport)
// ---------------------------------------------------------------------------

/// Drive a [`DapPeerBridge`] over a socket editor connection.
///
/// Reads Content-Length framed DAP requests from the editor, dispatches each to
/// the bridge, and writes framed responses/events back. Between reads it polls
/// the backend for asynchronous events (stops, output, termination) so they
/// reach the editor promptly — a short read timeout on the socket paces the
/// interleave (the same technique the peer backend uses internally).
///
/// A concrete `TcpStream` is required (not a generic reader) because the
/// interleave depends on `set_read_timeout`; the socket transport uses this.
/// The stdio counterpart is [`run_external_peer_session_stdio`], which uses a
/// reader thread + channel to interleave events since stdin has no read timeout.
///
/// # Errors
/// Returns a transport error if the socket read/write fails irrecoverably.
pub fn run_external_peer_session(
    stream: TcpStream,
    mut bridge: DapPeerBridge,
    poll_interval: Duration,
) -> std::io::Result<()> {
    use perl_lsp_rs_core::transport::ContentLengthFramer;

    stream.set_read_timeout(Some(poll_interval))?;
    let mut reader = stream.try_clone()?;
    let mut writer = stream;
    let mut framer = ContentLengthFramer::new();
    let mut buf = [0u8; 8 * 1024];

    loop {
        // Deliver any asynchronous backend events first.
        let events = bridge.poll_events();
        if !events.is_empty() {
            write_dap_msgs(&mut writer, &events)?;
        }

        match reader.read(&mut buf) {
            Ok(0) => break, // editor disconnected
            Ok(n) => {
                framer.push(&buf[..n]);
                loop {
                    match framer.try_next() {
                        Ok(Some(body)) => {
                            let (out, disconnect) = dispatch_frame(&mut bridge, &body);
                            write_dap_msgs(&mut writer, &out)?;
                            if disconnect {
                                return Ok(());
                            }
                        }
                        Ok(None) => break,
                        // `ContentLengthFramer::try_next` already discards the
                        // malformed header block before returning an error, so a
                        // valid subsequent frame can still be parsed. Skip and keep
                        // going rather than tearing down the whole session on one
                        // bad frame — same recoverable handling as the stdio driver
                        // (`run_peer_session_threaded`) and
                        // `ContentLengthMessageReader::read_next`.
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "peer bridge (socket): dropping malformed DAP frame"
                            );
                        }
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // No editor input this tick; loop to poll events again.
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Drive a [`DapPeerBridge`] over **stdio** — the default DAP transport an editor
/// uses when it spawns the adapter as a child process (`perl-dap --external-peer
/// HOST:PORT` with no `--socket`).
///
/// stdin has no read timeout, so a dedicated reader thread frames requests off
/// stdin and forwards each frame body over a channel; the main loop interleaves
/// draining backend events to stdout with a `recv_timeout` on that channel, so
/// asynchronous stops/output reach the editor promptly without blocking on
/// stdin. This is the stdio counterpart of [`run_external_peer_session`].
///
/// # Errors
/// Returns a transport error if writing framed messages to stdout fails.
pub fn run_external_peer_session_stdio(
    bridge: DapPeerBridge,
    poll_interval: Duration,
) -> std::io::Result<()> {
    // `stdin()`/`stdout()` are `'static` and `Send`, so they move cleanly into
    // the generic threaded driver (the reader half runs on its own thread).
    run_peer_session_threaded(std::io::stdin(), std::io::stdout(), bridge, poll_interval)
}

/// Generic threaded driver: read framed DAP requests from `reader_src` on a
/// dedicated thread, dispatch them to `bridge`, and write framed
/// responses/events to `writer` on the calling thread, interleaving backend
/// event delivery on `poll_interval` ticks.
///
/// Used by [`run_external_peer_session_stdio`] (stdin/stdout) and exercised in
/// tests over in-memory pipes. The reader thread is detached rather than joined:
/// on a DAP `disconnect` the editor may not close its write half immediately, so
/// joining could block; the thread exits on stdin EOF or process teardown.
///
/// # Errors
/// Returns a transport error if writing framed messages to `writer` fails.
fn run_peer_session_threaded<R, W>(
    reader_src: R,
    mut writer: W,
    mut bridge: DapPeerBridge,
    poll_interval: Duration,
) -> std::io::Result<()>
where
    R: Read + Send + 'static,
    W: Write,
{
    use perl_lsp_rs_core::transport::ContentLengthFramer;
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _reader = std::thread::spawn(move || {
        let mut src = reader_src;
        let mut framer = ContentLengthFramer::new();
        let mut buf = [0u8; 8 * 1024];
        loop {
            match src.read(&mut buf) {
                Ok(0) => break, // EOF: editor closed its write half
                Ok(n) => {
                    framer.push(&buf[..n]);
                    loop {
                        match framer.try_next() {
                            // Receiver gone (session ended) — stop reading.
                            Ok(Some(body)) => {
                                if tx.send(body).is_err() {
                                    return;
                                }
                            }
                            Ok(None) => break,
                            // `ContentLengthFramer::try_next` already discards the
                            // malformed header block before returning an error, so
                            // the buffer can still hold a valid subsequent frame.
                            // Skip and keep parsing rather than tearing down the
                            // whole session on one bad frame — mirrors the
                            // recoverable-error handling in
                            // `ContentLengthMessageReader::read_next`.
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "peer bridge (stdio): dropping malformed DAP frame"
                                );
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });

    loop {
        // Deliver any asynchronous backend events first.
        let events = bridge.poll_events();
        if !events.is_empty() {
            write_dap_msgs(&mut writer, &events)?;
        }

        match rx.recv_timeout(poll_interval) {
            Ok(body) => {
                let (out, disconnect) = dispatch_frame(&mut bridge, &body);
                write_dap_msgs(&mut writer, &out)?;
                if disconnect {
                    break;
                }
            }
            // No editor input this tick; loop to poll events again.
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            // Reader thread ended (stdin closed / malformed): end the session.
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

/// Serialize and Content-Length frame each DAP message to `writer`.
///
/// On the (practically impossible) serialize failure, the message is skipped
/// rather than writing a `Content-Length: 0` frame that would corrupt the
/// stream and desync the client.
fn write_dap_msgs<W: Write>(writer: &mut W, msgs: &[DapMessage]) -> std::io::Result<()> {
    use perl_lsp_rs_core::transport::frame;
    for m in msgs {
        match serde_json::to_vec(m) {
            Ok(body) => writer.write_all(&frame(&body))?,
            Err(e) => {
                tracing::error!(error = %e, "peer bridge: dropping unserializable DAP message")
            }
        }
    }
    writer.flush()
}

/// Dispatch one raw DAP request frame to the bridge, returning the messages to
/// write back and whether the request was a `disconnect` (session should end).
///
/// The frame is parsed leniently: `command` and `seq` are extracted from the raw
/// JSON even if the typed `Request` shape doesn't match exactly (e.g. `seq` sent
/// as a JSON float), so a client is never left hanging without a response. A
/// frame with no `command` is dropped.
fn dispatch_frame(bridge: &mut DapPeerBridge, body: &[u8]) -> (Vec<DapMessage>, bool) {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let Some(command) = v.get("command").and_then(Value::as_str) else {
        tracing::warn!("peer bridge: dropping DAP frame with no `command`");
        return (Vec::new(), false);
    };
    let seq =
        v.get("seq").and_then(|s| s.as_i64().or_else(|| s.as_f64().map(|f| f as i64))).unwrap_or(0);
    let out = bridge.dispatch(seq, command, v.get("arguments").cloned());
    let disconnect = command == "disconnect";
    (out, disconnect)
}

// ---------------------------------------------------------------------------
// Argument / value translation helpers
// ---------------------------------------------------------------------------

fn parse_initialize(args: Option<&Value>) -> InitializeBackendParams {
    InitializeBackendParams {
        client_id: args
            .and_then(|a| a.get("clientID"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        adapter_id: args
            .and_then(|a| a.get("adapterID"))
            .and_then(Value::as_str)
            .unwrap_or("perl")
            .to_string(),
        lines_start_at_1: args
            .and_then(|a| a.get("linesStartAt1"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        columns_start_at_1: args
            .and_then(|a| a.get("columnsStartAt1"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

fn parse_launch(args: Option<&Value>) -> LaunchBackendParams {
    let mut params = LaunchBackendParams::default();
    if let Some(a) = args {
        if let Some(p) = a.get("program").and_then(Value::as_str) {
            params.program = p.into();
        }
        if let Some(arr) = a.get("args").and_then(Value::as_array) {
            params.args = arr.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect();
        }
        params.cwd = a.get("cwd").and_then(Value::as_str).map(Into::into);
        if let Some(obj) = a.get("env").and_then(Value::as_object) {
            params.env = obj
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
        }
        if let Some(arr) = a.get("includePaths").and_then(Value::as_array) {
            params.include_paths = arr.iter().filter_map(|v| v.as_str().map(Into::into)).collect();
        }
        params.stop_on_entry = a.get("stopOnEntry").and_then(Value::as_bool).unwrap_or(false);
    }
    params
}

fn parse_attach(args: Option<&Value>) -> AttachBackendParams {
    AttachBackendParams {
        host: args
            .and_then(|a| a.get("host"))
            .and_then(Value::as_str)
            .unwrap_or("127.0.0.1")
            .to_string(),
        port: args.and_then(|a| a.get("port")).and_then(Value::as_u64).unwrap_or(0) as u16,
    }
}

fn thread_id_arg(args: Option<&Value>) -> ThreadId {
    ThreadId(args.and_then(|a| a.get("threadId")).and_then(Value::as_i64).unwrap_or(1))
}

fn dap_source(v: Option<&Value>) -> DebugSource {
    let path = v.and_then(|s| s.get("path")).and_then(Value::as_str).unwrap_or_default();
    DebugSource {
        path: path.into(),
        name: v.and_then(|s| s.get("name")).and_then(Value::as_str).map(ToString::to_string),
        source_reference: v.and_then(|s| s.get("sourceReference")).and_then(Value::as_i64),
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(ToString::to_string)
}

/// Answer a DAP `breakpointLocations` request from the local AST oracle.
///
/// Reads the source from disk (it is on the same host as `perl-dap`) and returns
/// the breakable lines within the requested `[line, endLine]` range. On any error
/// (missing args, unreadable/unparseable source) returns an empty set rather than
/// failing the request — the editor treats "no breakable locations" gracefully.
fn handle_breakpoint_locations(args: Option<&Value>) -> Value {
    let empty = json!({ "breakpoints": [] });
    let Some(args) = args else { return empty };
    let Some(path) = args.get("source").and_then(|s| s.get("path")).and_then(Value::as_str) else {
        return empty;
    };
    // DAP requires `line`; `endLine` is optional and defaults to `line` (a
    // single-line query). A request without `line` is malformed — return the
    // empty set rather than treating a lone `endLine` as `1..=endLine`, which
    // would leak every breakable line up to it.
    let Some(start) = args.get("line").and_then(Value::as_u64).map(|v| v as u32) else {
        return empty;
    };
    let end = args.get("endLine").and_then(Value::as_u64).map(|v| v as u32).unwrap_or(start);
    let Ok(text) = std::fs::read_to_string(path) else {
        return empty;
    };
    let Ok(oracle) = AstBreakpointOracle::new(DebugSource::from_path(path), &text) else {
        return empty;
    };
    let locations: Vec<Value> = oracle
        .breakable_line_candidates()
        .into_iter()
        .filter(|&line| line >= start && line <= end)
        .map(|line| json!({ "line": line }))
        .collect();
    json!({ "breakpoints": locations })
}

fn dap_stop_reason(reason: &StopReason) -> String {
    match reason {
        StopReason::Entry => "entry".into(),
        StopReason::Step => "step".into(),
        StopReason::Breakpoint => "breakpoint".into(),
        StopReason::FunctionBreakpoint => "function breakpoint".into(),
        StopReason::DataBreakpoint => "data breakpoint".into(),
        StopReason::Exception => "exception".into(),
        StopReason::Pause => "pause".into(),
        StopReason::Unknown(s) => s.clone(),
    }
}

fn category_str(c: OutputCategory) -> &'static str {
    c.as_dap_category()
}

fn evaluate_context(c: &str) -> EvaluateContext {
    match c {
        "watch" => EvaluateContext::Watch,
        "repl" => EvaluateContext::Repl,
        "hover" => EvaluateContext::Hover,
        "variables" => EvaluateContext::Variables,
        other => EvaluateContext::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::capabilities::DebugBackendCapabilities;
    use crate::backend::{
        AttachResult, BackendResult, ContinueResult, DebugBackend, EvaluateResult, LaunchResult,
    };
    use crate::model::{
        DebugPosition, DebugScope, DebugStackFrame, DebugVariable, ResolvedBreakpoint,
    };

    #[derive(Default)]
    struct ScriptBackend {
        events: Vec<DebugEvent>,
    }

    impl DebugBackend for ScriptBackend {
        fn name(&self) -> &str {
            "script"
        }
        fn capabilities(&self) -> DebugBackendCapabilities {
            DebugBackendCapabilities::ptkdb_v1_defaults()
        }
        fn initialize(&mut self, _p: InitializeBackendParams) -> BackendResult<()> {
            Ok(())
        }
        fn launch(&mut self, _p: LaunchBackendParams) -> BackendResult<LaunchResult> {
            Ok(LaunchResult { success: true })
        }
        fn attach(&mut self, _p: AttachBackendParams) -> BackendResult<AttachResult> {
            Ok(AttachResult { success: true })
        }
        fn set_breakpoints(
            &mut self,
            p: SetBackendBreakpointsParams,
        ) -> BackendResult<Vec<ResolvedBreakpoint>> {
            Ok(p.breakpoints
                .into_iter()
                .enumerate()
                .map(|(i, b)| ResolvedBreakpoint {
                    id: i as i64 + 1,
                    verified: true,
                    actual_position: DebugPosition {
                        source: b.source,
                        line: b.line,
                        column: b.column,
                    },
                    message: None,
                })
                .collect())
        }
        fn set_function_breakpoints(
            &mut self,
            _p: SetFunctionBreakpointsParams,
        ) -> BackendResult<Vec<ResolvedBreakpoint>> {
            Ok(Vec::new())
        }
        fn continue_thread(&mut self, tid: ThreadId) -> BackendResult<ContinueResult> {
            // Simulate the debuggee resuming then hitting a breakpoint.
            self.events.push(DebugEvent::Continued { thread_id: tid });
            self.events.push(DebugEvent::Stopped {
                reason: StopReason::Breakpoint,
                thread_id: tid,
                position: None,
            });
            Ok(ContinueResult { all_threads_continued: true })
        }
        fn next(&mut self, _t: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn step_in(&mut self, _t: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn step_out(&mut self, _t: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn pause(&mut self, _t: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn stack_trace(&mut self, _p: StackTraceParams) -> BackendResult<Vec<DebugStackFrame>> {
            Ok(vec![DebugStackFrame {
                id: 1,
                name: "main::run".into(),
                source: DebugSource::from_path("/work/script.pl"),
                line: 42,
                column: 1,
            }])
        }
        fn scopes(&mut self, _f: FrameId) -> BackendResult<Vec<DebugScope>> {
            Ok(vec![DebugScope {
                name: "Locals".into(),
                variables_reference: VariablesRef(1000),
                expensive: false,
            }])
        }
        fn variables(&mut self, _r: VariablesRef) -> BackendResult<Vec<DebugVariable>> {
            Ok(vec![DebugVariable {
                name: "$x".into(),
                value: "42".into(),
                type_name: Some("scalar".into()),
                variables_reference: None,
                indexed_variables: None,
                named_variables: None,
            }])
        }
        fn evaluate(&mut self, p: EvaluateParams) -> BackendResult<EvaluateResult> {
            Ok(EvaluateResult {
                result: format!("={}", p.expression),
                type_name: None,
                variables_reference: None,
            })
        }
        fn drain_events(&mut self) -> Vec<DebugEvent> {
            std::mem::take(&mut self.events)
        }
        fn disconnect(&mut self, _t: bool) -> BackendResult<()> {
            Ok(())
        }
    }

    fn bridge() -> DapPeerBridge {
        DapPeerBridge::new(Box::new(ScriptBackend::default()))
    }

    fn as_response(msg: &DapMessage) -> (&str, bool, Option<&Value>) {
        match msg {
            DapMessage::Response { command, success, body, .. } => {
                (command.as_str(), *success, body.as_ref())
            }
            _ => panic!("expected response, got {msg:?}"),
        }
    }

    fn event_name(msg: &DapMessage) -> &str {
        match msg {
            DapMessage::Event { event, .. } => event.as_str(),
            _ => panic!("expected event, got {msg:?}"),
        }
    }

    #[test]
    fn initialize_returns_capabilities_and_initialized_event() {
        let mut b = bridge();
        let out = b.dispatch(1, "initialize", None);
        assert_eq!(out.len(), 2);
        let (cmd, ok, body) = as_response(&out[0]);
        assert_eq!(cmd, "initialize");
        assert!(ok);
        let caps = body.expect("capabilities");
        assert_eq!(caps["supportsConfigurationDoneRequest"], true);
        assert_eq!(caps["supportsBreakpointLocationsRequest"], true);
        // ptkdb v1 negotiated: no logpoints/data breakpoints.
        assert_eq!(caps["supportsLogPoints"], false);
        assert_eq!(caps["supportsDataBreakpoints"], false);
        assert_eq!(event_name(&out[1]), "initialized");
    }

    #[test]
    fn set_breakpoints_translates_to_dap_body() {
        let mut b = bridge();
        let args = json!({
            "source": { "path": "/work/script.pl" },
            "breakpoints": [{ "line": 42, "condition": "$x > 10" }, { "line": 7 }],
        });
        let out = b.dispatch(2, "setBreakpoints", Some(args));
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok);
        let bps = body.expect("body")["breakpoints"].as_array().expect("array");
        assert_eq!(bps.len(), 2);
        assert_eq!(bps[0]["verified"], true);
        assert_eq!(bps[0]["line"], 42);
    }

    #[test]
    fn set_breakpoints_keeps_positional_slots_for_line_less_entries() {
        // DAP requires the response array to match the request positionally and
        // in length. A middle entry missing `line` must occupy its slot as
        // `verified: false`, not be dropped (which would shift line 9 onto slot 1).
        let mut b = bridge();
        let args = json!({
            "source": { "path": "/work/script.pl" },
            "breakpoints": [{ "line": 3 }, { "condition": "$x" }, { "line": 9 }],
        });
        let out = b.dispatch(2, "setBreakpoints", Some(args));
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert_eq!(bps.len(), 3, "response length must equal request length: {bps:?}");
        assert_eq!(bps[0]["verified"], true);
        assert_eq!(bps[0]["line"], 3);
        assert_eq!(bps[1]["verified"], false, "the line-less entry stays in slot 1");
        assert_eq!(bps[1]["message"], "line required");
        assert_eq!(bps[2]["verified"], true);
        assert_eq!(bps[2]["line"], 9);
    }

    #[test]
    fn set_function_breakpoints_keeps_positional_slots_for_name_less_entries() {
        let mut b = bridge();
        // The ScriptBackend returns no function breakpoints, so the only slot that
        // can be `verified: true` is none — but the name-less entry must still be
        // echoed as its own `verified: false` slot rather than dropped.
        let args = json!({ "breakpoints": [{ "condition": "1" }, { "name": "main::run" }] });
        let out = b.dispatch(2, "setFunctionBreakpoints", Some(args));
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert_eq!(bps.len(), 2, "response length must equal request length: {bps:?}");
        assert_eq!(bps[0]["verified"], false);
        assert_eq!(bps[0]["message"], "name required");
    }

    #[test]
    fn disconnect_with_terminate_emits_terminated_and_arms_dedup() {
        let mut b = bridge();
        let out = b.dispatch(2, "disconnect", Some(json!({ "terminateDebuggee": true })));
        assert!(
            out.iter()
                .any(|m| matches!(m, DapMessage::Event { event, .. } if event == "terminated")),
            "disconnect(terminateDebuggee=true) must emit terminated: {out:?}"
        );
        // Dedup armed: a subsequent peer-queued terminated is suppressed.
        let mut more = Vec::new();
        b.push_dap_events(DebugEvent::Terminated { exit_code: Some(0) }, &mut more);
        assert!(more.is_empty(), "second terminated must be suppressed: {more:?}");
    }

    #[test]
    fn disconnect_without_terminate_does_not_emit_terminated() {
        let mut b = bridge();
        let out = b.dispatch(2, "disconnect", Some(json!({ "terminateDebuggee": false })));
        assert!(
            !out.iter()
                .any(|m| matches!(m, DapMessage::Event { event, .. } if event == "terminated")),
            "a plain disconnect must not synthesize terminated: {out:?}"
        );
    }

    #[test]
    fn continue_emits_continued_then_stopped_after_response() {
        let mut b = bridge();
        let out = b.dispatch(3, "continue", Some(json!({ "threadId": 1 })));
        // response, then the two events the backend queued.
        let (cmd, ok, body) = as_response(&out[0]);
        assert_eq!(cmd, "continue");
        assert!(ok);
        assert_eq!(body.expect("body")["allThreadsContinued"], true);
        let events: Vec<&str> = out[1..].iter().map(event_name).collect();
        assert_eq!(events, vec!["continued", "stopped"]);
        // The stopped event carries the DAP reason + threadId.
        if let DapMessage::Event { body: Some(b), .. } = &out[2] {
            assert_eq!(b["reason"], "breakpoint");
            assert_eq!(b["threadId"], 1);
            assert_eq!(b["allThreadsStopped"], true);
        } else {
            panic!("expected stopped event body");
        }
    }

    #[test]
    fn stack_scopes_variables_evaluate_round_trip() {
        let mut b = bridge();
        let st = b.dispatch(4, "stackTrace", Some(json!({ "threadId": 1 })));
        let frames =
            as_response(&st[0]).2.expect("body")["stackFrames"].as_array().expect("frames").clone();
        assert_eq!(frames[0]["name"], "main::run");
        assert_eq!(frames[0]["line"], 42);
        assert_eq!(frames[0]["source"]["path"], "/work/script.pl");

        let sc = b.dispatch(5, "scopes", Some(json!({ "frameId": 1 })));
        assert_eq!(as_response(&sc[0]).2.expect("body")["scopes"][0]["variablesReference"], 1000);

        let va = b.dispatch(6, "variables", Some(json!({ "variablesReference": 1000 })));
        let vars = as_response(&va[0]).2.expect("body")["variables"].clone();
        assert_eq!(vars[0]["name"], "$x");
        assert_eq!(vars[0]["value"], "42");
        assert_eq!(vars[0]["variablesReference"], 0);

        let ev = b.dispatch(7, "evaluate", Some(json!({ "expression": "$x", "context": "watch" })));
        assert_eq!(as_response(&ev[0]).2.expect("body")["result"], "=$x");
    }

    #[test]
    fn breakpoint_locations_reports_breakable_lines_from_ast() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(f, "# a comment").expect("w"); // line 1 — not breakable
        writeln!(f, "my $x = 1;").expect("w"); // line 2 — breakable
        writeln!(f, "print $x;").expect("w"); // line 3 — breakable
        let path = f.path().to_string_lossy().to_string();

        let mut b = bridge();
        let out = b.dispatch(
            9,
            "breakpointLocations",
            Some(json!({ "source": { "path": path }, "line": 1, "endLine": 3 })),
        );
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        let lines: Vec<i64> = bps.iter().filter_map(|b| b["line"].as_i64()).collect();
        assert!(lines.contains(&2), "line 2 is breakable: {lines:?}");
        assert!(!lines.contains(&1), "comment line 1 is excluded: {lines:?}");
    }

    #[test]
    fn breakpoint_locations_missing_line_returns_empty_not_all_lines() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(f, "my $x = 1;").expect("w");
        let path = f.path().to_string_lossy().to_string();

        let mut b = bridge();
        // No "line" field at all. DAP marks `line` required, but a client that
        // omits it must not be answered with every breakable line in the file
        // (or crash) — the handler must still return the empty-set contract.
        let out =
            b.dispatch(20, "breakpointLocations", Some(json!({ "source": { "path": path } })));
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok, "the request itself still succeeds");
        let bps = body.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "missing line yields empty set, not every line: {bps:?}");
    }

    #[test]
    fn breakpoint_locations_only_end_line_returns_empty_not_all_lines() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(f, "my $x = 1;").expect("w"); // line 1 — breakable
        writeln!(f, "my $y = 2;").expect("w"); // line 2 — breakable
        let path = f.path().to_string_lossy().to_string();

        let mut b = bridge();
        // `endLine` present but `line` absent is the same malformed class as
        // "missing line" — it must NOT return every breakable line up to endLine.
        let out = b.dispatch(
            22,
            "breakpointLocations",
            Some(json!({ "source": { "path": path }, "endLine": 100 })),
        );
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok, "the request itself still succeeds");
        let bps = body.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "endLine-only (no line) yields empty set: {bps:?}");
    }

    #[test]
    fn breakpoint_locations_line_only_defaults_end_to_that_line() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(f, "# comment").expect("w"); // line 1 — not breakable
        writeln!(f, "my $x = 1;").expect("w"); // line 2 — breakable
        let path = f.path().to_string_lossy().to_string();

        let mut b = bridge();
        // A valid single-line query (`line` with no `endLine`) must still work:
        // endLine defaults to line, so line 2 is reported.
        let out = b.dispatch(
            23,
            "breakpointLocations",
            Some(json!({ "source": { "path": path }, "line": 2 })),
        );
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        let lines: Vec<i64> = bps.iter().filter_map(|b| b["line"].as_i64()).collect();
        assert_eq!(lines, vec![2], "line-only query reports just that breakable line: {lines:?}");
    }

    #[test]
    fn breakpoint_locations_end_line_before_start_line_returns_empty() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(f, "my $x = 1;").expect("w"); // line 1
        writeln!(f, "my $y = 2;").expect("w"); // line 2
        writeln!(f, "print $x + $y;").expect("w"); // line 3
        let path = f.path().to_string_lossy().to_string();

        let mut b = bridge();
        let out = b.dispatch(
            21,
            "breakpointLocations",
            Some(json!({ "source": { "path": path }, "line": 3, "endLine": 1 })),
        );
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "endLine < line is an empty (not inverted) range: {bps:?}");
    }

    #[test]
    fn breakpoint_locations_unreadable_path_returns_empty_not_an_error_response() {
        let mut b = bridge();
        let out = b.dispatch(
            22,
            "breakpointLocations",
            Some(json!({
                "source": { "path": "/nonexistent/definitely-not-a-real-path.pl" },
                "line": 1,
                "endLine": 10,
            })),
        );
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok, "an unreadable source must not fail the DAP request");
        let bps = body.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "unreadable path yields empty set: {bps:?}");
    }

    #[test]
    fn breakpoint_locations_missing_source_path_returns_empty() {
        let mut b = bridge();
        let out = b.dispatch(23, "breakpointLocations", Some(json!({ "line": 1, "endLine": 3 })));
        let bps =
            as_response(&out[0]).2.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "missing source.path yields empty set: {bps:?}");
    }

    #[test]
    fn breakpoint_locations_missing_arguments_returns_empty() {
        let mut b = bridge();
        let out = b.dispatch(24, "breakpointLocations", None);
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok, "even a bodyless breakpointLocations request must get a success response");
        let bps = body.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert!(bps.is_empty(), "missing arguments yields empty set: {bps:?}");
    }

    #[test]
    fn terminate_disconnects_the_backend_and_emits_terminated() {
        // The bridge advertises supportsTerminateRequest, so a DAP `terminate`
        // must be handled explicitly (not fall through the lenient default):
        // a success response plus a `terminated` event so the editor ends the
        // session rather than leaving the external-peer debuggee running.
        let mut b = bridge();
        let out = b.dispatch(9, "terminate", None);
        let (cmd, ok, _) = as_response(&out[0]);
        assert_eq!(cmd, "terminate");
        assert!(ok, "terminate must be acknowledged");
        assert!(
            out.iter()
                .any(|m| matches!(m, DapMessage::Event { event, .. } if event == "terminated")),
            "terminate must emit a `terminated` event: {out:?}"
        );

        // A peer's own `debugger/terminated`, queued before our `peer/goodbye`
        // and drained afterwards, must NOT produce a second DAP `terminated`.
        let mut more = Vec::new();
        b.push_dap_events(DebugEvent::Terminated { exit_code: Some(0) }, &mut more);
        assert!(
            more.is_empty(),
            "a second terminated must be suppressed after terminate: {more:?}"
        );
    }

    #[test]
    fn threads_reports_single_main_thread() {
        let mut b = bridge();
        let out = b.dispatch(8, "threads", None);
        let threads = as_response(&out[0]).2.expect("body")["threads"].clone();
        assert_eq!(threads[0]["id"], 1);
        assert_eq!(threads[0]["name"], "main");
    }

    #[test]
    fn output_and_terminated_events_translate() {
        let mut b = bridge();
        // Inject events directly through the backend and poll.
        b.push_dap_events(
            DebugEvent::Output { category: OutputCategory::Stderr, output: "boom\n".into() },
            &mut Vec::new(),
        );
        // Use dispatch of a no-op that drains: simulate via a fresh backend event.
        let mut msgs = Vec::new();
        b.push_dap_events(DebugEvent::Terminated { exit_code: Some(0) }, &mut msgs);
        assert_eq!(event_name(&msgs[0]), "terminated");
        if let DapMessage::Event { body: Some(body), .. } = &msgs[0] {
            assert_eq!(body["exitCode"], 0);
        } else {
            panic!("terminated body");
        }
    }

    #[test]
    fn peer_initialized_readiness_is_not_forwarded_as_a_second_dap_initialized() {
        // The DAP `initialized` event is emitted once on the initialize response;
        // a peer's `debugger/initialized` readiness signal must NOT become a
        // second DAP `initialized` (which would re-trigger client configuration).
        let mut b = bridge();
        let mut out = Vec::new();
        b.push_dap_events(DebugEvent::Initialized, &mut out);
        assert!(out.is_empty(), "peer readiness must not emit a DAP event");
    }

    #[test]
    fn threaded_driver_runs_a_dap_session_over_pipes() {
        use perl_lsp_rs_core::transport::{ContentLengthFramer, frame};
        use std::io::Cursor;
        use std::sync::{Arc, Mutex};

        // A `Write` sink that appends into a shared buffer, so the test thread
        // can inspect what the driver wrote after it returns. This exercises the
        // same generic driver that `run_external_peer_session_stdio` uses over
        // real stdin/stdout, but over in-memory pipes.
        #[derive(Clone)]
        struct SharedSink(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for SharedSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        // Editor input: an initialize request, then disconnect. The driver must
        // respond to both and return once it sees disconnect.
        let frame_of = |v: Value| frame(&serde_json::to_vec(&v).expect("ser"));
        let mut input = frame_of(
            json!({ "seq": 1, "type": "request", "command": "initialize", "arguments": {} }),
        );
        input.extend_from_slice(&frame_of(
            json!({ "seq": 2, "type": "request", "command": "disconnect" }),
        ));

        let out_buf = Arc::new(Mutex::new(Vec::new()));
        run_peer_session_threaded(
            Cursor::new(input),
            SharedSink(out_buf.clone()),
            bridge(),
            Duration::from_millis(5),
        )
        .expect("session ok");

        // Reparse the framed output stream and collect response commands + events.
        let raw = out_buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut framer = ContentLengthFramer::new();
        framer.push(&raw);
        let (mut commands, mut events) = (Vec::new(), Vec::new());
        while let Ok(Some(body)) = framer.try_next() {
            let v: Value = serde_json::from_slice(&body).expect("json");
            if v.get("type").and_then(Value::as_str) == Some("response") {
                if let Some(c) = v.get("command").and_then(Value::as_str) {
                    commands.push(c.to_string());
                }
            }
            if let Some(e) = v.get("event").and_then(Value::as_str) {
                events.push(e.to_string());
            }
        }
        assert!(commands.contains(&"initialize".to_string()), "commands: {commands:?}");
        assert!(commands.contains(&"disconnect".to_string()), "commands: {commands:?}");
        assert!(events.contains(&"initialized".to_string()), "events: {events:?}");
    }

    #[test]
    fn threaded_driver_recovers_from_a_leading_malformed_frame() {
        use perl_lsp_rs_core::transport::{ContentLengthFramer, frame};
        use std::io::Cursor;
        use std::sync::{Arc, Mutex};

        // `ContentLengthFramer::try_next` discards a malformed header block
        // before returning its error, so a single garbled frame must not end
        // the whole session — the reader thread should skip it and keep
        // parsing whatever follows. Regression coverage for a bug where the
        // reader thread unconditionally `return`ed on the first framing
        // error, silently killing the session (surfaced to the main loop only
        // as an untraceable `RecvTimeoutError::Disconnected`) even though
        // well-formed frames followed in the same stream.
        #[derive(Clone)]
        struct SharedSink(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for SharedSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let frame_of = |v: Value| frame(&serde_json::to_vec(&v).expect("ser"));

        // A header block with an unparseable Content-Length value, followed by
        // a valid initialize + disconnect pair.
        let mut input = b"Content-Length: notanumber\r\n\r\n".to_vec();
        input.extend_from_slice(&frame_of(
            json!({ "seq": 1, "type": "request", "command": "initialize", "arguments": {} }),
        ));
        input.extend_from_slice(&frame_of(
            json!({ "seq": 2, "type": "request", "command": "disconnect" }),
        ));

        let out_buf = Arc::new(Mutex::new(Vec::new()));
        run_peer_session_threaded(
            Cursor::new(input),
            SharedSink(out_buf.clone()),
            bridge(),
            Duration::from_millis(5),
        )
        .expect("session ok despite the leading malformed frame");

        let raw = out_buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut framer = ContentLengthFramer::new();
        framer.push(&raw);
        let mut commands = Vec::new();
        while let Ok(Some(body)) = framer.try_next() {
            let v: Value = serde_json::from_slice(&body).expect("json");
            if v.get("type").and_then(Value::as_str) == Some("response")
                && let Some(c) = v.get("command").and_then(Value::as_str)
            {
                commands.push(c.to_string());
            }
        }
        assert!(
            commands.contains(&"initialize".to_string()),
            "the valid frames after the malformed one must still be processed: {commands:?}"
        );
        assert!(commands.contains(&"disconnect".to_string()), "commands: {commands:?}");
    }
}
