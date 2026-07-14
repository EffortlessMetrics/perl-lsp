//! Canonical, backend-neutral Perl debug model.
//!
//! These types describe *what a Perl debugger session contains* — sources,
//! positions, breakpoints, stack frames, scopes, variables, stop reasons —
//! without being shaped by any particular wire protocol. They are deliberately
//! **not** DAP structs (those live in [`crate::protocol`]) and **not**
//! peer-protocol structs (those live in [`crate::peer_protocol`]).
//!
//! # Why a separate model
//!
//! `perl-dap` speaks DAP to editors and a small JSON peer protocol to external
//! debugger engines (ptkdb-first). If either wire shape leaked into every
//! backend, adding a second backend would mean re-deriving the whole surface.
//! The canonical model is the shared vocabulary every [`crate::backend::DebugBackend`]
//! produces and consumes; translation to/from DAP and the peer protocol happens
//! only at the respective boundaries.
//!
//! These types intentionally use distinct names (`DebugSource`, `DebugStackFrame`,
//! `DebugVariable`) from the existing DAP-adjacent internal types in
//! [`crate::types`] (`Source`, `StackFrame`, `Variable`) so the two layers never
//! get confused during translation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Identifier for a debuggee thread.
///
/// Perl's stock debugger is single-threaded, so this is almost always `1`, but
/// the model keeps it explicit so multi-interpreter peers can distinguish them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub i64);

/// Identifier for a stack frame within a stopped thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameId(pub i64);

/// Opaque handle used to lazily expand a structured variable/scope.
///
/// A value of `0` conventionally means "no children" (matching DAP's
/// `variablesReference` convention), so producers should allocate positive
/// references for expandable values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VariablesRef(pub i64);

impl VariablesRef {
    /// The sentinel reference meaning "this value has no expandable children".
    pub const NONE: VariablesRef = VariablesRef(0);

    /// Whether this reference points at expandable children.
    #[must_use]
    pub fn is_expandable(self) -> bool {
        self.0 != 0
    }
}

/// A source file participating in a debug session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSource {
    /// Absolute path to the source on the machine running the debuggee.
    pub path: PathBuf,
    /// Human-friendly short name (usually the file's basename).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Reference for sources without an on-disk path (e.g. `eval` text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
}

impl DebugSource {
    /// Construct a source from a path, deriving `name` from the basename.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = path.file_name().and_then(|n| n.to_str()).map(ToString::to_string);
        Self { path, name, source_reference: None }
    }
}

/// A precise position (line, optional column) inside a source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugPosition {
    /// The source this position refers to.
    pub source: DebugSource,
    /// 1-based line number.
    pub line: u32,
    /// Optional 1-based column number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

/// A breakpoint request expressed against a source position.
///
/// This is the *desired* breakpoint; the backend resolves it into a
/// [`ResolvedBreakpoint`] once the engine confirms placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBreakpoint {
    /// Backend-assigned id, if the breakpoint has been registered yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// The source the breakpoint lives in.
    pub source: DebugSource,
    /// 1-based line number.
    pub line: u32,
    /// Optional 1-based column number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Perl expression that must be true for the breakpoint to fire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit-count expression (e.g. `>= 3`) gating when the breakpoint fires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    /// Logpoint message; when set the engine logs instead of stopping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
}

/// The backend's answer to a [`DebugBreakpoint`] request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedBreakpoint {
    /// Stable id assigned by the backend.
    pub id: i64,
    /// Whether the engine actually bound the breakpoint.
    pub verified: bool,
    /// Where the breakpoint really landed (may differ from the request).
    pub actual_position: DebugPosition,
    /// Human-readable note (e.g. "moved to next executable line").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// A function/subroutine breakpoint by name (or name pattern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugFunctionBreakpoint {
    /// Fully-qualified sub name (e.g. `My::App::dispatch`).
    pub name: String,
    /// Optional condition expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// A stack frame in a stopped thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStackFrame {
    /// Frame id, unique within the current stop.
    pub id: i64,
    /// Display name (usually the sub name or `main`).
    pub name: String,
    /// Source the frame is executing in.
    pub source: DebugSource,
    /// 1-based line number of the current instruction.
    pub line: u32,
    /// 1-based column number of the current instruction.
    pub column: u32,
}

/// A variable scope (locals, package, globals) surfaced for a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugScope {
    /// Display name of the scope (e.g. `Locals`).
    pub name: String,
    /// Handle used to fetch the scope's variables.
    pub variables_reference: VariablesRef,
    /// Whether expanding this scope is expensive (editors defer if so).
    #[serde(default)]
    pub expensive: bool,
}

/// A single variable/value pair, possibly with expandable children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugVariable {
    /// Variable name as the user would write it (e.g. `$self`).
    pub name: String,
    /// Rendered value string.
    pub value: String,
    /// Perl type/ref-kind when known (e.g. `HASH`, `ARRAY`, `My::Class`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Handle to expand children, or [`VariablesRef::NONE`] for scalars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables_reference: Option<VariablesRef>,
    /// Number of indexed (array) children, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<u64>,
    /// Number of named (hash) children, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<u64>,
}

/// A subroutine discovered in a source, used for function breakpoints and
/// the source-facts handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugFunctionSymbol {
    /// Fully-qualified sub name.
    pub name: String,
    /// Source the sub is defined in.
    pub source: DebugSource,
    /// 1-based first line of the sub body.
    pub start_line: u32,
    /// 1-based last line of the sub body.
    pub end_line: u32,
}

/// Static facts about a source that help both the IDE and an external peer
/// place breakpoints intelligently.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDebugFacts {
    /// Lines the parser considers breakable (executable) in this source.
    #[serde(default)]
    pub breakable_line_candidates: Vec<u32>,
    /// Subroutines defined in this source.
    #[serde(default)]
    pub subroutines: Vec<DebugFunctionSymbol>,
}

/// Why the debuggee stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    /// Stopped at program entry.
    Entry,
    /// Stopped after a single step.
    Step,
    /// Stopped at a source-line breakpoint.
    Breakpoint,
    /// Stopped at a function/subroutine breakpoint.
    FunctionBreakpoint,
    /// Stopped at a data/watchpoint.
    DataBreakpoint,
    /// Stopped by an exception (`die`/`warn`).
    Exception,
    /// Stopped because the user requested a pause.
    Pause,
    /// A reason we do not model explicitly; carries the raw label.
    Unknown(String),
}

/// An event emitted by a backend as the session progresses.
///
/// Backends translate their native/wire events into these; the DAP frontend
/// translates these into DAP events for the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DebugEvent {
    /// The backend finished initializing and is ready for configuration.
    Initialized,
    /// The debuggee stopped.
    Stopped {
        /// Why it stopped.
        reason: StopReason,
        /// Which thread stopped.
        thread_id: ThreadId,
        /// Where it stopped, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        position: Option<DebugPosition>,
    },
    /// The debuggee resumed execution.
    Continued {
        /// Which thread resumed.
        thread_id: ThreadId,
    },
    /// The debuggee produced output.
    Output {
        /// Output category (`stdout`, `stderr`, `console`).
        category: OutputCategory,
        /// The output text.
        output: String,
    },
    /// The session terminated.
    Terminated {
        /// Exit code, when the debuggee exited normally.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i64>,
    },
    /// New static facts became available for a source.
    SourceFacts {
        /// The source the facts describe.
        source: DebugSource,
        /// The facts.
        facts: SourceDebugFacts,
    },
    /// One or more breakpoints changed state (verified/moved/removed).
    BreakpointsChanged {
        /// The updated breakpoints.
        breakpoints: Vec<ResolvedBreakpoint>,
    },
}

/// Output stream category for [`DebugEvent::Output`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputCategory {
    /// Debuggee standard output.
    Stdout,
    /// Debuggee standard error.
    Stderr,
    /// Debugger console / informational output.
    Console,
}

impl OutputCategory {
    /// The DAP `category` string for this stream.
    #[must_use]
    pub fn as_dap_category(self) -> &'static str {
        match self {
            OutputCategory::Stdout => "stdout",
            OutputCategory::Stderr => "stderr",
            OutputCategory::Console => "console",
        }
    }
}

/// A frozen, serializable description of a debug session's inputs and known
/// facts. This is the stable handoff format an external tool (ptkdb today, any
/// engine tomorrow) can consume regardless of transport.
///
/// The schema string is versioned; consumers should reject unknown majors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSessionPacket {
    /// Schema tag, e.g. [`DebugSessionPacket::SCHEMA`].
    pub schema: String,
    /// The program under debug.
    pub program: PathBuf,
    /// Working directory for the debuggee.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// `@INC` additions.
    #[serde(default)]
    pub include_paths: Vec<PathBuf>,
    /// Source-line breakpoints.
    #[serde(default)]
    pub breakpoints: Vec<DebugBreakpoint>,
    /// Function breakpoints by exact name.
    #[serde(default)]
    pub function_breakpoints: Vec<String>,
    /// Function breakpoints by regex over sub names.
    #[serde(default)]
    pub function_breakpoint_regexes: Vec<String>,
    /// Expressions to watch/evaluate on each stop.
    #[serde(default)]
    pub watch_expressions: Vec<String>,
    /// Per-source static facts, keyed by path for deterministic ordering.
    #[serde(default)]
    pub source_facts: BTreeMap<PathBuf, SourceDebugFacts>,
}

impl DebugSessionPacket {
    /// The current session-packet schema tag.
    pub const SCHEMA: &'static str = "perl-lsp-debug-session-v1";

    /// Create an empty packet for `program` carrying the current schema tag.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            schema: Self::SCHEMA.to_string(),
            program: program.into(),
            cwd: None,
            include_paths: Vec::new(),
            breakpoints: Vec::new(),
            function_breakpoints: Vec::new(),
            function_breakpoint_regexes: Vec::new(),
            watch_expressions: Vec::new(),
            source_facts: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_source_from_path_derives_name() {
        let s = DebugSource::from_path("/work/script.pl");
        assert_eq!(s.name.as_deref(), Some("script.pl"));
        assert_eq!(s.source_reference, None);
    }

    #[test]
    fn variables_ref_none_is_not_expandable() {
        assert!(!VariablesRef::NONE.is_expandable());
        assert!(VariablesRef(7).is_expandable());
    }

    #[test]
    fn stop_reason_unknown_round_trips() {
        let r = StopReason::Unknown("watchpoint-ish".to_string());
        let json = serde_json::to_string(&r).expect("serialize");
        let back: StopReason = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn stop_reason_known_variants_are_camel_case() {
        let json = serde_json::to_string(&StopReason::FunctionBreakpoint).expect("serialize");
        assert_eq!(json, "\"functionBreakpoint\"");
    }

    #[test]
    fn session_packet_carries_schema_and_sorts_sources() {
        let mut p = DebugSessionPacket::new("/work/script.pl");
        p.source_facts.insert(PathBuf::from("/z.pl"), SourceDebugFacts::default());
        p.source_facts.insert(PathBuf::from("/a.pl"), SourceDebugFacts::default());
        let json = serde_json::to_string(&p).expect("serialize");
        assert!(json.contains(DebugSessionPacket::SCHEMA));
        // BTreeMap guarantees deterministic key ordering: /a.pl before /z.pl.
        let a = json.find("/a.pl").expect("a present");
        let z = json.find("/z.pl").expect("z present");
        assert!(a < z, "source_facts must serialize in sorted path order");
    }

    #[test]
    fn debug_event_stopped_round_trips() {
        let ev = DebugEvent::Stopped {
            reason: StopReason::Breakpoint,
            thread_id: ThreadId(1),
            position: Some(DebugPosition {
                source: DebugSource::from_path("/work/script.pl"),
                line: 42,
                column: Some(1),
            }),
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: DebugEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, back);
    }

    #[test]
    fn output_category_maps_to_dap_strings() {
        assert_eq!(OutputCategory::Stdout.as_dap_category(), "stdout");
        assert_eq!(OutputCategory::Stderr.as_dap_category(), "stderr");
        assert_eq!(OutputCategory::Console.as_dap_category(), "console");
    }
}
