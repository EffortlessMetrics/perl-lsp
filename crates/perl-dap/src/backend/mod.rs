//! Backend abstraction for Perl debug sessions.
//!
//! The DAP frontend ([`crate::debug_adapter`]) speaks DAP to editors. Behind it,
//! a [`DebugBackend`] drives *some* debugger engine — the stock `perl -d`
//! runtime, the legacy Perl::LanguageServer bridge, or an external debugger peer
//! (ptkdb-first). Every backend is expressed in terms of the canonical,
//! backend-neutral [`crate::model`] vocabulary, never DAP wire types. That is
//! what lets an external engine cooperate without implementing DAP.
//!
//! See [`docs/reference/EXTERNAL_DEBUGGER_PEER_DECISIONS.md`] (decision D1) for
//! why the contract is model-typed rather than `DapMessage`-typed.

pub mod capabilities;
pub mod external_peer;
pub mod native_perldb;
pub mod peer_bridge;

use std::collections::HashMap;
use std::path::PathBuf;

pub use capabilities::{DebugBackendCapabilities, intersect_dap_capabilities};
pub use peer_bridge::{DapPeerBridge, run_external_peer_session, run_external_peer_session_stdio};

use crate::model::{
    DebugBreakpoint, DebugEvent, DebugFunctionBreakpoint, DebugScope, DebugSource, DebugStackFrame,
    DebugVariable, FrameId, ResolvedBreakpoint, ThreadId, VariablesRef,
};

/// Errors a backend can surface.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// The backend is not connected to its engine (peer not attached yet).
    #[error("debug backend is not connected")]
    NotConnected,
    /// A timeout elapsed waiting for the engine/peer to respond.
    #[error("debug backend timed out: {0}")]
    Timeout(String),
    /// The peer/engine reported an error for a request.
    #[error("debug backend reported an error: {0}")]
    Engine(String),
    /// The operation is not supported by this backend/negotiated capabilities.
    #[error("operation not supported by this backend: {0}")]
    Unsupported(String),
    /// Transport/framing failure.
    #[error("debug backend transport error: {0}")]
    Transport(String),
    /// Serialization/deserialization failure.
    #[error("debug backend protocol error: {0}")]
    Protocol(String),
}

/// Result alias for backend operations.
pub type BackendResult<T> = Result<T, BackendError>;

/// Parameters for [`DebugBackend::initialize`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitializeBackendParams {
    /// Editor/client identifier, if provided.
    pub client_id: Option<String>,
    /// Adapter identifier advertised by the editor (usually `"perl"`).
    pub adapter_id: String,
    /// Whether the client uses 1-based lines.
    pub lines_start_at_1: bool,
    /// Whether the client uses 1-based columns.
    pub columns_start_at_1: bool,
}

/// Parameters for [`DebugBackend::launch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchBackendParams {
    /// Program to debug.
    pub program: PathBuf,
    /// Program arguments.
    pub args: Vec<String>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
    /// Environment overrides.
    pub env: HashMap<String, String>,
    /// `@INC` additions.
    pub include_paths: Vec<PathBuf>,
    /// Whether to stop at program entry.
    pub stop_on_entry: bool,
}

/// Outcome of a launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    /// Whether the launch was accepted.
    pub success: bool,
}

/// Parameters for [`DebugBackend::attach`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachBackendParams {
    /// Host of the running debuggee/engine.
    pub host: String,
    /// Port of the running debuggee/engine.
    pub port: u16,
}

/// Outcome of an attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachResult {
    /// Whether the attach was accepted.
    pub success: bool,
}

/// Parameters for [`DebugBackend::set_breakpoints`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetBackendBreakpointsParams {
    /// Source the breakpoints apply to.
    pub source: DebugSource,
    /// Requested breakpoints (REPLACE semantics — this is the full set).
    pub breakpoints: Vec<DebugBreakpoint>,
}

/// Parameters for [`DebugBackend::set_function_breakpoints`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFunctionBreakpointsParams {
    /// Requested function breakpoints (REPLACE semantics).
    pub breakpoints: Vec<DebugFunctionBreakpoint>,
}

/// Outcome of a continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueResult {
    /// Whether all threads resumed (Perl is single-threaded, usually `true`).
    pub all_threads_continued: bool,
}

/// Parameters for [`DebugBackend::stack_trace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackTraceParams {
    /// Thread to inspect.
    pub thread_id: ThreadId,
    /// Index of the first frame to return.
    pub start_frame: Option<u32>,
    /// Maximum number of frames to return.
    pub levels: Option<u32>,
}

/// The `context` an evaluate request was issued in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateContext {
    /// A watch expression.
    Watch,
    /// The debug REPL / console.
    Repl,
    /// A hover in the editor.
    Hover,
    /// The variables view.
    Variables,
    /// Any other context, carrying the raw label.
    Other(String),
}

/// Parameters for [`DebugBackend::evaluate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateParams {
    /// Expression to evaluate.
    pub expression: String,
    /// Frame to evaluate in, if any.
    pub frame_id: Option<FrameId>,
    /// Context the evaluate was requested in.
    pub context: EvaluateContext,
}

/// Result of an evaluate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateResult {
    /// Rendered result value.
    pub result: String,
    /// Type/ref-kind, when known.
    pub type_name: Option<String>,
    /// Handle to expand children, if the value is structured.
    pub variables_reference: Option<VariablesRef>,
}

/// A debugger engine driver, expressed in the canonical [`crate::model`] terms.
///
/// Implementations translate between the model and their engine's native or
/// wire representation. Methods are synchronous; backends that talk to an
/// asynchronous engine (e.g. a socket peer) encapsulate that internally and
/// surface out-of-band engine events through [`DebugBackend::drain_events`].
pub trait DebugBackend: Send {
    /// A short, stable identifier for the backend (for logs/diagnostics).
    fn name(&self) -> &str;

    /// The capabilities this backend can offer, after any negotiation.
    fn capabilities(&self) -> DebugBackendCapabilities;

    /// Initialize the backend/session.
    fn initialize(&mut self, params: InitializeBackendParams) -> BackendResult<()>;

    /// Launch a new debuggee.
    fn launch(&mut self, params: LaunchBackendParams) -> BackendResult<LaunchResult>;

    /// Attach to a running debuggee/engine.
    fn attach(&mut self, params: AttachBackendParams) -> BackendResult<AttachResult>;

    /// Set (replace) the source breakpoints for a source.
    fn set_breakpoints(
        &mut self,
        params: SetBackendBreakpointsParams,
    ) -> BackendResult<Vec<ResolvedBreakpoint>>;

    /// Set (replace) function/subroutine breakpoints.
    fn set_function_breakpoints(
        &mut self,
        params: SetFunctionBreakpointsParams,
    ) -> BackendResult<Vec<ResolvedBreakpoint>>;

    /// Resume a thread.
    fn continue_thread(&mut self, thread_id: ThreadId) -> BackendResult<ContinueResult>;

    /// Step over (`next`).
    fn next(&mut self, thread_id: ThreadId) -> BackendResult<()>;

    /// Step into.
    fn step_in(&mut self, thread_id: ThreadId) -> BackendResult<()>;

    /// Step out.
    fn step_out(&mut self, thread_id: ThreadId) -> BackendResult<()>;

    /// Pause a running thread.
    fn pause(&mut self, thread_id: ThreadId) -> BackendResult<()>;

    /// Fetch the stack trace of a stopped thread.
    fn stack_trace(&mut self, params: StackTraceParams) -> BackendResult<Vec<DebugStackFrame>>;

    /// Fetch the scopes for a frame.
    fn scopes(&mut self, frame_id: FrameId) -> BackendResult<Vec<DebugScope>>;

    /// Fetch the variables behind a scope/variable reference.
    fn variables(&mut self, variables_ref: VariablesRef) -> BackendResult<Vec<DebugVariable>>;

    /// Evaluate an expression.
    fn evaluate(&mut self, params: EvaluateParams) -> BackendResult<EvaluateResult>;

    /// Drain any engine-originated events (stopped/output/…). Non-blocking.
    ///
    /// The default returns nothing; backends with an asynchronous engine
    /// override this to surface events the frontend forwards to the editor.
    fn drain_events(&mut self) -> Vec<DebugEvent> {
        Vec::new()
    }

    /// Disconnect from the engine.
    fn disconnect(&mut self, terminate_debuggee: bool) -> BackendResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DebugPosition, StopReason};

    /// A trivial in-memory backend used to prove the trait is object-safe and
    /// the model-typed contract composes.
    #[derive(Default)]
    struct MockBackend {
        events: Vec<DebugEvent>,
        launched: bool,
    }

    impl DebugBackend for MockBackend {
        fn name(&self) -> &str {
            "mock"
        }
        fn capabilities(&self) -> DebugBackendCapabilities {
            DebugBackendCapabilities::full()
        }
        fn initialize(&mut self, _params: InitializeBackendParams) -> BackendResult<()> {
            self.events.push(DebugEvent::Initialized);
            Ok(())
        }
        fn launch(&mut self, _params: LaunchBackendParams) -> BackendResult<LaunchResult> {
            self.launched = true;
            Ok(LaunchResult { success: true })
        }
        fn attach(&mut self, _params: AttachBackendParams) -> BackendResult<AttachResult> {
            Ok(AttachResult { success: true })
        }
        fn set_breakpoints(
            &mut self,
            params: SetBackendBreakpointsParams,
        ) -> BackendResult<Vec<ResolvedBreakpoint>> {
            Ok(params
                .breakpoints
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
            _params: SetFunctionBreakpointsParams,
        ) -> BackendResult<Vec<ResolvedBreakpoint>> {
            Ok(Vec::new())
        }
        fn continue_thread(&mut self, _thread_id: ThreadId) -> BackendResult<ContinueResult> {
            Ok(ContinueResult { all_threads_continued: true })
        }
        fn next(&mut self, _thread_id: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn step_in(&mut self, _thread_id: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn step_out(&mut self, _thread_id: ThreadId) -> BackendResult<()> {
            Ok(())
        }
        fn pause(&mut self, thread_id: ThreadId) -> BackendResult<()> {
            self.events.push(DebugEvent::Stopped {
                reason: StopReason::Pause,
                thread_id,
                position: None,
            });
            Ok(())
        }
        fn stack_trace(
            &mut self,
            _params: StackTraceParams,
        ) -> BackendResult<Vec<DebugStackFrame>> {
            Ok(Vec::new())
        }
        fn scopes(&mut self, _frame_id: FrameId) -> BackendResult<Vec<DebugScope>> {
            Ok(Vec::new())
        }
        fn variables(&mut self, _variables_ref: VariablesRef) -> BackendResult<Vec<DebugVariable>> {
            Ok(Vec::new())
        }
        fn evaluate(&mut self, params: EvaluateParams) -> BackendResult<EvaluateResult> {
            Ok(EvaluateResult {
                result: format!("=> {}", params.expression),
                type_name: None,
                variables_reference: None,
            })
        }
        fn drain_events(&mut self) -> Vec<DebugEvent> {
            std::mem::take(&mut self.events)
        }
        fn disconnect(&mut self, _terminate_debuggee: bool) -> BackendResult<()> {
            Ok(())
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let mut b: Box<dyn DebugBackend> = Box::new(MockBackend::default());
        assert_eq!(b.name(), "mock");
        b.initialize(InitializeBackendParams::default()).expect("init");
        let events = b.drain_events();
        assert_eq!(events, vec![DebugEvent::Initialized]);
    }

    #[test]
    fn mock_resolves_breakpoints_from_model() {
        let mut b = MockBackend::default();
        let src = DebugSource::from_path("/work/script.pl");
        let out = b
            .set_breakpoints(SetBackendBreakpointsParams {
                source: src.clone(),
                breakpoints: vec![DebugBreakpoint {
                    id: None,
                    source: src,
                    line: 42,
                    column: None,
                    condition: Some("$x > 10".to_string()),
                    hit_condition: None,
                    log_message: None,
                }],
            })
            .expect("set breakpoints");
        assert_eq!(out.len(), 1);
        assert!(out[0].verified);
        assert_eq!(out[0].actual_position.line, 42);
    }
}
