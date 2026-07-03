//! [`ExternalDebuggerPeerBackend`] — a [`DebugBackend`] that drives an external
//! debugger engine (ptkdb-first) over the [`crate::peer_protocol`].
//!
//! # Transport (decision D3)
//!
//! Blocking `std::net` sockets with a dedicated reader thread and
//! `std::sync::mpsc` request/response correlation. This keeps the backend
//! decoupled from any tokio runtime and adds no new dependencies. The reader
//! thread:
//! - answers peer→host requests (`peer/hello`, `peer/goodbye`),
//! - routes peer responses to the waiting request via a per-request channel,
//! - translates peer events into [`crate::model::DebugEvent`]s queued for
//!   [`DebugBackend::drain_events`].
//!
//! # Handshake
//!
//! The peer is expected to send `peer/hello` once the connection is up (in both
//! `Listen` and `Connect` modes). The host replies with its own capabilities and
//! a session id. [`DebugBackend::initialize`] blocks until the handshake
//! completes or a timeout elapses.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::capabilities::{ControlMode, DebugBackendCapabilities};
use super::{
    AttachBackendParams, AttachResult, BackendError, BackendResult, ContinueResult, DebugBackend,
    EvaluateContext, EvaluateParams, EvaluateResult, InitializeBackendParams, LaunchBackendParams,
    LaunchResult, SetBackendBreakpointsParams, SetFunctionBreakpointsParams, StackTraceParams,
};
use crate::model::{
    DebugEvent, DebugPosition, DebugScope, DebugSource, DebugStackFrame, DebugVariable, FrameId,
    OutputCategory, ResolvedBreakpoint, StopReason, ThreadId, VariablesRef,
};
use crate::peer_protocol::message::{
    PeerEvent, PeerMessage, PeerRequest, PeerResponse, command, event,
};
use crate::peer_protocol::payloads::{
    self, EvaluateArgs, HelloArgs, HelloResponseBody, ScopesArgs, SetBreakpointsArgs,
    SetFunctionBreakpointsArgs, StackTraceArgs, ThreadArgs, VariablesArgs, WireSource,
    WireSourceBreakpoint,
};
use crate::peer_protocol::{
    HostReportedCapabilities, PROTOCOL_VERSION, PeerFrameDecoder, PeerFrameError,
    PeerReportedCapabilities, encode_message,
};

/// Default time to wait for the peer handshake / a request response.
pub const DEFAULT_PEER_TIMEOUT: Duration = Duration::from_secs(10);

/// How a peer connection is established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalPeerMode {
    /// `perl-dap` listens; the peer connects to it.
    Listen {
        /// Bind host.
        host: String,
        /// Bind port (`0` = ephemeral).
        port: u16,
        /// Optional shared-secret token.
        token: Option<String>,
    },
    /// `perl-dap` connects to an already-running peer.
    Connect {
        /// Peer host.
        host: String,
        /// Peer port.
        port: u16,
        /// Optional shared-secret token.
        token: Option<String>,
    },
    /// `perl-dap` launches the peer process, then listens for it.
    LaunchPeer {
        /// Peer command.
        command: PathBuf,
        /// Peer arguments.
        args: Vec<String>,
        /// Bind host `perl-dap` listens on.
        host: String,
        /// Bind port (`0` = ephemeral).
        port: u16,
    },
}

// ---------------------------------------------------------------------------
// Shared connection state
// ---------------------------------------------------------------------------

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Recover from poisoning rather than propagate a panic (banned in prod).
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Shared {
    write: Mutex<TcpStream>,
    pending: Mutex<HashMap<i64, Sender<PeerResponse>>>,
    events: Mutex<Vec<DebugEvent>>,
    peer_caps: Mutex<Option<PeerReportedCapabilities>>,
    handshake_done: Mutex<bool>,
    /// Set when the handshake is rejected (e.g. protocol-version mismatch), so
    /// `initialize()` returns a clear error instead of an opaque timeout.
    handshake_error: Mutex<Option<String>>,
    handshake_cv: Condvar,
    host_seq: AtomicI64,
    closed: AtomicBool,
    host_caps: HostReportedCapabilities,
    session_id: String,
}

impl Shared {
    fn next_host_seq(&self) -> i64 {
        self.host_seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn write_message(&self, msg: &PeerMessage) -> BackendResult<()> {
        let bytes = encode_message(msg).map_err(|e| BackendError::Protocol(e.to_string()))?;
        let mut guard = lock(&self.write);
        // A write timeout is set on the socket, so a peer that stops draining its
        // receive buffer (flow control, no clean close) bounds this write instead
        // of blocking `write_all` indefinitely while holding the mutex — which
        // would wedge `request()` and `Drop::join()`. On any write failure the
        // connection is dead: mark it closed so subsequent ops fail fast.
        let result = guard
            .write_all(&bytes)
            .and_then(|()| guard.flush())
            .map_err(|e| BackendError::Transport(e.to_string()));
        drop(guard);
        if result.is_err() {
            self.mark_closed();
        }
        result
    }

    fn mark_closed(&self) {
        self.closed.store(true, Ordering::SeqCst);
        // Fail any in-flight requests by dropping their senders.
        lock(&self.pending).clear();
        // Instantly interrupt the reader thread's blocking read (otherwise it
        // waits up to the read-timeout poll) so `Drop::join` returns promptly.
        // Safe here: callers release the write guard before calling mark_closed.
        let _ = lock(&self.write).shutdown(std::net::Shutdown::Both);
        // Wake anyone waiting on the handshake. Take the lock briefly (so a
        // concurrent waiter cannot miss the wakeup between its `closed` check and
        // its wait) but do NOT re-lock inside the same statement — the std Mutex
        // is not reentrant, so `*lock(x) = *lock(x)` would self-deadlock.
        drop(lock(&self.handshake_done));
        self.handshake_cv.notify_all();
    }
}

/// A [`DebugBackend`] over an external peer.
pub struct ExternalDebuggerPeerBackend {
    shared: Arc<Shared>,
    reader: Option<JoinHandle<()>>,
    timeout: Duration,
    control_mode: ControlMode,
}

impl ExternalDebuggerPeerBackend {
    /// Establish a backend from a connected stream and spawn the reader thread.
    fn from_stream(stream: TcpStream, timeout: Duration) -> BackendResult<Self> {
        let write = stream.try_clone().map_err(|e| BackendError::Transport(e.to_string()))?;
        // Periodic read timeout so the reader can observe `closed`.
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| BackendError::Transport(e.to_string()))?;
        // Bounded write timeout so a stalled (not closed) peer cannot block a
        // writer forever while holding the write mutex.
        write
            .set_write_timeout(Some(timeout))
            .map_err(|e| BackendError::Transport(e.to_string()))?;

        let shared = Arc::new(Shared {
            write: Mutex::new(write),
            pending: Mutex::new(HashMap::new()),
            events: Mutex::new(Vec::new()),
            peer_caps: Mutex::new(None),
            handshake_done: Mutex::new(false),
            handshake_error: Mutex::new(None),
            handshake_cv: Condvar::new(),
            host_seq: AtomicI64::new(0),
            closed: AtomicBool::new(false),
            host_caps: HostReportedCapabilities::default(),
            // Deterministic session id derived from the peer address; avoids
            // pulling in a uuid/random dependency.
            session_id: stream
                .peer_addr()
                .map(|a| format!("perl-dap-peer-{a}"))
                .unwrap_or_else(|_| "perl-dap-peer".to_string()),
        });

        let reader_shared = Arc::clone(&shared);
        let reader = std::thread::Builder::new()
            .name("perl-dap-peer-reader".to_string())
            .spawn(move || reader_loop(stream, reader_shared))
            .map_err(|e| BackendError::Transport(e.to_string()))?;

        Ok(Self { shared, reader: Some(reader), timeout, control_mode: ControlMode::Mirror })
    }

    /// Build a backend over an already-connected peer stream.
    ///
    /// Use this when the caller manages the socket rendezvous itself (e.g. it
    /// accepted the peer on its own listener). The peer is still expected to send
    /// `peer/hello` once the stream is up.
    ///
    /// # Errors
    /// Fails if the socket cannot be cloned or configured.
    pub fn from_connected_stream(stream: TcpStream, timeout: Duration) -> BackendResult<Self> {
        Self::from_stream(stream, timeout)
    }

    /// Connect to a running peer (`Connect` mode).
    ///
    /// # Errors
    /// Fails if the TCP connection cannot be established.
    pub fn connect<A: ToSocketAddrs>(addr: A, timeout: Duration) -> BackendResult<Self> {
        // Bound the *entire* connect phase by a single deadline across the whole
        // resolved address list — not `timeout` per address. A hostname with N
        // A-records (or a dual-stack IPv4+IPv6 entry) would otherwise stall for
        // up to `timeout * N`, blowing past the documented within-`timeout`
        // contract and most editors' adapter-startup watchdog. (Plain
        // TcpStream::connect ignores the timeout entirely and can block longer.)
        let addrs = addr.to_socket_addrs().map_err(|e| BackendError::Transport(e.to_string()))?;
        let mut last_err = None;
        let deadline = Instant::now() + timeout;
        for address in addrs {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match TcpStream::connect_timeout(&address, remaining) {
                Ok(stream) => return Self::from_stream(stream, timeout),
                Err(e) => last_err = Some(e),
            }
        }
        Err(BackendError::Transport(
            last_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "address resolved to no socket addresses".to_string()),
        ))
    }

    /// Listen for a peer to connect (`Listen` mode), accepting one client.
    ///
    /// Returns the backend and the actually-bound socket address (useful when
    /// `port` was `0`).
    ///
    /// # Errors
    /// Fails if binding fails or no peer connects before `timeout`.
    pub fn listen(
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> BackendResult<(Self, std::net::SocketAddr)> {
        let listener =
            TcpListener::bind((host, port)).map_err(|e| BackendError::Transport(e.to_string()))?;
        let bound = listener.local_addr().map_err(|e| BackendError::Transport(e.to_string()))?;
        listener.set_nonblocking(true).map_err(|e| BackendError::Transport(e.to_string()))?;

        let deadline = Instant::now() + timeout;
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(BackendError::Timeout(
                            "no peer connected before timeout".to_string(),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(BackendError::Transport(e.to_string())),
            }
        };
        stream.set_nonblocking(false).map_err(|e| BackendError::Transport(e.to_string()))?;
        Ok((Self::from_stream(stream, timeout)?, bound))
    }

    /// Block until the peer handshake completes or the timeout elapses.
    fn await_handshake(&self) -> BackendResult<()> {
        let mut done = lock(&self.shared.handshake_done);
        let deadline = Instant::now() + self.timeout;
        while !*done {
            if let Some(reason) = lock(&self.shared.handshake_error).clone() {
                return Err(BackendError::Protocol(reason));
            }
            if self.shared.closed.load(Ordering::SeqCst) {
                return Err(BackendError::NotConnected);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(BackendError::Timeout("peer handshake".to_string()));
            }
            let (guard, res) = self
                .shared
                .handshake_cv
                .wait_timeout(done, remaining)
                .unwrap_or_else(PoisonError::into_inner);
            done = guard;
            if res.timed_out() && !*done {
                return Err(BackendError::Timeout("peer handshake".to_string()));
            }
        }
        Ok(())
    }

    /// Send a host→peer request and block for its response.
    fn request(&self, command: &str, arguments: Option<Value>) -> BackendResult<PeerResponse> {
        if self.shared.closed.load(Ordering::SeqCst) {
            return Err(BackendError::NotConnected);
        }
        let seq = self.shared.next_host_seq();
        let (tx, rx): (Sender<PeerResponse>, Receiver<PeerResponse>) = channel();
        lock(&self.shared.pending).insert(seq, tx);

        let msg =
            PeerMessage::Request(PeerRequest { seq, command: command.to_string(), arguments });
        if let Err(e) = self.shared.write_message(&msg) {
            lock(&self.shared.pending).remove(&seq);
            return Err(e);
        }

        match rx.recv_timeout(self.timeout) {
            Ok(resp) => {
                if resp.success {
                    Ok(resp)
                } else {
                    Err(BackendError::Engine(
                        resp.message.unwrap_or_else(|| format!("{command} failed")),
                    ))
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                lock(&self.shared.pending).remove(&seq);
                Err(BackendError::Timeout(command.to_string()))
            }
            Err(RecvTimeoutError::Disconnected) => Err(BackendError::NotConnected),
        }
    }

    fn negotiated_caps(&self) -> DebugBackendCapabilities {
        // `.as_ref()` borrows the inner Option so we do not depend on the
        // `Copy` derive to read it out of the guard.
        lock(&self.shared.peer_caps)
            .as_ref()
            .map(|c| c.to_backend_capabilities())
            .unwrap_or_else(DebugBackendCapabilities::none)
    }

    /// Guard a control command against negotiated capabilities so the backend
    /// never issues something the peer said it cannot do (mirror-mode honesty).
    fn require(&self, ok: bool, what: &str) -> BackendResult<()> {
        if ok { Ok(()) } else { Err(BackendError::Unsupported(what.to_string())) }
    }
}

impl Drop for ExternalDebuggerPeerBackend {
    fn drop(&mut self) {
        self.shared.mark_closed();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
    }
}

impl DebugBackend for ExternalDebuggerPeerBackend {
    fn name(&self) -> &str {
        "external-peer"
    }

    fn capabilities(&self) -> DebugBackendCapabilities {
        self.negotiated_caps()
    }

    fn initialize(&mut self, _params: InitializeBackendParams) -> BackendResult<()> {
        self.await_handshake()?;
        self.control_mode = self.negotiated_caps().control_mode;
        Ok(())
    }

    fn launch(&mut self, _params: LaunchBackendParams) -> BackendResult<LaunchResult> {
        // In peer mode the peer owns the debuggee lifecycle; launch is a no-op
        // acknowledgement once the handshake is up. Explicit peer launch is
        // future work (mirror MVP — see DECISIONS DF2/DF4).
        self.await_handshake()?;
        Ok(LaunchResult { success: true })
    }

    fn attach(&mut self, _params: AttachBackendParams) -> BackendResult<AttachResult> {
        self.await_handshake()?;
        Ok(AttachResult { success: true })
    }

    fn set_breakpoints(
        &mut self,
        params: SetBackendBreakpointsParams,
    ) -> BackendResult<Vec<ResolvedBreakpoint>> {
        let caps = self.negotiated_caps();
        self.require(caps.source_breakpoints, "peer does not accept breakpoints")?;

        let source = wire_source(&params.source);
        let breakpoints = params
            .breakpoints
            .into_iter()
            .map(|b| WireSourceBreakpoint {
                line: b.line,
                column: b.column,
                condition: b.condition,
                hit_condition: b.hit_condition,
                log_message: b.log_message,
            })
            .collect();
        let args = SetBreakpointsArgs { source: source.clone(), breakpoints };
        let resp = self.request(command::SET_BREAKPOINTS, Some(to_value(&args)?))?;
        let body: payloads::SetBreakpointsResponseBody = from_body(resp.body)?;
        Ok(body.breakpoints.into_iter().map(|bp| resolved_from_wire(bp, &params.source)).collect())
    }

    fn set_function_breakpoints(
        &mut self,
        params: SetFunctionBreakpointsParams,
    ) -> BackendResult<Vec<ResolvedBreakpoint>> {
        let caps = self.negotiated_caps();
        self.require(caps.function_breakpoints, "peer does not accept function breakpoints")?;
        let args = SetFunctionBreakpointsArgs {
            names: params.breakpoints.into_iter().map(|b| b.name).collect(),
        };
        let resp = self.request(command::SET_FUNCTION_BREAKPOINTS, Some(to_value(&args)?))?;
        let body: payloads::SetBreakpointsResponseBody = from_body(resp.body)?;
        let stub = DebugSource { path: PathBuf::new(), name: None, source_reference: None };
        Ok(body.breakpoints.into_iter().map(|bp| resolved_from_wire(bp, &stub)).collect())
    }

    fn continue_thread(&mut self, thread_id: ThreadId) -> BackendResult<ContinueResult> {
        // Gate on the dedicated resume capability, not `stepping`: a peer can
        // resume a stopped program without supporting single-step.
        self.require(self.negotiated_caps().continue_execution, "peer cannot continue")?;
        let args = ThreadArgs { thread_id: thread_id.0 };
        self.request(command::CONTINUE, Some(to_value(&args)?))?;
        Ok(ContinueResult { all_threads_continued: true })
    }

    fn next(&mut self, thread_id: ThreadId) -> BackendResult<()> {
        self.require(self.negotiated_caps().stepping, "peer cannot step")?;
        self.request(command::NEXT, Some(to_value(&ThreadArgs { thread_id: thread_id.0 })?))?;
        Ok(())
    }

    fn step_in(&mut self, thread_id: ThreadId) -> BackendResult<()> {
        self.require(self.negotiated_caps().stepping, "peer cannot step")?;
        self.request(command::STEP_IN, Some(to_value(&ThreadArgs { thread_id: thread_id.0 })?))?;
        Ok(())
    }

    fn step_out(&mut self, thread_id: ThreadId) -> BackendResult<()> {
        self.require(self.negotiated_caps().stepping, "peer cannot step")?;
        self.request(command::STEP_OUT, Some(to_value(&ThreadArgs { thread_id: thread_id.0 })?))?;
        Ok(())
    }

    fn pause(&mut self, thread_id: ThreadId) -> BackendResult<()> {
        self.require(self.negotiated_caps().pause, "peer cannot pause")?;
        self.request(command::PAUSE, Some(to_value(&ThreadArgs { thread_id: thread_id.0 })?))?;
        Ok(())
    }

    fn stack_trace(&mut self, params: StackTraceParams) -> BackendResult<Vec<DebugStackFrame>> {
        self.require(self.negotiated_caps().stack_trace, "peer has no stack trace")?;
        let args = StackTraceArgs {
            thread_id: params.thread_id.0,
            start_frame: params.start_frame,
            levels: params.levels,
        };
        let resp = self.request(command::STACK_TRACE, Some(to_value(&args)?))?;
        let body: payloads::StackTraceResponseBody = from_body(resp.body)?;
        Ok(body
            .stack_frames
            .into_iter()
            .map(|f| DebugStackFrame {
                id: f.id,
                name: f.name,
                source: model_source(&f.source),
                line: f.line,
                column: f.column,
            })
            .collect())
    }

    fn scopes(&mut self, frame_id: FrameId) -> BackendResult<Vec<DebugScope>> {
        self.require(self.negotiated_caps().scopes, "peer has no scopes")?;
        let args = ScopesArgs { frame_id: frame_id.0 };
        let resp = self.request(command::SCOPES, Some(to_value(&args)?))?;
        let body: payloads::ScopesResponseBody = from_body(resp.body)?;
        Ok(body
            .scopes
            .into_iter()
            .map(|s| DebugScope {
                name: s.name,
                variables_reference: VariablesRef(s.variables_reference),
                expensive: s.expensive,
            })
            .collect())
    }

    fn variables(&mut self, variables_ref: VariablesRef) -> BackendResult<Vec<DebugVariable>> {
        self.require(self.negotiated_caps().variables, "peer has no variables")?;
        let args = VariablesArgs { variables_reference: variables_ref.0 };
        let resp = self.request(command::VARIABLES, Some(to_value(&args)?))?;
        let body: payloads::VariablesResponseBody = from_body(resp.body)?;
        Ok(body
            .variables
            .into_iter()
            .map(|v| DebugVariable {
                name: v.name,
                value: v.value,
                type_name: v.type_name,
                variables_reference: Some(VariablesRef(v.variables_reference)),
                indexed_variables: v.indexed_variables,
                named_variables: v.named_variables,
            })
            .collect())
    }

    fn evaluate(&mut self, params: EvaluateParams) -> BackendResult<EvaluateResult> {
        self.require(self.negotiated_caps().evaluate, "peer cannot evaluate")?;
        let args = EvaluateArgs {
            expression: params.expression,
            frame_id: params.frame_id.map(|f| f.0),
            context: Some(context_str(&params.context).to_string()),
        };
        let resp = self.request(command::EVALUATE, Some(to_value(&args)?))?;
        let body: payloads::EvaluateResponseBody = from_body(resp.body)?;
        Ok(EvaluateResult {
            result: body.result,
            type_name: body.type_name,
            variables_reference: Some(VariablesRef(body.variables_reference)),
        })
    }

    fn drain_events(&mut self) -> Vec<DebugEvent> {
        std::mem::take(&mut *lock(&self.shared.events))
    }

    fn disconnect(&mut self, terminate_debuggee: bool) -> BackendResult<()> {
        // Best-effort goodbye; ignore errors since we are tearing down. Thread
        // the editor's terminate intent to the peer in the goodbye arguments so
        // a mirror peer can distinguish "kill the debuggee" from "just close the
        // protocol session" (peers that ignore the field still disconnect
        // cleanly). Mirrors `native_perldb`, which forwards `terminateDebuggee`.
        let seq = self.shared.next_host_seq();
        let _ = self.shared.write_message(&PeerMessage::Request(PeerRequest {
            seq,
            command: command::GOODBYE.to_string(),
            arguments: Some(serde_json::json!({ "terminateDebuggee": terminate_debuggee })),
        }));
        self.shared.mark_closed();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reader thread
// ---------------------------------------------------------------------------

fn reader_loop(mut stream: TcpStream, shared: Arc<Shared>) {
    let mut decoder = PeerFrameDecoder::new();
    let mut buf = [0u8; 8192];
    loop {
        if shared.closed.load(Ordering::SeqCst) {
            break;
        }
        match stream.read(&mut buf) {
            Ok(0) => {
                // Peer closed the connection.
                shared.mark_closed();
                break;
            }
            Ok(n) => {
                decoder.push(&buf[..n]);
                loop {
                    match decoder.try_next() {
                        Ok(Some(msg)) => handle_incoming(&shared, msg),
                        Ok(None) => break,
                        Err(PeerFrameError::Framing(_)) => {
                            // Genuinely broken wire format (unparseable header, bad
                            // Content-Length, missing CRLF): the peer is
                            // misbehaving at the framing layer — defensive shutdown.
                            shared.mark_closed();
                            return;
                        }
                        Err(PeerFrameError::Json(e)) => {
                            // The framer succeeded but the body did not deserialize
                            // as a PeerMessage (e.g. an unknown `type` from a future
                            // peer-protocol extension). Log and keep parsing rather
                            // than tearing the session down — same recoverable
                            // posture as the DAP-side drivers in this crate.
                            tracing::warn!(
                                error = %e,
                                "peer reader: dropping unrecognized peer message body"
                            );
                        }
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // Periodic timeout so we can re-check `closed`.
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                // Signal-interrupted read: retry, matching the stdio driver in
                // `run_peer_session_threaded`. Without this, a stray SIGCHLD/
                // SIGPIPE from elsewhere in the process (any handler installed
                // without SA_RESTART) could take the peer session down even though
                // the wire itself is fine.
                continue;
            }
            Err(_e) => {
                shared.mark_closed();
                break;
            }
        }
    }
}

fn handle_incoming(shared: &Arc<Shared>, msg: PeerMessage) {
    match msg {
        PeerMessage::Response(resp) => {
            if let Some(tx) = lock(&shared.pending).remove(&resp.request_seq) {
                let _ = tx.send(resp);
            }
        }
        PeerMessage::Event(ev) => {
            if let Some(model_ev) = translate_event(&ev) {
                lock(&shared.events).push(model_ev);
            }
        }
        PeerMessage::Request(req) => handle_peer_request(shared, req),
    }
}

fn handle_peer_request(shared: &Arc<Shared>, req: PeerRequest) {
    match req.command.as_str() {
        command::HELLO => {
            // Parse the hello; reject an unfamiliar protocol version rather than
            // guess (per peer_protocol::PROTOCOL_VERSION's contract).
            let hello =
                req.arguments.clone().and_then(|a| serde_json::from_value::<HelloArgs>(a).ok());
            let rejection = match &hello {
                None => Some("malformed peer/hello arguments".to_string()),
                Some(h) if h.protocol_version != PROTOCOL_VERSION => Some(format!(
                    "unsupported peer protocol version {:?}; host speaks {:?}",
                    h.protocol_version, PROTOCOL_VERSION
                )),
                Some(_) => None,
            };

            if let Some(reason) = rejection {
                let resp = PeerMessage::Response(PeerResponse {
                    seq: shared.next_host_seq(),
                    request_seq: req.seq,
                    success: false,
                    command: command::HELLO.to_string(),
                    message: Some(reason.clone()),
                    body: None,
                });
                let _ = shared.write_message(&resp);
                // Signal a clear rejection to `initialize()` and wake it.
                *lock(&shared.handshake_error) = Some(reason);
                shared.handshake_cv.notify_all();
                return;
            }

            // A second `peer/hello` during a live session must NOT silently
            // rewrite the already-negotiated capabilities — they are part of the
            // immutable session contract that `negotiated_caps()` reads per
            // request. Reject the replay and leave `peer_caps` untouched.
            if *lock(&shared.handshake_done) {
                let resp = PeerMessage::Response(PeerResponse {
                    seq: shared.next_host_seq(),
                    request_seq: req.seq,
                    success: false,
                    command: command::HELLO.to_string(),
                    message: Some("already handshaken".to_string()),
                    body: None,
                });
                let _ = shared.write_message(&resp);
                return;
            }
            if let Some(h) = hello {
                *lock(&shared.peer_caps) = Some(h.capabilities);
            }
            let body = HelloResponseBody {
                protocol_version: PROTOCOL_VERSION.to_string(),
                session_id: shared.session_id.clone(),
                capabilities: shared.host_caps,
            };
            let resp = PeerMessage::Response(PeerResponse {
                seq: shared.next_host_seq(),
                request_seq: req.seq,
                success: true,
                command: command::HELLO.to_string(),
                message: None,
                body: serde_json::to_value(body).ok(),
            });
            let _ = shared.write_message(&resp);
            // `write_message` calls `mark_closed()` on any write failure. If the
            // hello response never reached the peer (disconnect / SIGPIPE between
            // building and flushing), do NOT report handshake success against a
            // dead stream — surface an error so `initialize()` fails cleanly
            // instead of the first real request hitting `NotConnected`.
            if shared.closed.load(Ordering::SeqCst) {
                *lock(&shared.handshake_error) = Some("peer closed during handshake".to_string());
                shared.handshake_cv.notify_all();
                return;
            }
            *lock(&shared.handshake_done) = true;
            shared.handshake_cv.notify_all();
        }
        command::GOODBYE => {
            let resp = PeerMessage::Response(PeerResponse {
                seq: shared.next_host_seq(),
                request_seq: req.seq,
                success: true,
                command: command::GOODBYE.to_string(),
                message: None,
                body: None,
            });
            let _ = shared.write_message(&resp);
            shared.mark_closed();
        }
        other => {
            // Unknown peer→host request: reply with a failure rather than hang.
            let resp = PeerMessage::Response(PeerResponse {
                seq: shared.next_host_seq(),
                request_seq: req.seq,
                success: false,
                command: other.to_string(),
                message: Some(format!("unsupported host command: {other}")),
                body: None,
            });
            let _ = shared.write_message(&resp);
        }
    }
}

// ---------------------------------------------------------------------------
// Translation helpers
// ---------------------------------------------------------------------------

fn translate_event(ev: &PeerEvent) -> Option<DebugEvent> {
    match ev.event.as_str() {
        event::INITIALIZED => Some(DebugEvent::Initialized),
        event::STOPPED => {
            let body: payloads::StoppedEventBody = serde_json::from_value(ev.body.clone()?).ok()?;
            // A position needs BOTH a source and a line. If the peer reports a
            // stop with no determinable line (e.g. inside an `eval` string),
            // surface `None` rather than fabricating line 0 — `DebugPosition.line`
            // is documented 1-based, so 0 would be an out-of-range sentinel.
            let position = match (body.source, body.line) {
                (Some(s), Some(line)) => {
                    Some(DebugPosition { source: model_source(&s), line, column: body.column })
                }
                _ => None,
            };
            Some(DebugEvent::Stopped {
                reason: stop_reason(&body.reason),
                thread_id: ThreadId(body.thread_id),
                position,
            })
        }
        event::CONTINUED => {
            let body: payloads::ContinuedEventBody =
                serde_json::from_value(ev.body.clone()?).ok()?;
            Some(DebugEvent::Continued { thread_id: ThreadId(body.thread_id) })
        }
        event::OUTPUT => {
            let body: payloads::OutputEventBody = serde_json::from_value(ev.body.clone()?).ok()?;
            Some(DebugEvent::Output {
                category: output_category(&body.category),
                output: body.output,
            })
        }
        event::TERMINATED => {
            let body: payloads::TerminatedEventBody =
                ev.body.clone().and_then(|b| serde_json::from_value(b).ok()).unwrap_or_default();
            Some(DebugEvent::Terminated { exit_code: body.exit_code })
        }
        event::SOURCE_FACTS => {
            let body: payloads::SourceFactsEventBody =
                serde_json::from_value(ev.body.clone()?).ok()?;
            let facts = crate::model::SourceDebugFacts {
                breakable_line_candidates: body.breakable_lines,
                subroutines: body
                    .subroutines
                    .into_iter()
                    .map(|s| crate::model::DebugFunctionSymbol {
                        name: s.name,
                        source: model_source(&s.source),
                        start_line: s.start_line,
                        end_line: s.end_line,
                    })
                    .collect(),
            };
            Some(DebugEvent::SourceFacts { source: model_source(&body.source), facts })
        }
        event::BREAKPOINTS_CHANGED => {
            let body: payloads::BreakpointsChangedEventBody =
                serde_json::from_value(ev.body.clone()?).ok()?;
            let stub = DebugSource { path: PathBuf::new(), name: None, source_reference: None };
            Some(DebugEvent::BreakpointsChanged {
                breakpoints: body
                    .breakpoints
                    .into_iter()
                    .map(|bp| resolved_from_wire(bp, &stub))
                    .collect(),
            })
        }
        _ => None,
    }
}

fn stop_reason(raw: &str) -> StopReason {
    match raw {
        "entry" => StopReason::Entry,
        "step" => StopReason::Step,
        "breakpoint" => StopReason::Breakpoint,
        "functionBreakpoint" | "function breakpoint" => StopReason::FunctionBreakpoint,
        "dataBreakpoint" | "data breakpoint" => StopReason::DataBreakpoint,
        "exception" => StopReason::Exception,
        "pause" => StopReason::Pause,
        other => StopReason::Unknown(other.to_string()),
    }
}

fn output_category(raw: &str) -> OutputCategory {
    match raw {
        "stderr" => OutputCategory::Stderr,
        "console" => OutputCategory::Console,
        _ => OutputCategory::Stdout,
    }
}

fn context_str(ctx: &EvaluateContext) -> &str {
    match ctx {
        EvaluateContext::Watch => "watch",
        EvaluateContext::Repl => "repl",
        EvaluateContext::Hover => "hover",
        EvaluateContext::Variables => "variables",
        EvaluateContext::Other(s) => s.as_str(),
    }
}

fn wire_source(s: &DebugSource) -> WireSource {
    WireSource {
        path: s.path.to_string_lossy().into_owned(),
        name: s.name.clone(),
        source_reference: s.source_reference,
    }
}

fn model_source(s: &WireSource) -> DebugSource {
    DebugSource {
        path: PathBuf::from(&s.path),
        name: s.name.clone(),
        source_reference: s.source_reference,
    }
}

fn resolved_from_wire(
    bp: payloads::WireResolvedBreakpoint,
    source: &DebugSource,
) -> ResolvedBreakpoint {
    ResolvedBreakpoint {
        id: bp.id,
        verified: bp.verified,
        actual_position: DebugPosition { source: source.clone(), line: bp.line, column: bp.column },
        message: bp.message,
    }
}

fn to_value<T: serde::Serialize>(v: &T) -> BackendResult<Value> {
    serde_json::to_value(v).map_err(|e| BackendError::Protocol(e.to_string()))
}

fn from_body<T: serde::de::DeserializeOwned>(body: Option<Value>) -> BackendResult<T> {
    let body = body.ok_or_else(|| BackendError::Protocol("response had no body".to_string()))?;
    serde_json::from_value(body).map_err(|e| BackendError::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer_protocol::payloads::{
        HelloArgs, SetBreakpointsResponseBody, WireResolvedBreakpoint,
    };

    /// Spawn a minimal fake peer that connects to `addr`, sends hello, then runs
    /// `handler` for each host request it receives.
    fn spawn_fake_peer(
        addr: std::net::SocketAddr,
        caps: PeerReportedCapabilities,
        handler: impl Fn(&PeerRequest) -> Option<PeerResponse> + Send + 'static,
    ) -> JoinHandle<()> {
        spawn_fake_peer_version(addr, PROTOCOL_VERSION.to_string(), caps, handler)
    }

    fn spawn_fake_peer_version(
        addr: std::net::SocketAddr,
        protocol_version: String,
        caps: PeerReportedCapabilities,
        handler: impl Fn(&PeerRequest) -> Option<PeerResponse> + Send + 'static,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let stream = match TcpStream::connect(addr) {
                Ok(s) => s,
                Err(_) => return,
            };
            let mut write = stream.try_clone().expect("clone");
            let mut read = stream;
            let mut seq = 100;

            // Send hello.
            let hello = PeerMessage::Request(PeerRequest {
                seq,
                command: command::HELLO.to_string(),
                arguments: serde_json::to_value(HelloArgs {
                    peer: "FakePtkdb".to_string(),
                    peer_version: Some("0.1".to_string()),
                    protocol_version,
                    capabilities: caps,
                })
                .ok(),
            });
            let _ = write.write_all(&encode_message(&hello).expect("enc"));

            let mut decoder = PeerFrameDecoder::new();
            let mut buf = [0u8; 4096];
            loop {
                match read.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        decoder.push(&buf[..n]);
                        while let Ok(Some(msg)) = decoder.try_next() {
                            if let PeerMessage::Request(req) = msg {
                                if req.command == command::HELLO {
                                    continue; // host's hello response arrives as Response
                                }
                                if let Some(mut resp) = handler(&req) {
                                    seq += 1;
                                    resp.seq = seq;
                                    resp.request_seq = req.seq;
                                    let _ = write.write_all(
                                        &encode_message(&PeerMessage::Response(resp)).expect("enc"),
                                    );
                                    if req.command == command::GOODBYE {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    }

    fn ok_resp(command: &str, body: Option<Value>) -> PeerResponse {
        PeerResponse {
            seq: 0,
            request_seq: 0,
            success: true,
            command: command.to_string(),
            message: None,
            body,
        }
    }

    #[test]
    fn handshake_records_peer_capabilities() {
        let (listener, addr) = bind_ephemeral();
        let caps = PeerReportedCapabilities {
            can_continue: true,
            can_step: true,
            can_evaluate: true,
            can_set_breakpoints: true,
            can_condition_breakpoints: true,
            can_list_stack: true,
            can_list_variables: true,
            ..Default::default()
        };
        let peer = spawn_fake_peer(addr, caps, |_req| None);
        let mut backend = accept_backend(listener);
        backend.initialize(InitializeBackendParams::default()).expect("handshake");
        let negotiated = backend.capabilities();
        assert!(negotiated.evaluate);
        assert!(negotiated.stepping);
        assert!(negotiated.conditional_breakpoints);
        assert!(!negotiated.logpoints, "v1 peer never claims logpoints");
        drop(backend);
        let _ = peer.join();
    }

    #[test]
    fn set_breakpoints_round_trips_through_peer() {
        let (listener, addr) = bind_ephemeral();
        let caps = PeerReportedCapabilities { can_set_breakpoints: true, ..Default::default() };
        let peer = spawn_fake_peer(addr, caps, |req| {
            if req.command == command::SET_BREAKPOINTS {
                let body = SetBreakpointsResponseBody {
                    breakpoints: vec![WireResolvedBreakpoint {
                        id: 7,
                        verified: true,
                        line: 42,
                        column: None,
                        message: None,
                    }],
                };
                Some(ok_resp(command::SET_BREAKPOINTS, serde_json::to_value(body).ok()))
            } else {
                None
            }
        });
        let mut backend = accept_backend(listener);
        backend.initialize(InitializeBackendParams::default()).expect("handshake");
        let src = DebugSource::from_path("/work/script.pl");
        let out = backend
            .set_breakpoints(SetBackendBreakpointsParams {
                source: src.clone(),
                breakpoints: vec![crate::model::DebugBreakpoint {
                    id: None,
                    source: src,
                    line: 42,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                }],
            })
            .expect("set breakpoints");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 7);
        assert!(out[0].verified);
        assert_eq!(out[0].actual_position.line, 42);
        drop(backend);
        let _ = peer.join();
    }

    #[test]
    fn mirror_mode_rejects_uncapable_control() {
        let (listener, addr) = bind_ephemeral();
        // Peer that only reports stops: no step capability.
        let peer = spawn_fake_peer(addr, PeerReportedCapabilities::default(), |_req| None);
        let mut backend = accept_backend(listener);
        backend.initialize(InitializeBackendParams::default()).expect("handshake");
        let err = backend.continue_thread(ThreadId(1)).expect_err("should reject");
        assert!(matches!(err, BackendError::Unsupported(_)));
        drop(backend);
        let _ = peer.join();
    }

    #[test]
    fn pause_requires_dedicated_can_pause_capability() {
        // A peer that can step but did NOT advertise async pause must not be
        // sent a pause it never negotiated (mirror-mode honesty; capability #3).
        let (listener, addr) = bind_ephemeral();
        let caps =
            PeerReportedCapabilities { can_step: true, can_pause: false, ..Default::default() };
        let peer = spawn_fake_peer(addr, caps, |_req| None);
        let mut backend = accept_backend(listener);
        backend.initialize(InitializeBackendParams::default()).expect("handshake");
        // Stepping is allowed...
        assert!(backend.capabilities().stepping);
        // ...but pause is not, because can_pause was false.
        assert!(!backend.capabilities().pause);
        let err = backend.pause(ThreadId(1)).expect_err("pause not negotiated");
        assert!(matches!(err, BackendError::Unsupported(_)));
        drop(backend);
        let _ = peer.join();
    }

    #[test]
    fn protocol_version_mismatch_rejects_handshake() {
        // A peer speaking an incompatible version must be rejected with a clear
        // error, not silently accepted (capability #2).
        let (listener, addr) = bind_ephemeral();
        let peer = spawn_fake_peer_version(
            addr,
            "perl-debug-peer-v99".to_string(),
            PeerReportedCapabilities::default(),
            |_req| None,
        );
        let mut backend = accept_backend_with_timeout(listener, Duration::from_secs(2));
        let err = backend
            .initialize(InitializeBackendParams::default())
            .expect_err("mismatched version must be rejected");
        assert!(
            matches!(err, BackendError::Protocol(_)),
            "expected a clear protocol rejection, got {err:?}"
        );
        drop(backend);
        let _ = peer.join();
    }

    #[test]
    fn request_times_out_when_peer_never_answers() {
        let (listener, addr) = bind_ephemeral();
        let caps = PeerReportedCapabilities { can_set_breakpoints: true, ..Default::default() };
        // Peer completes handshake but never answers setBreakpoints.
        let peer = spawn_fake_peer(addr, caps, |_req| None);
        let mut backend = accept_backend_with_timeout(listener, Duration::from_millis(300));
        backend.initialize(InitializeBackendParams::default()).expect("handshake");
        let src = DebugSource::from_path("/x.pl");
        let err = backend
            .set_breakpoints(SetBackendBreakpointsParams {
                source: src.clone(),
                breakpoints: vec![crate::model::DebugBreakpoint {
                    id: None,
                    source: src,
                    line: 1,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                }],
            })
            .expect_err("should time out");
        assert!(matches!(err, BackendError::Timeout(_)));
        drop(backend);
        let _ = peer.join();
    }

    // --- test rendezvous helpers (host listens, fake peer connects) ---

    fn bind_ephemeral() -> (TcpListener, std::net::SocketAddr) {
        let l = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let a = l.local_addr().expect("addr");
        (l, a)
    }

    fn accept_backend(listener: TcpListener) -> ExternalDebuggerPeerBackend {
        accept_backend_with_timeout(listener, DEFAULT_PEER_TIMEOUT)
    }

    fn accept_backend_with_timeout(
        listener: TcpListener,
        timeout: Duration,
    ) -> ExternalDebuggerPeerBackend {
        let (stream, _) = listener.accept().expect("accept");
        ExternalDebuggerPeerBackend::from_stream(stream, timeout).expect("backend")
    }
}
