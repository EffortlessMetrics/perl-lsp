//! End-to-end conformance harness for the external debugger peer seam.
//!
//! A self-contained *fake ptkdb peer* drives [`ExternalDebuggerPeerBackend`]
//! through the [Perl Debugger Peer Protocol](perl_dap::peer_protocol): handshake,
//! events (stopped/output/sourceFacts), and request/response round-trips
//! (stackTrace/scopes/variables/evaluate). This locks the host side down before
//! any real ptkdb change is requested.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use perl_dap::backend::external_peer::ExternalDebuggerPeerBackend;
use perl_dap::backend::{
    DebugBackend, EvaluateContext, EvaluateParams, InitializeBackendParams, StackTraceParams,
};
use perl_dap::model::{DebugEvent, FrameId, OutputCategory, StopReason, ThreadId, VariablesRef};
use perl_dap::peer_protocol::message::{
    PeerEvent, PeerMessage, PeerRequest, PeerResponse, command, event,
};
use perl_dap::peer_protocol::payloads::{
    EvaluateResponseBody, HelloArgs, ScopesResponseBody, StackTraceResponseBody,
    VariablesResponseBody, WireScope, WireSource, WireStackFrame, WireVariable,
};
use perl_dap::peer_protocol::{
    PROTOCOL_VERSION, PeerFrameDecoder, PeerReportedCapabilities, encode_message,
};

/// Directives the fake peer executes after the handshake, in order.
enum PeerStep {
    /// Emit an event to the host.
    Emit(PeerEvent),
    /// Register a response for a command (answered when that command arrives).
    Answer(&'static str, PeerResponse),
}

/// A fake ptkdb peer: listens, accepts one host connection, says hello, then
/// runs a script of steps + answers host requests.
struct FakePeer {
    handle: JoinHandle<()>,
    addr: std::net::SocketAddr,
}

impl FakePeer {
    fn start(caps: PeerReportedCapabilities, steps: Vec<PeerStep>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = std::thread::spawn(move || run_peer(listener, caps, steps));
        FakePeer { handle, addr }
    }
}

fn run_peer(listener: TcpListener, caps: PeerReportedCapabilities, steps: Vec<PeerStep>) {
    let (stream, _) = listener.accept().expect("accept");
    let mut write = stream.try_clone().expect("clone");
    let mut read = stream;
    let mut seq = 500i64;

    let mut send = |w: &mut TcpStream, msg: &PeerMessage| {
        let _ = w.write_all(&encode_message(msg).expect("encode"));
        let _ = w.flush();
    };

    // Handshake: peer says hello.
    seq += 1;
    send(
        &mut write,
        &PeerMessage::Request(PeerRequest {
            seq,
            command: command::HELLO.to_string(),
            arguments: serde_json::to_value(HelloArgs {
                peer: "FakePtkdb".to_string(),
                peer_version: Some("0.1".to_string()),
                protocol_version: PROTOCOL_VERSION.to_string(),
                token: None,
                capabilities: caps,
            })
            .ok(),
        }),
    );

    // Partition steps into events (fire now) and answers (fire on request).
    let mut answers: Vec<(&'static str, PeerResponse)> = Vec::new();
    for step in steps {
        match step {
            PeerStep::Emit(mut ev) => {
                seq += 1;
                ev.seq = seq;
                send(&mut write, &PeerMessage::Event(ev));
            }
            PeerStep::Answer(cmd, resp) => answers.push((cmd, resp)),
        }
    }

    // Serve host requests.
    let mut decoder = PeerFrameDecoder::new();
    let mut buf = [0u8; 4096];
    read.set_read_timeout(Some(Duration::from_millis(500))).ok();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match read.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                decoder.push(&buf[..n]);
                while let Ok(Some(msg)) = decoder.try_next() {
                    if let PeerMessage::Request(req) = msg {
                        if req.command == command::HELLO {
                            continue;
                        }
                        if let Some((_, tmpl)) = answers.iter().find(|(c, _)| *c == req.command) {
                            seq += 1;
                            let mut resp = tmpl.clone();
                            resp.seq = seq;
                            resp.request_seq = req.seq;
                            resp.command = req.command.clone();
                            send(&mut write, &PeerMessage::Response(resp));
                        }
                        if req.command == command::GOODBYE {
                            return;
                        }
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn full_caps() -> PeerReportedCapabilities {
    PeerReportedCapabilities {
        can_continue: true,
        can_step: true,
        can_pause: true,
        can_evaluate: true,
        can_set_breakpoints: true,
        can_set_function_breakpoints: true,
        can_condition_breakpoints: true,
        can_list_stack: true,
        can_list_variables: true,
        can_report_subroutines: true,
        can_report_breakable_lines: true,
        ..Default::default()
    }
}

fn ok_resp(body: Option<serde_json::Value>) -> PeerResponse {
    PeerResponse {
        seq: 0,
        request_seq: 0,
        success: true,
        command: String::new(),
        message: None,
        body,
    }
}

fn connect(peer: &FakePeer) -> ExternalDebuggerPeerBackend {
    ExternalDebuggerPeerBackend::connect(peer.addr, Duration::from_secs(5)).expect("connect")
}

/// Poll `drain_events` until it yields something or a deadline passes.
fn wait_for_event(backend: &mut ExternalDebuggerPeerBackend) -> Vec<DebugEvent> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let evs = backend.drain_events();
        if !evs.is_empty() || Instant::now() >= deadline {
            return evs;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn stopped_event_becomes_model_event() {
    let stopped = PeerEvent {
        seq: 0,
        event: event::STOPPED.to_string(),
        body: serde_json::to_value(perl_dap::peer_protocol::payloads::StoppedEventBody {
            reason: "breakpoint".to_string(),
            thread_id: 1,
            source: Some(WireSource {
                path: "/work/script.pl".to_string(),
                name: None,
                source_reference: None,
            }),
            line: Some(42),
            column: Some(1),
        })
        .ok(),
    };
    let peer = FakePeer::start(full_caps(), vec![PeerStep::Emit(stopped)]);
    let mut backend = connect(&peer);
    backend.initialize(InitializeBackendParams::default()).expect("handshake");

    let events = wait_for_event(&mut backend);
    let stopped =
        events.iter().find(|e| matches!(e, DebugEvent::Stopped { .. })).expect("a stopped event");
    match stopped {
        DebugEvent::Stopped { reason, thread_id, position } => {
            assert_eq!(*reason, StopReason::Breakpoint);
            assert_eq!(*thread_id, ThreadId(1));
            let pos = position.as_ref().expect("position");
            assert_eq!(pos.line, 42);
        }
        _ => unreachable!(),
    }
    drop(backend);
    let _ = peer.handle.join();
}

#[test]
fn output_event_forwards_category_and_text() {
    let out = PeerEvent {
        seq: 0,
        event: event::OUTPUT.to_string(),
        body: serde_json::to_value(perl_dap::peer_protocol::payloads::OutputEventBody {
            category: "stderr".to_string(),
            output: "boom\n".to_string(),
        })
        .ok(),
    };
    let peer = FakePeer::start(full_caps(), vec![PeerStep::Emit(out)]);
    let mut backend = connect(&peer);
    backend.initialize(InitializeBackendParams::default()).expect("handshake");
    let events = wait_for_event(&mut backend);
    let found = events.iter().any(|e| {
        matches!(e, DebugEvent::Output { category, output }
            if *category == OutputCategory::Stderr && output == "boom\n")
    });
    assert!(found, "output event forwarded: {events:?}");
    drop(backend);
    let _ = peer.handle.join();
}

#[test]
fn stack_scopes_variables_evaluate_round_trip() {
    let stack_body = StackTraceResponseBody {
        stack_frames: vec![WireStackFrame {
            id: 1,
            name: "main::run".to_string(),
            source: WireSource {
                path: "/work/script.pl".to_string(),
                name: Some("script.pl".to_string()),
                source_reference: None,
            },
            line: 10,
            column: 1,
        }],
    };
    let scopes_body = ScopesResponseBody {
        scopes: vec![WireScope {
            name: "Locals".to_string(),
            variables_reference: 1000,
            expensive: false,
        }],
    };
    let vars_body = VariablesResponseBody {
        variables: vec![WireVariable {
            name: "$x".to_string(),
            value: "42".to_string(),
            type_name: Some("scalar".to_string()),
            variables_reference: 0,
            indexed_variables: None,
            named_variables: None,
        }],
    };
    let eval_body = EvaluateResponseBody {
        result: "84".to_string(),
        type_name: Some("scalar".to_string()),
        variables_reference: 0,
    };

    let peer = FakePeer::start(
        full_caps(),
        vec![
            PeerStep::Answer(command::STACK_TRACE, ok_resp(serde_json::to_value(stack_body).ok())),
            PeerStep::Answer(command::SCOPES, ok_resp(serde_json::to_value(scopes_body).ok())),
            PeerStep::Answer(command::VARIABLES, ok_resp(serde_json::to_value(vars_body).ok())),
            PeerStep::Answer(command::EVALUATE, ok_resp(serde_json::to_value(eval_body).ok())),
        ],
    );
    let mut backend = connect(&peer);
    backend.initialize(InitializeBackendParams::default()).expect("handshake");

    let frames = backend
        .stack_trace(StackTraceParams { thread_id: ThreadId(1), start_frame: None, levels: None })
        .expect("stack trace");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].name, "main::run");
    assert_eq!(frames[0].line, 10);

    let scopes = backend.scopes(FrameId(1)).expect("scopes");
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].variables_reference, VariablesRef(1000));

    let vars = backend.variables(VariablesRef(1000)).expect("variables");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "$x");
    assert_eq!(vars[0].value, "42");

    let eval = backend
        .evaluate(EvaluateParams {
            expression: "$x * 2".to_string(),
            frame_id: Some(FrameId(1)),
            context: EvaluateContext::Watch,
        })
        .expect("evaluate");
    assert_eq!(eval.result, "84");

    drop(backend);
    let _ = peer.handle.join();
}

#[test]
fn source_facts_event_is_translated() {
    let facts = PeerEvent {
        seq: 0,
        event: event::SOURCE_FACTS.to_string(),
        body: serde_json::to_value(perl_dap::peer_protocol::payloads::SourceFactsEventBody {
            source: WireSource {
                path: "/work/script.pl".to_string(),
                name: None,
                source_reference: None,
            },
            breakable_lines: vec![7, 8, 12],
            subroutines: vec![],
        })
        .ok(),
    };
    let peer = FakePeer::start(full_caps(), vec![PeerStep::Emit(facts)]);
    let mut backend = connect(&peer);
    backend.initialize(InitializeBackendParams::default()).expect("handshake");
    let events = wait_for_event(&mut backend);
    let found = events.iter().any(|e| {
        matches!(e, DebugEvent::SourceFacts { facts, .. }
            if facts.breakable_line_candidates == vec![7, 8, 12])
    });
    assert!(found, "source facts translated: {events:?}");
    drop(backend);
    let _ = peer.handle.join();
}

#[test]
fn peer_crash_surfaces_as_error_not_hang() {
    // Peer says hello, then immediately drops the connection.
    let peer = FakePeer::start(full_caps(), vec![]);
    let mut backend = ExternalDebuggerPeerBackend::connect(peer.addr, Duration::from_millis(500))
        .expect("connect");
    backend.initialize(InitializeBackendParams::default()).expect("handshake");
    // Let the peer thread finish and close the socket.
    let _ = peer.handle.join();

    // A subsequent request must return an error rather than block forever.
    let err = backend
        .stack_trace(StackTraceParams { thread_id: ThreadId(1), start_frame: None, levels: None })
        .expect_err("peer gone");
    // Either NotConnected (reader saw EOF) or Timeout (race) is acceptable.
    let msg = format!("{err}");
    assert!(
        matches!(
            err,
            perl_dap::backend::BackendError::NotConnected
                | perl_dap::backend::BackendError::Timeout(_)
        ),
        "unexpected error: {msg}"
    );
}
