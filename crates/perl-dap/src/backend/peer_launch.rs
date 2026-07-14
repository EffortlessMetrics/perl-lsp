//! Mirror-mode **launch wiring** for an external debugger peer (ptkdb-first).
//!
//! This is the live-wiring layer that sits above the [`super::external_peer`]
//! backend and the [`super::peer_bridge`] frontend. Where [`super::peer_bridge`]
//! drives an *already-connected* peer, this module handles the earlier
//! **launch** lifecycle for a `mode: "listen"` mirror session:
//!
//! 1. parse the DAP launch config
//!    (`{"debuggerBackend":"external","externalDebugger":{…}}`),
//! 2. allocate a loopback listener (`port: 0` ⇒ an OS-assigned ephemeral port)
//!    and mint a session token,
//! 3. expose the env-var contract a (future) peer process reads to find us
//!    (`PERL_DAP_PEER` / `PERL_DAP_PEER_TOKEN` / `PERL_DAP_PEER_MODE`), and
//! 4. serve DAP to the editor through a [`MirrorPeerBridge`] that answers
//!    `initialize` from a **static conservative capability profile** before any
//!    peer is connected, **queues** breakpoints that arrive before the peer's
//!    handshake, **flushes** them once the peer says hello, mirrors the peer's
//!    `stopped`/`output`/`terminated` into DAP events, and **rejects
//!    editor-initiated control** (`continue`/`step`) because in mirror mode the
//!    peer's own UI owns execution.
//!
//! This is a **parallel** path: it does not rehome the native
//! [`crate::debug_adapter::DebugAdapter`] dispatch funnel. End-to-end
//! editor↔real-`Devel::ptkdb` sessions remain deferred; the seam is proven with
//! a fake peer.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

use super::capabilities::ControlMode;
use super::external_peer::{DEFAULT_PEER_TIMEOUT, ExternalDebuggerPeerBackend};
use super::{
    BackendError, DebugBackend, EvaluateContext, EvaluateParams, InitializeBackendParams,
    SetBackendBreakpointsParams, SetFunctionBreakpointsParams, StackTraceParams,
};
use crate::breakpoint_oracle::{AstBreakpointOracle, BreakpointOracle};
use crate::debug_adapter::DapMessage;
use crate::model::{
    DebugBreakpoint, DebugEvent, DebugFunctionBreakpoint, DebugSource, FrameId, StopReason,
    ThreadId, VariablesRef,
};

/// Environment variable naming the `HOST:PORT` a peer connects back to.
pub const ENV_PEER_ADDR: &str = "PERL_DAP_PEER";
/// Environment variable carrying the per-session shared-secret token.
pub const ENV_PEER_TOKEN: &str = "PERL_DAP_PEER_TOKEN";
/// Environment variable naming the control mode (`mirror` for this PR).
pub const ENV_PEER_MODE: &str = "PERL_DAP_PEER_MODE";

// ---------------------------------------------------------------------------
// Launch configuration
// ---------------------------------------------------------------------------

/// Which external debugger engine a launch config cooperates with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalDebuggerKind {
    /// `Devel::ptkdb` — the first partner engine.
    #[default]
    Ptkdb,
    /// Any other peer that speaks the Perl Debugger Peer Protocol.
    Custom,
}

/// How `perl-dap` rendezvous with the peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PeerRendezvousMode {
    /// `perl-dap` connects out to a peer already listening (wired in #3321).
    #[default]
    Connect,
    /// `perl-dap` listens; the peer connects back (this PR's mirror wiring).
    Listen,
    /// `perl-dap` launches the peer itself (deferred).
    LaunchPeer,
}

/// The `externalDebugger` block of a DAP launch config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalPeerLaunchConfig {
    /// The engine to cooperate with.
    pub kind: ExternalDebuggerKind,
    /// The rendezvous mode.
    pub mode: PeerRendezvousMode,
    /// Who owns execution control.
    pub control: ControlMode,
    /// Host to bind (listen) or connect to.
    pub host: String,
    /// Port (`0` ⇒ allocate an ephemeral loopback port in listen mode).
    pub port: u16,
}

impl Default for ExternalPeerLaunchConfig {
    fn default() -> Self {
        Self {
            kind: ExternalDebuggerKind::default(),
            mode: PeerRendezvousMode::default(),
            control: ControlMode::default(),
            host: "127.0.0.1".to_string(),
            port: 0,
        }
    }
}

impl ExternalPeerLaunchConfig {
    /// Parse an [`ExternalPeerLaunchConfig`] from DAP `launch` request arguments.
    ///
    /// Returns `None` when the config does not select the external backend
    /// (`debuggerBackend != "external"`) — the caller then runs the native
    /// adapter. Unknown/missing sub-fields fall back to conservative defaults
    /// (`kind: ptkdb`, `mode: connect`, `control: mirror`, `host: 127.0.0.1`,
    /// `port: 0`) rather than failing the launch.
    #[must_use]
    pub fn from_launch_arguments(args: &Value) -> Option<Self> {
        if args.get("debuggerBackend").and_then(Value::as_str) != Some("external") {
            return None;
        }
        let ext = args.get("externalDebugger");
        let mut cfg = Self::default();
        let Some(ext) = ext else {
            return Some(cfg);
        };
        if let Some(kind) = ext.get("kind").and_then(Value::as_str) {
            cfg.kind = match kind {
                "custom" => ExternalDebuggerKind::Custom,
                _ => ExternalDebuggerKind::Ptkdb,
            };
        }
        if let Some(mode) = ext.get("mode").and_then(Value::as_str) {
            cfg.mode = match mode {
                "listen" => PeerRendezvousMode::Listen,
                "launchPeer" => PeerRendezvousMode::LaunchPeer,
                _ => PeerRendezvousMode::Connect,
            };
        }
        if let Some(control) = ext.get("control").and_then(Value::as_str) {
            cfg.control = match control {
                "cooperative" => ControlMode::Cooperative,
                "dapControlled" => ControlMode::DapControlled,
                _ => ControlMode::Mirror,
            };
        }
        if let Some(host) = ext.get("host").and_then(Value::as_str) {
            if !host.trim().is_empty() {
                cfg.host = host.trim().to_string();
            }
        }
        if let Some(port) = ext.get("port").and_then(Value::as_u64) {
            // Ports above u16 range are meaningless; clamp to 0 (allocate).
            cfg.port = u16::try_from(port).unwrap_or(0);
        }
        Some(cfg)
    }
}

/// The wire string for a control mode, as exposed in `PERL_DAP_PEER_MODE`.
#[must_use]
pub fn control_mode_env_str(mode: ControlMode) -> &'static str {
    match mode {
        ControlMode::Mirror => "mirror",
        ControlMode::Cooperative => "cooperative",
        ControlMode::DapControlled => "dapControlled",
    }
}

// ---------------------------------------------------------------------------
// Listen endpoint (bind + token + env contract)
// ---------------------------------------------------------------------------

/// A bound loopback listener a peer connects back to, plus the session token
/// and control mode exposed to the (future) peer process via env vars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerListenEndpoint {
    /// The actually-bound address (with the OS-assigned port when `port` was 0).
    pub addr: SocketAddr,
    /// Per-session shared-secret token.
    pub token: String,
    /// Control mode for the session.
    pub control: ControlMode,
}

impl PeerListenEndpoint {
    /// Bind a loopback listener for the peer to connect back to, minting a
    /// session token. A `port` of `0` yields an OS-assigned ephemeral port; the
    /// resolved address is recorded on the returned endpoint.
    ///
    /// The host **must** resolve to loopback only. A mirror session relays the
    /// debuggee's output/stack/variables, and the `PERL_DAP_PEER` env contract is
    /// loopback-only, so binding a routable interface would expose the debug
    /// session to the network. Loopback bind and the per-session token are
    /// layered controls: the token authenticates the peer's handshake, and the
    /// loopback bind keeps the port off the network. Any host that resolves to a
    /// non-loopback address is refused rather than silently exposed.
    ///
    /// # Errors
    /// Fails if the host resolves to a non-loopback address, or the listener
    /// cannot be bound.
    pub fn bind(
        host: &str,
        port: u16,
        control: ControlMode,
    ) -> std::io::Result<(TcpListener, Self)> {
        // Resolve first and refuse any non-loopback target *before* opening a
        // socket, so a routable host never even briefly binds an exposed port.
        let resolved: Vec<SocketAddr> = (host, port).to_socket_addrs()?.collect();
        if resolved.is_empty() || resolved.iter().any(|a| !a.ip().is_loopback()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "external-peer listen host {host:?} must be loopback \
                     (127.0.0.1 / ::1 / localhost); refusing to bind a non-loopback \
                     interface that would expose the mirror debug session"
                ),
            ));
        }
        let listener = TcpListener::bind(resolved.as_slice())?;
        let addr = listener.local_addr()?;
        let endpoint = Self { addr, token: mint_session_token(), control };
        Ok((listener, endpoint))
    }

    /// The environment-variable contract a peer process reads to find and
    /// authenticate to this host session.
    ///
    /// - `PERL_DAP_PEER` = `HOST:PORT` (the bound loopback address)
    /// - `PERL_DAP_PEER_TOKEN` = the session token the peer must echo back in its
    ///   `peer/hello` (`HelloArgs::token`); the host rejects any handshake whose
    ///   token is missing or does not match
    /// - `PERL_DAP_PEER_MODE` = the control mode (`mirror` for this PR)
    #[must_use]
    pub fn env_vars(&self) -> Vec<(String, String)> {
        vec![
            (ENV_PEER_ADDR.to_string(), self.addr.to_string()),
            (ENV_PEER_TOKEN.to_string(), self.token.clone()),
            (ENV_PEER_MODE.to_string(), control_mode_env_str(self.control).to_string()),
        ]
    }
}

/// Mint a per-session token without pulling in a uuid/rand dependency.
///
/// `RandomState` is seeded from the OS RNG on each construction, so hashing a
/// per-call salt (time + a process-wide counter) yields an unpredictable
/// 128-bit value rendered as 32 hex chars. This is a session-scoped shared
/// secret for a loopback handshake, not a cryptographic key.
fn mint_session_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut token = String::with_capacity(32);
    for salt in 0u64..2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(salt);
        hasher.write_u128(nanos);
        hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
}

// ---------------------------------------------------------------------------
// Static conservative capability profile
// ---------------------------------------------------------------------------

/// The **static conservative** DAP capability profile advertised at
/// `initialize`, before any peer is connected.
///
/// The peer's runtime capabilities (learned at `peer/hello`) only ever *narrow*
/// behavior internally; they never require the editor to renegotiate, so the
/// editor-facing capabilities are fixed and conservative. `breakpointLocations`
/// is answered locally from the AST oracle, so it is always available;
/// conditional and function breakpoints are within the ptkdb v1 floor;
/// hovers/hit-conditions/logpoints/data-breakpoints are conservatively off.
#[must_use]
pub fn static_mirror_capabilities() -> Value {
    json!({
        "supportsConfigurationDoneRequest": true,
        "supportsConditionalBreakpoints": true,
        "supportsFunctionBreakpoints": true,
        "supportsBreakpointLocationsRequest": true,
        "supportsEvaluateForHovers": false,
        "supportsHitConditionalBreakpoints": false,
        "supportsLogPoints": false,
        "supportsDataBreakpoints": false,
        // Needed so the editor's Stop button can tear the session down; the peer
        // owns the debuggee lifecycle, so terminate is best-effort.
        "supportsTerminateRequest": true,
    })
}

// ---------------------------------------------------------------------------
// The mirror-mode DAP bridge (two-phase: pending → live)
// ---------------------------------------------------------------------------

/// Breakpoints queued for a source while the peer is not yet connected.
#[derive(Debug, Clone)]
struct QueuedBreakpoints {
    source: DebugSource,
    breakpoints: Vec<DebugBreakpoint>,
}

/// The mirror-mode DAP frontend for a listen-launch session.
///
/// It has two phases:
/// - **pending** (no peer): `initialize` returns [`static_mirror_capabilities`];
///   `setBreakpoints` is queued; control requests are rejected.
/// - **live** (peer handshaken): [`MirrorPeerBridge::go_live`] flushes queued
///   breakpoints and installs the backend; data requests are proxied and peer
///   events are mirrored.
///
/// Editor-initiated control (`continue`/`next`/`stepIn`/`stepOut`/`pause`) is
/// rejected in both phases when the control mode is [`ControlMode::Mirror`].
pub struct MirrorPeerBridge {
    backend: Option<Box<dyn DebugBackend>>,
    control: ControlMode,
    seq: i64,
    pending: Vec<QueuedBreakpoints>,
    /// Function breakpoints queued while no peer is connected yet (REPLACE
    /// semantics per DAP `setFunctionBreakpoints` — the most recent request
    /// before handshake is what gets flushed in [`Self::go_live`]).
    pending_function_breakpoints: Option<Vec<DebugFunctionBreakpoint>>,
    terminated_emitted: bool,
}

impl MirrorPeerBridge {
    /// Create a bridge in the **pending** phase (no peer connected yet).
    #[must_use]
    pub fn new_pending(control: ControlMode) -> Self {
        Self {
            backend: None,
            control,
            seq: 0,
            pending: Vec::new(),
            pending_function_breakpoints: None,
            terminated_emitted: false,
        }
    }

    /// Whether a peer backend has been installed (the session is live).
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.backend.is_some()
    }

    /// The number of sources with queued (not-yet-flushed) breakpoints.
    #[must_use]
    pub fn pending_source_count(&self) -> usize {
        self.pending.len()
    }

    /// Whether function breakpoints are queued (not-yet-flushed) awaiting the
    /// peer handshake.
    #[must_use]
    pub fn has_pending_function_breakpoints(&self) -> bool {
        self.pending_function_breakpoints.is_some()
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

    fn error(&mut self, request_seq: i64, command: &str, e: BackendError) -> DapMessage {
        self.response(request_seq, command, false, None, Some(e.to_string()))
    }

    /// Install a handshaken peer backend, flush queued breakpoints, and return
    /// the resulting `breakpoint` (reason `changed`) events for the editor.
    ///
    /// The `backend` must already have completed its `peer/hello` handshake
    /// (the caller's accept loop does this). Flushing sends each queued source's
    /// breakpoints to the peer and reports the resolved result as a `breakpoint`
    /// changed event, since the editor already received a `pending` response
    /// when the request was first queued.
    pub fn go_live(&mut self, mut backend: Box<dyn DebugBackend>) -> Vec<DapMessage> {
        let mut out = Vec::new();
        let queued = std::mem::take(&mut self.pending);
        for q in queued {
            let breakpoints = q.breakpoints.clone();
            match backend
                .set_breakpoints(SetBackendBreakpointsParams { source: q.source, breakpoints })
            {
                Ok(resolved) => {
                    for r in resolved {
                        let body = json!({
                            "reason": "changed",
                            "breakpoint": {
                                "id": r.id,
                                "verified": r.verified,
                                "line": r.actual_position.line,
                                "column": r.actual_position.column,
                                "message": r.message,
                            },
                        });
                        out.push(self.event("breakpoint", Some(body)));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "mirror bridge: flushing queued breakpoints failed");
                    // Tell the editor the queued breakpoints did not bind — it
                    // otherwise keeps showing the earlier `pending` placeholder
                    // forever, with no signal that the flush ever failed.
                    let message = format!("failed to set breakpoint after peer handshake: {e}");
                    for bp in q.breakpoints {
                        let body = json!({
                            "reason": "changed",
                            "breakpoint": {
                                "verified": false,
                                "line": bp.line,
                                "column": bp.column,
                                "message": message,
                            },
                        });
                        out.push(self.event("breakpoint", Some(body)));
                    }
                }
            }
        }
        if let Some(function_breakpoints) = self.pending_function_breakpoints.take() {
            let names: Vec<String> = function_breakpoints.iter().map(|b| b.name.clone()).collect();
            match backend.set_function_breakpoints(SetFunctionBreakpointsParams {
                breakpoints: function_breakpoints,
            }) {
                Ok(resolved) => {
                    for r in resolved {
                        let body = json!({
                            "reason": "changed",
                            "breakpoint": { "id": r.id, "verified": r.verified },
                        });
                        out.push(self.event("breakpoint", Some(body)));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "mirror bridge: flushing queued function breakpoints failed"
                    );
                    let message =
                        format!("failed to set function breakpoint after peer handshake: {e}");
                    for name in names {
                        let body = json!({
                            "reason": "changed",
                            "breakpoint": {
                                "verified": false,
                                "message": format!("{name}: {message}"),
                            },
                        });
                        out.push(self.event("breakpoint", Some(body)));
                    }
                }
            }
        }
        self.backend = Some(backend);
        out
    }

    /// Convert and drain any backend events into DAP event messages.
    ///
    /// Also synthesizes a single `terminated` event when the peer connection has
    /// closed without an explicit `debugger/terminated` (e.g. the peer process
    /// died), so the editor tears the session down instead of hanging.
    pub fn poll_events(&mut self) -> Vec<DapMessage> {
        let (events, closed) = match self.backend.as_mut() {
            Some(b) => (b.drain_events(), b.is_closed()),
            None => return Vec::new(),
        };
        let mut out = Vec::new();
        for ev in events {
            self.push_model_event(ev, &mut out);
        }
        if closed && !self.terminated_emitted {
            self.terminated_emitted = true;
            out.push(self.event("terminated", None));
        }
        out
    }

    fn push_model_event(&mut self, ev: DebugEvent, out: &mut Vec<DapMessage>) {
        match ev {
            // The DAP `initialized` event is emitted once, on the initialize
            // response; the peer's readiness signal is not re-forwarded.
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
                let body = json!({ "category": category.as_dap_category(), "output": output });
                out.push(self.event("output", Some(body)));
            }
            DebugEvent::Terminated { exit_code } => {
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

    /// Dispatch a single DAP request, returning the response followed by any
    /// backend events drained while servicing it.
    pub fn dispatch(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> Vec<DapMessage> {
        let mut out = Vec::new();
        match command {
            "initialize" => {
                // Static conservative profile — never blocks on or consults the
                // peer, which may not be connected yet.
                let body = static_mirror_capabilities();
                out.push(self.response(request_seq, command, true, Some(body), None));
                out.push(self.event("initialized", None));
            }
            "launch" | "attach" => {
                // In mirror listen mode the peer owns the debuggee; acknowledge.
                out.push(self.response(request_seq, command, true, None, None));
            }
            "configurationDone" => {
                out.push(self.response(request_seq, command, true, None, None));
            }
            "threads" => {
                let body = json!({ "threads": [{ "id": 1, "name": "main" }] });
                out.push(self.response(request_seq, command, true, Some(body), None));
            }
            "breakpointLocations" => {
                let body = handle_breakpoint_locations(arguments.as_ref());
                out.push(self.response(request_seq, command, true, Some(body), None));
            }
            "setBreakpoints" => {
                let msg = self.handle_set_breakpoints(request_seq, arguments.as_ref());
                out.push(msg);
            }
            "setFunctionBreakpoints" => {
                let msg = self.handle_set_function_breakpoints(request_seq, arguments.as_ref());
                out.push(msg);
            }
            "continue" | "next" | "stepIn" | "stepOut" | "pause" => {
                out.push(self.handle_control(request_seq, command));
            }
            "stackTrace" => out.push(self.handle_stack_trace(request_seq, arguments.as_ref())),
            "scopes" => out.push(self.handle_scopes(request_seq, arguments.as_ref())),
            "variables" => out.push(self.handle_variables(request_seq, arguments.as_ref())),
            "evaluate" => out.push(self.handle_evaluate(request_seq, arguments.as_ref())),
            "terminate" => {
                if let Some(b) = self.backend.as_mut() {
                    let _ = b.disconnect(true);
                }
                out.push(self.response(request_seq, command, true, None, None));
                // The peer may already have emitted `terminated` (e.g. its
                // connection closed just before the editor's `terminate`
                // arrived); guard against sending a second one.
                if !self.terminated_emitted {
                    self.terminated_emitted = true;
                    out.push(self.event("terminated", None));
                }
            }
            "disconnect" => {
                let terminate = arguments
                    .as_ref()
                    .and_then(|a| a.get("terminateDebuggee"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if let Some(b) = self.backend.as_mut() {
                    let _ = b.disconnect(terminate);
                }
                out.push(self.response(request_seq, command, true, None, None));
                if terminate && !self.terminated_emitted {
                    self.terminated_emitted = true;
                    out.push(self.event("terminated", None));
                }
            }
            other => {
                tracing::warn!(command = other, "mirror bridge: unhandled DAP request");
                out.push(self.response(request_seq, other, true, None, None));
            }
        }
        out.extend(self.poll_events());
        out
    }

    /// Reject an editor-initiated control request in mirror mode.
    ///
    /// In mirror mode the peer's own UI (ptkdb) owns execution, so the editor
    /// cannot drive control. The rejection is a well-formed unsuccessful DAP
    /// response (never a crash), so the editor surfaces a clear message.
    fn handle_control(&mut self, request_seq: i64, command: &str) -> DapMessage {
        match self.control {
            ControlMode::Mirror => self.response(
                request_seq,
                command,
                false,
                None,
                Some(format!(
                    "mirror mode: execution control ({command}) is owned by the external \
                     debugger UI; the editor cannot drive it"
                )),
            ),
            // Cooperative/dapControlled are out of scope for this PR; conservatively
            // reject rather than forward a control command the seam hasn't wired.
            _ => self.response(
                request_seq,
                command,
                false,
                None,
                Some(format!("control mode {:?} is not wired yet", self.control)),
            ),
        }
    }

    fn handle_set_breakpoints(&mut self, request_seq: i64, args: Option<&Value>) -> DapMessage {
        let Some(args) = args else {
            return self.error(
                request_seq,
                "setBreakpoints",
                BackendError::Protocol("missing arguments".into()),
            );
        };
        let source = dap_source(args.get("source"));
        let input = args.get("breakpoints").and_then(Value::as_array).cloned().unwrap_or_default();
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

        if let Some(backend) = self.backend.as_mut() {
            // Live: proxy straight to the peer.
            match backend.set_breakpoints(SetBackendBreakpointsParams { source, breakpoints }) {
                Ok(resolved) => {
                    let bps = slots
                        .iter()
                        .map(|slot| match slot.and_then(|i| resolved.get(i)) {
                            Some(r) => json!({
                                "id": r.id,
                                "verified": r.verified,
                                "line": r.actual_position.line,
                                "column": r.actual_position.column,
                                "message": r.message,
                            }),
                            None => json!({ "verified": false, "message": "line required" }),
                        })
                        .collect::<Vec<_>>();
                    self.response(
                        request_seq,
                        "setBreakpoints",
                        true,
                        Some(json!({ "breakpoints": bps })),
                        None,
                    )
                }
                Err(e) => self.error(request_seq, "setBreakpoints", e),
            }
        } else {
            // Pending: queue (REPLACE semantics per source) and answer with an
            // unverified, `pending` response. The verified result arrives later
            // as a `breakpoint` changed event when the peer handshakes (go_live).
            self.pending.retain(|q| q.source.path != source.path);
            self.pending.push(QueuedBreakpoints { source, breakpoints });
            let bps = slots
                .iter()
                .zip(input.iter())
                .map(|(slot, b)| match slot {
                    Some(_) => json!({
                        "verified": false,
                        "line": b.get("line").and_then(Value::as_u64),
                        "message": "pending: waiting for the external debugger peer to connect",
                    }),
                    None => json!({ "verified": false, "message": "line required" }),
                })
                .collect::<Vec<_>>();
            self.response(
                request_seq,
                "setBreakpoints",
                true,
                Some(json!({ "breakpoints": bps })),
                None,
            )
        }
    }

    fn handle_set_function_breakpoints(
        &mut self,
        request_seq: i64,
        args: Option<&Value>,
    ) -> DapMessage {
        let input = args
            .and_then(|a| a.get("breakpoints"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
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
        // Pending: queue (REPLACE semantics, mirroring setBreakpoints) and
        // answer with an unverified `pending` response. The verified result
        // arrives later as a `breakpoint` changed event once the peer
        // handshakes (go_live flushes `pending_function_breakpoints`).
        let Some(backend) = self.backend.as_mut() else {
            self.pending_function_breakpoints = Some(breakpoints);
            let bps = slots
                .iter()
                .map(|_| {
                    json!({
                        "verified": false,
                        "message": "pending: waiting for the external debugger peer to connect",
                    })
                })
                .collect::<Vec<_>>();
            return self.response(
                request_seq,
                "setFunctionBreakpoints",
                true,
                Some(json!({ "breakpoints": bps })),
                None,
            );
        };
        match backend.set_function_breakpoints(SetFunctionBreakpointsParams { breakpoints }) {
            Ok(resolved) => {
                let bps = slots
                    .iter()
                    .map(|slot| match slot.and_then(|i| resolved.get(i)) {
                        Some(r) => json!({ "id": r.id, "verified": r.verified }),
                        None => json!({ "verified": false, "message": "name required" }),
                    })
                    .collect::<Vec<_>>();
                self.response(
                    request_seq,
                    "setFunctionBreakpoints",
                    true,
                    Some(json!({ "breakpoints": bps })),
                    None,
                )
            }
            Err(e) => self.error(request_seq, "setFunctionBreakpoints", e),
        }
    }

    fn handle_stack_trace(&mut self, request_seq: i64, args: Option<&Value>) -> DapMessage {
        let Some(backend) = self.backend.as_mut() else {
            return self.response(
                request_seq,
                "stackTrace",
                true,
                Some(json!({ "stackFrames": [], "totalFrames": 0 })),
                None,
            );
        };
        let params = StackTraceParams {
            thread_id: thread_id_arg(args),
            start_frame: args
                .and_then(|a| a.get("startFrame"))
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            levels: args.and_then(|a| a.get("levels")).and_then(Value::as_u64).map(|v| v as u32),
        };
        match backend.stack_trace(params) {
            Ok(frames) => {
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
                self.response(
                    request_seq,
                    "stackTrace",
                    true,
                    Some(json!({ "stackFrames": out, "totalFrames": total })),
                    None,
                )
            }
            Err(e) => self.error(request_seq, "stackTrace", e),
        }
    }

    fn handle_scopes(&mut self, request_seq: i64, args: Option<&Value>) -> DapMessage {
        let Some(backend) = self.backend.as_mut() else {
            return self.response(request_seq, "scopes", true, Some(json!({ "scopes": [] })), None);
        };
        let frame_id =
            FrameId(args.and_then(|a| a.get("frameId")).and_then(Value::as_i64).unwrap_or(0));
        match backend.scopes(frame_id) {
            Ok(scopes) => {
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
                self.response(request_seq, "scopes", true, Some(json!({ "scopes": out })), None)
            }
            Err(e) => self.error(request_seq, "scopes", e),
        }
    }

    fn handle_variables(&mut self, request_seq: i64, args: Option<&Value>) -> DapMessage {
        let Some(backend) = self.backend.as_mut() else {
            return self.response(
                request_seq,
                "variables",
                true,
                Some(json!({ "variables": [] })),
                None,
            );
        };
        let vref = VariablesRef(
            args.and_then(|a| a.get("variablesReference")).and_then(Value::as_i64).unwrap_or(0),
        );
        match backend.variables(vref) {
            Ok(vars) => {
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
                self.response(
                    request_seq,
                    "variables",
                    true,
                    Some(json!({ "variables": out })),
                    None,
                )
            }
            Err(e) => self.error(request_seq, "variables", e),
        }
    }

    fn handle_evaluate(&mut self, request_seq: i64, args: Option<&Value>) -> DapMessage {
        let Some(backend) = self.backend.as_mut() else {
            return self.error(request_seq, "evaluate", BackendError::NotConnected);
        };
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
        match backend.evaluate(EvaluateParams { expression, frame_id, context }) {
            Ok(result) => self.response(
                request_seq,
                "evaluate",
                true,
                Some(json!({
                    "result": result.result,
                    "type": result.type_name,
                    "variablesReference": result.variables_reference.map(|r| r.0).unwrap_or(0),
                })),
                None,
            ),
            Err(e) => self.error(request_seq, "evaluate", e),
        }
    }
}

// ---------------------------------------------------------------------------
// Session drivers
// ---------------------------------------------------------------------------

/// Accept a peer on `peer_listener`, complete its (token-authenticated)
/// handshake, and deliver the ready backend over a channel — used by the
/// listen-mode session drivers so the editor loop can serve DAP (static caps,
/// queued breakpoints) before the peer has connected. When `expected_token` is
/// `Some`, only a peer that presents the matching token in its `peer/hello` is
/// delivered as a live backend; a mismatched or tokenless handshake is rejected
/// and no backend is produced.
fn spawn_peer_acceptor(
    peer_listener: TcpListener,
    handshake_timeout: Duration,
    expected_token: Option<String>,
) -> mpsc::Receiver<Box<dyn DebugBackend>> {
    let (tx, rx) = mpsc::channel::<Box<dyn DebugBackend>>();
    std::thread::spawn(move || {
        peer_listener.set_nonblocking(true).ok();
        let deadline = Instant::now() + handshake_timeout;
        let stream = loop {
            match peer_listener.accept() {
                Ok((s, _)) => break Some(s),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break None,
            }
        };
        let Some(stream) = stream else { return };
        if stream.set_nonblocking(false).is_err() {
            return;
        }
        // Enforce the session token minted at bind (listen mode always mints
        // one). A peer that connected to the loopback port but cannot present
        // the matching secret is rejected inside the handshake and never
        // delivered here as a live backend.
        let mut backend = match ExternalDebuggerPeerBackend::from_connected_stream_with_token(
            stream,
            handshake_timeout,
            expected_token,
        ) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, "mirror listen: building peer backend failed");
                return;
            }
        };
        if let Err(e) = backend.initialize(InitializeBackendParams::default()) {
            tracing::warn!(error = %e, "mirror listen: peer handshake failed");
            return;
        }
        let _ = tx.send(Box::new(backend));
    });
    rx
}

/// Drive a [`MirrorPeerBridge`] listen-launch session over **stdio** (the editor
/// spawns `perl-dap` and speaks DAP over stdin/stdout) while the peer connects
/// back on `peer_listener`. `expected_token` is the session token the peer must
/// present in its `peer/hello` (pass `Some(endpoint.token)`); a peer that
/// cannot present it is rejected during the handshake.
///
/// # Errors
/// Returns a transport error if writing framed DAP messages to stdout fails.
pub fn run_mirror_listen_session_stdio(
    peer_listener: TcpListener,
    bridge: MirrorPeerBridge,
    handshake_timeout: Duration,
    poll_interval: Duration,
    expected_token: Option<String>,
) -> std::io::Result<()> {
    let peer_rx = spawn_peer_acceptor(peer_listener, handshake_timeout, expected_token);
    run_mirror_editor_loop(std::io::stdin(), std::io::stdout(), bridge, peer_rx, poll_interval)
}

/// Drive a [`MirrorPeerBridge`] listen-launch session over a **socket** editor
/// connection while the peer connects back on `peer_listener`. `expected_token`
/// is the session token the peer must present in its `peer/hello` (pass
/// `Some(endpoint.token)`); a peer that cannot present it is rejected during the
/// handshake.
///
/// # Errors
/// Returns a transport error if the socket read/write fails irrecoverably.
pub fn run_mirror_listen_session_socket(
    editor: TcpStream,
    peer_listener: TcpListener,
    bridge: MirrorPeerBridge,
    handshake_timeout: Duration,
    poll_interval: Duration,
    expected_token: Option<String>,
) -> std::io::Result<()> {
    let peer_rx = spawn_peer_acceptor(peer_listener, handshake_timeout, expected_token);
    let reader = editor.try_clone()?;
    let writer = editor;
    run_mirror_editor_loop(reader, writer, bridge, peer_rx, poll_interval)
}

/// The transport-agnostic editor loop: read framed DAP requests off `reader_src`
/// on a dedicated thread, dispatch them, write framed responses/events to
/// `writer`, interleave backend-event delivery, and transition the bridge to
/// live when the peer backend arrives on `peer_rx`.
fn run_mirror_editor_loop<R, W>(
    reader_src: R,
    mut writer: W,
    mut bridge: MirrorPeerBridge,
    peer_rx: mpsc::Receiver<Box<dyn DebugBackend>>,
    poll_interval: Duration,
) -> std::io::Result<()>
where
    R: Read + Send + 'static,
    W: Write,
{
    use perl_lsp_rs_core::transport::ContentLengthFramer;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _reader = std::thread::spawn(move || {
        let mut src = reader_src;
        let mut framer = ContentLengthFramer::new();
        let mut buf = [0u8; 8 * 1024];
        loop {
            match src.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    framer.push(&buf[..n]);
                    loop {
                        match framer.try_next() {
                            Ok(Some(body)) => {
                                if tx.send(body).is_err() {
                                    return;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "mirror listen (editor): dropping malformed DAP frame"
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
        // Transition to live as soon as the peer backend is ready, flushing any
        // breakpoints the editor already sent.
        if !bridge.is_live() {
            match peer_rx.try_recv() {
                Ok(backend) => {
                    let flush = bridge.go_live(backend);
                    if !flush.is_empty() {
                        write_dap_msgs(&mut writer, &flush)?;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    // spawn_peer_acceptor's sender was dropped without ever
                    // delivering a live backend — the handshake deadline
                    // elapsed (or failed) with no peer. Without this, the
                    // editor session stays pending forever: tell the editor
                    // the session ended instead of silently hanging.
                    if !bridge.terminated_emitted {
                        bridge.terminated_emitted = true;
                        let msg = bridge.event("terminated", None);
                        write_dap_msgs(&mut writer, &[msg])?;
                    }
                    break;
                }
            }
        }

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
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn write_dap_msgs<W: Write>(writer: &mut W, msgs: &[DapMessage]) -> std::io::Result<()> {
    use perl_lsp_rs_core::transport::frame;
    for m in msgs {
        match serde_json::to_vec(m) {
            Ok(body) => writer.write_all(&frame(&body))?,
            Err(e) => {
                tracing::error!(error = %e, "mirror listen: dropping unserializable DAP message")
            }
        }
    }
    writer.flush()
}

fn dispatch_frame(bridge: &mut MirrorPeerBridge, body: &[u8]) -> (Vec<DapMessage>, bool) {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let Some(command) = v.get("command").and_then(Value::as_str) else {
        tracing::warn!("mirror listen: dropping DAP frame with no `command`");
        return (Vec::new(), false);
    };
    let seq =
        v.get("seq").and_then(|s| s.as_i64().or_else(|| s.as_f64().map(|f| f as i64))).unwrap_or(0);
    let out = bridge.dispatch(seq, command, v.get("arguments").cloned());
    let disconnect = command == "disconnect";
    (out, disconnect)
}

// ---------------------------------------------------------------------------
// One-shot helper: bind + build a pending bridge from a launch config
// ---------------------------------------------------------------------------

/// Prepare a mirror listen session from a parsed launch config: bind the peer
/// listener (allocating an ephemeral port when `port == 0`), mint the token, and
/// build a pending [`MirrorPeerBridge`]. The returned [`PeerListenEndpoint`]
/// carries the env-var contract the (future) peer process reads via
/// [`PeerListenEndpoint::env_vars`].
///
/// # Errors
/// Fails if the peer listener cannot be bound.
pub fn prepare_mirror_listen_session(
    config: &ExternalPeerLaunchConfig,
) -> std::io::Result<(TcpListener, PeerListenEndpoint, MirrorPeerBridge)> {
    let (listener, endpoint) = PeerListenEndpoint::bind(&config.host, config.port, config.control)?;
    let bridge = MirrorPeerBridge::new_pending(config.control);
    Ok((listener, endpoint, bridge))
}

/// The default peer-handshake timeout for a listen-launch session.
pub const DEFAULT_LISTEN_HANDSHAKE_TIMEOUT: Duration = DEFAULT_PEER_TIMEOUT;

// ---------------------------------------------------------------------------
// Translation helpers
// ---------------------------------------------------------------------------

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

fn thread_id_arg(args: Option<&Value>) -> ThreadId {
    ThreadId(args.and_then(|a| a.get("threadId")).and_then(Value::as_i64).unwrap_or(1))
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

/// Answer a DAP `breakpointLocations` request from the local AST oracle (the
/// source is on the same host as `perl-dap`), independent of the peer.
fn handle_breakpoint_locations(args: Option<&Value>) -> Value {
    let empty = json!({ "breakpoints": [] });
    let Some(args) = args else { return empty };
    let Some(path) = args.get("source").and_then(|s| s.get("path")).and_then(Value::as_str) else {
        return empty;
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_returns_none_for_non_external_backend() {
        let args = json!({ "debuggerBackend": "native", "program": "/x.pl" });
        assert!(ExternalPeerLaunchConfig::from_launch_arguments(&args).is_none());
        // Missing debuggerBackend also yields None.
        assert!(ExternalPeerLaunchConfig::from_launch_arguments(&json!({})).is_none());
    }

    #[test]
    fn parse_full_mirror_listen_config() {
        let args = json!({
            "debuggerBackend": "external",
            "externalDebugger": {
                "kind": "ptkdb",
                "mode": "listen",
                "control": "mirror",
                "host": "127.0.0.1",
                "port": 0,
            },
        });
        let cfg = ExternalPeerLaunchConfig::from_launch_arguments(&args).expect("external config");
        assert_eq!(cfg.kind, ExternalDebuggerKind::Ptkdb);
        assert_eq!(cfg.mode, PeerRendezvousMode::Listen);
        assert_eq!(cfg.control, ControlMode::Mirror);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 0);
    }

    #[test]
    fn parse_defaults_when_external_block_absent() {
        let cfg = ExternalPeerLaunchConfig::from_launch_arguments(
            &json!({ "debuggerBackend": "external" }),
        )
        .expect("defaults");
        assert_eq!(cfg.kind, ExternalDebuggerKind::Ptkdb);
        assert_eq!(cfg.mode, PeerRendezvousMode::Connect);
        assert_eq!(cfg.control, ControlMode::Mirror);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 0);
    }

    #[test]
    fn bind_allocates_ephemeral_port_and_env_contract() {
        let (listener, endpoint) =
            PeerListenEndpoint::bind("127.0.0.1", 0, ControlMode::Mirror).expect("bind");
        assert_ne!(endpoint.addr.port(), 0, "port 0 must resolve to an OS-assigned port");
        assert!(endpoint.addr.ip().is_loopback());
        assert_eq!(endpoint.token.len(), 32, "token is 32 hex chars");
        assert!(endpoint.token.chars().all(|c| c.is_ascii_hexdigit()));

        let env: std::collections::HashMap<_, _> = endpoint.env_vars().into_iter().collect();
        assert_eq!(env[ENV_PEER_ADDR], endpoint.addr.to_string());
        assert_eq!(env[ENV_PEER_TOKEN], endpoint.token);
        assert_eq!(env[ENV_PEER_MODE], "mirror");
        drop(listener);
    }

    #[test]
    fn bind_refuses_non_loopback_host() {
        // A mirror session must never be exposed beyond loopback: binding a
        // routable interface (e.g. 0.0.0.0, all-interfaces) would expose the
        // debuggee's output/stack/variables to the network. The token
        // authenticates the handshake, but loopback bind is the layered control
        // that keeps the port off the network; `bind` must refuse rather than
        // expose.
        for host in ["0.0.0.0", "::"] {
            let err = PeerListenEndpoint::bind(host, 0, ControlMode::Mirror)
                .expect_err("non-loopback host must be refused");
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::InvalidInput,
                "{host} must be refused as InvalidInput, got {err:?}"
            );
        }
        // Loopback forms remain accepted.
        for host in ["127.0.0.1", "localhost"] {
            let (listener, endpoint) = PeerListenEndpoint::bind(host, 0, ControlMode::Mirror)
                .expect("loopback host must bind");
            assert!(endpoint.addr.ip().is_loopback());
            drop(listener);
        }
    }

    #[test]
    fn tokens_are_unique_per_session() {
        let a = mint_session_token();
        let b = mint_session_token();
        assert_ne!(a, b, "each session mints a distinct token");
    }

    #[test]
    fn static_capabilities_match_the_conservative_profile() {
        let caps = static_mirror_capabilities();
        assert_eq!(caps["supportsConfigurationDoneRequest"], true);
        assert_eq!(caps["supportsConditionalBreakpoints"], true);
        assert_eq!(caps["supportsFunctionBreakpoints"], true);
        assert_eq!(caps["supportsBreakpointLocationsRequest"], true);
        assert_eq!(caps["supportsEvaluateForHovers"], false);
        assert_eq!(caps["supportsHitConditionalBreakpoints"], false);
        assert_eq!(caps["supportsLogPoints"], false);
        assert_eq!(caps["supportsDataBreakpoints"], false);
    }

    #[test]
    fn initialize_returns_static_caps_before_any_peer() {
        let mut bridge = MirrorPeerBridge::new_pending(ControlMode::Mirror);
        assert!(!bridge.is_live());
        let out = bridge.dispatch(1, "initialize", Some(json!({ "adapterID": "perl" })));
        let (cmd, ok, body) = as_response(&out[0]);
        assert_eq!(cmd, "initialize");
        assert!(ok);
        let caps = body.expect("caps");
        assert_eq!(caps["supportsConditionalBreakpoints"], true);
        assert_eq!(caps["supportsLogPoints"], false);
        assert_eq!(event_name(&out[1]), "initialized");
    }

    #[test]
    fn mirror_rejects_control_gracefully_without_a_peer() {
        let mut bridge = MirrorPeerBridge::new_pending(ControlMode::Mirror);
        for cmd in ["continue", "next", "stepIn", "stepOut", "pause"] {
            let out = bridge.dispatch(2, cmd, Some(json!({ "threadId": 1 })));
            let (rcmd, ok, _) = as_response(&out[0]);
            assert_eq!(rcmd, cmd);
            assert!(!ok, "{cmd} must be rejected in mirror mode");
            if let DapMessage::Response { message, .. } = &out[0] {
                assert!(message.as_deref().unwrap_or("").contains("mirror mode"));
            }
        }
    }

    #[test]
    fn setbreakpoints_before_handshake_queues_and_answers_pending() {
        let mut bridge = MirrorPeerBridge::new_pending(ControlMode::Mirror);
        let out = bridge.dispatch(
            3,
            "setBreakpoints",
            Some(json!({
                "source": { "path": "/work/script.pl" },
                "breakpoints": [{ "line": 42 }, { "line": 7 }],
            })),
        );
        assert_eq!(bridge.pending_source_count(), 1);
        let (_, ok, body) = as_response(&out[0]);
        assert!(ok, "queued setBreakpoints still returns a success response");
        let bps = body.expect("body")["breakpoints"].as_array().expect("array").clone();
        assert_eq!(bps.len(), 2);
        assert_eq!(bps[0]["verified"], false, "queued breakpoints are unverified until flush");
    }

    fn as_response(msg: &DapMessage) -> (&str, bool, Option<&Value>) {
        match msg {
            DapMessage::Response { command, success, body, .. } => {
                (command.as_str(), *success, body.as_ref())
            }
            other => panic!("expected response, got {other:?}"),
        }
    }

    fn event_name(msg: &DapMessage) -> &str {
        match msg {
            DapMessage::Event { event, .. } => event.as_str(),
            other => panic!("expected event, got {other:?}"),
        }
    }
}
