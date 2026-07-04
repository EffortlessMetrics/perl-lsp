//! End-to-end reachability proof for the external-peer seam.
//!
//! Drives a real [`ExternalDebuggerPeerBackend`] (connected to a fake ptkdb
//! peer) through the [`DapPeerBridge`] using actual DAP requests, and asserts
//! that:
//! - DAP requests reach the peer over the Perl Debugger Peer Protocol and their
//!   responses come back as DAP responses, and
//! - an asynchronous peer `debugger/stopped` event surfaces to the editor as a
//!   DAP `stopped` event.
//!
//! This closes the Reachability axis: the peer backend is live-drivable from a
//! real DAP session, not just component-tested.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use perl_dap::backend::external_peer::ExternalDebuggerPeerBackend;
use perl_dap::backend::{DapPeerBridge, run_external_peer_session};
use perl_dap::debug_adapter::DapMessage;
use perl_dap::peer_protocol::message::{
    PeerEvent, PeerMessage, PeerRequest, PeerResponse, command, event,
};
use perl_dap::peer_protocol::payloads::{
    HelloArgs, SetBreakpointsResponseBody, StackTraceResponseBody, StoppedEventBody,
    WireResolvedBreakpoint, WireSource, WireStackFrame,
};
use perl_dap::peer_protocol::{
    PROTOCOL_VERSION, PeerFrameDecoder, PeerReportedCapabilities, encode_message,
};
use perl_lsp_rs_core::transport::{ContentLengthFramer, frame};

/// A fake ptkdb peer: connects to `addr`, sends hello, answers debugger/*
/// requests, and — on `debugger/continue` — emits a `debugger/stopped` event.
fn spawn_fake_peer(addr: std::net::SocketAddr) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let stream = match TcpStream::connect(addr) {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut write = stream.try_clone().expect("clone");
        let mut read = stream;
        let mut seq = 900i64;

        let caps = PeerReportedCapabilities {
            can_continue: true,
            can_step: true,
            can_pause: true,
            can_evaluate: true,
            can_set_breakpoints: true,
            can_condition_breakpoints: true,
            can_list_stack: true,
            can_list_variables: true,
            ..Default::default()
        };
        seq += 1;
        let hello = PeerMessage::Request(PeerRequest {
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
        });
        let _ = write.write_all(&encode_message(&hello).expect("enc"));

        let send = |w: &mut TcpStream, m: &PeerMessage| {
            let _ = w.write_all(&encode_message(m).expect("enc"));
            let _ = w.flush();
        };

        let mut decoder = PeerFrameDecoder::new();
        let mut buf = [0u8; 4096];
        read.set_read_timeout(Some(Duration::from_millis(400))).ok();
        let deadline = Instant::now() + Duration::from_secs(6);
        while Instant::now() < deadline {
            match read.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    decoder.push(&buf[..n]);
                    while let Ok(Some(msg)) = decoder.try_next() {
                        let PeerMessage::Request(req) = msg else { continue };
                        if req.command == command::HELLO {
                            continue;
                        }
                        seq += 1;
                        let body = match req.command.as_str() {
                            command::SET_BREAKPOINTS => {
                                serde_json::to_value(SetBreakpointsResponseBody {
                                    breakpoints: vec![WireResolvedBreakpoint {
                                        id: 1,
                                        verified: true,
                                        line: 42,
                                        column: None,
                                        message: None,
                                    }],
                                })
                                .ok()
                            }
                            command::STACK_TRACE => serde_json::to_value(StackTraceResponseBody {
                                stack_frames: vec![WireStackFrame {
                                    id: 1,
                                    name: "main::run".to_string(),
                                    source: WireSource {
                                        path: "/work/script.pl".to_string(),
                                        name: None,
                                        source_reference: None,
                                    },
                                    line: 42,
                                    column: 1,
                                }],
                            })
                            .ok(),
                            _ => None,
                        };
                        send(
                            &mut write,
                            &PeerMessage::Response(PeerResponse {
                                seq,
                                request_seq: req.seq,
                                success: true,
                                command: req.command.clone(),
                                message: None,
                                body,
                            }),
                        );

                        // After a continue, the debuggee "hits a breakpoint".
                        if req.command == command::CONTINUE {
                            seq += 1;
                            send(
                                &mut write,
                                &PeerMessage::Event(PeerEvent {
                                    seq,
                                    event: event::STOPPED.to_string(),
                                    body: serde_json::to_value(StoppedEventBody {
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
                                }),
                            );
                        }
                        if req.command == command::DISCONNECT {
                            return;
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
    })
}

fn response_body<'a>(msg: &'a DapMessage, expect_cmd: &str) -> &'a serde_json::Value {
    match msg {
        DapMessage::Response { command, success, body, .. } => {
            assert_eq!(command, expect_cmd, "unexpected response command");
            assert!(success, "response for {expect_cmd} was not successful: {msg:?}");
            body.as_ref().unwrap_or_else(|| panic!("{expect_cmd} had no body"))
        }
        _ => panic!("expected a response to {expect_cmd}, got {msg:?}"),
    }
}

fn find_event<'a>(msgs: &'a [DapMessage], name: &str) -> Option<&'a DapMessage> {
    msgs.iter().find(|m| matches!(m, DapMessage::Event { event, .. } if event == name))
}

#[test]
fn full_dap_session_drives_the_live_peer_backend() {
    // Fake ptkdb peer listens; the backend connects to it.
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let addr = listener.local_addr().expect("addr");
    let peer = spawn_fake_peer(addr);
    let (stream, _) = listener.accept().expect("accept");
    let backend =
        ExternalDebuggerPeerBackend::from_connected_stream(stream, Duration::from_secs(5))
            .expect("backend");

    let mut bridge = DapPeerBridge::new(Box::new(backend));

    // initialize → capabilities + initialized event (also completes the handshake).
    let out = bridge.dispatch(1, "initialize", Some(serde_json::json!({ "adapterID": "perl" })));
    let caps = response_body(&out[0], "initialize");
    assert_eq!(caps["supportsConditionalBreakpoints"], true, "ptkdb peer negotiated conditions");
    assert_eq!(caps["supportsLogPoints"], false, "ptkdb v1 has no logpoints");
    assert!(find_event(&out, "initialized").is_some(), "initialized event emitted");

    // setBreakpoints → reaches the peer, resolved breakpoints come back.
    let out = bridge.dispatch(
        2,
        "setBreakpoints",
        Some(serde_json::json!({
            "source": { "path": "/work/script.pl" },
            "breakpoints": [{ "line": 42, "condition": "$x > 10" }],
        })),
    );
    let body = response_body(&out[0], "setBreakpoints");
    assert_eq!(body["breakpoints"][0]["id"], 1);
    assert_eq!(body["breakpoints"][0]["verified"], true);
    assert_eq!(body["breakpoints"][0]["line"], 42);

    // continue → response, and the peer asynchronously emits a stopped event.
    let out = bridge.dispatch(3, "continue", Some(serde_json::json!({ "threadId": 1 })));
    assert_eq!(response_body(&out[0], "continue")["allThreadsContinued"], true);

    // Accumulate events (DapMessage is not Clone) and poll until the peer's
    // stopped event surfaces as a DAP stopped event.
    let mut acc: Vec<DapMessage> = out;
    let deadline = Instant::now() + Duration::from_secs(3);
    while find_event(&acc, "stopped").is_none() && Instant::now() < deadline {
        acc.extend(bridge.poll_events());
        if find_event(&acc, "stopped").is_none() {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    let stopped = find_event(&acc, "stopped").expect("peer stopped event surfaced as DAP stopped");
    if let DapMessage::Event { body: Some(b), .. } = stopped {
        assert_eq!(b["reason"], "breakpoint");
        assert_eq!(b["threadId"], 1);
        assert_eq!(b["allThreadsStopped"], true);
    } else {
        panic!("stopped event had no body");
    }

    // stackTrace → proxied to the peer and returned as a DAP stack.
    let out = bridge.dispatch(4, "stackTrace", Some(serde_json::json!({ "threadId": 1 })));
    let frames = &response_body(&out[0], "stackTrace")["stackFrames"];
    assert_eq!(frames[0]["name"], "main::run");
    assert_eq!(frames[0]["line"], 42);

    // disconnect → clean teardown (a bodyless successful response).
    let out =
        bridge.dispatch(5, "disconnect", Some(serde_json::json!({ "terminateDebuggee": false })));
    match &out[0] {
        DapMessage::Response { command, success, .. } => {
            assert_eq!(command, "disconnect");
            assert!(success, "disconnect should succeed");
        }
        other => panic!("expected disconnect response, got {other:?}"),
    }

    drop(bridge);
    let _ = peer.join();
}

#[test]
fn run_external_peer_session_serves_dap_over_a_socket() {
    // Fake ptkdb peer (peer side) and an editor listener (editor side).
    let peer_listener = TcpListener::bind(("127.0.0.1", 0)).expect("peer bind");
    let peer_addr = peer_listener.local_addr().expect("peer addr");
    let peer = spawn_fake_peer(peer_addr);

    let editor_listener = TcpListener::bind(("127.0.0.1", 0)).expect("editor bind");
    let editor_addr = editor_listener.local_addr().expect("editor addr");

    // Server side: accept the peer, build the bridge, accept the editor, run.
    let server = std::thread::spawn(move || {
        let (peer_stream, _) = peer_listener.accept().expect("accept peer");
        let backend =
            ExternalDebuggerPeerBackend::from_connected_stream(peer_stream, Duration::from_secs(5))
                .expect("backend");
        let bridge = DapPeerBridge::new(Box::new(backend));
        let (editor, _) = editor_listener.accept().expect("accept editor");
        let _ = run_external_peer_session(editor, bridge, Duration::from_millis(50));
    });

    // Editor client: send framed DAP requests, read framed responses.
    let mut client = TcpStream::connect(editor_addr).expect("editor connect");
    client.set_read_timeout(Some(Duration::from_secs(3))).ok();
    let send = |c: &mut TcpStream, v: &serde_json::Value| {
        let body = serde_json::to_vec(v).expect("ser");
        c.write_all(&frame(&body)).expect("write");
        c.flush().expect("flush");
    };

    send(
        &mut client,
        &serde_json::json!({ "seq": 1, "type": "request", "command": "initialize", "arguments": { "adapterID": "perl" } }),
    );

    // Read framed messages until we see the initialize response.
    let mut framer = ContentLengthFramer::new();
    let mut buf = [0u8; 4096];
    let mut saw_initialize = false;
    let deadline = Instant::now() + Duration::from_secs(3);
    'outer: while Instant::now() < deadline {
        let n = match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        framer.push(&buf[..n]);
        while let Ok(Some(body)) = framer.try_next() {
            let v: serde_json::Value = serde_json::from_slice(&body).expect("json");
            if v["type"] == "response" && v["command"] == "initialize" {
                assert_eq!(v["success"], true);
                assert_eq!(v["body"]["supportsConditionalBreakpoints"], true);
                saw_initialize = true;
                break 'outer;
            }
        }
    }
    assert!(saw_initialize, "editor received the initialize response over the socket");

    // Clean teardown.
    send(
        &mut client,
        &serde_json::json!({ "seq": 2, "type": "request", "command": "disconnect", "arguments": {} }),
    );
    drop(client);
    let _ = server.join();
    let _ = peer.join();
}

#[test]
fn socket_session_recovers_from_a_leading_malformed_frame() {
    // A malformed frame arriving before a valid request must NOT tear down the
    // whole socket session: the framer discards just the bad header block, and
    // the driver keeps parsing the well-formed `initialize` that follows.
    let peer_listener = TcpListener::bind(("127.0.0.1", 0)).expect("peer bind");
    let peer_addr = peer_listener.local_addr().expect("peer addr");
    let peer = spawn_fake_peer(peer_addr);

    let editor_listener = TcpListener::bind(("127.0.0.1", 0)).expect("editor bind");
    let editor_addr = editor_listener.local_addr().expect("editor addr");

    let server = std::thread::spawn(move || {
        let (peer_stream, _) = peer_listener.accept().expect("accept peer");
        let backend =
            ExternalDebuggerPeerBackend::from_connected_stream(peer_stream, Duration::from_secs(5))
                .expect("backend");
        let bridge = DapPeerBridge::new(Box::new(backend));
        let (editor, _) = editor_listener.accept().expect("accept editor");
        let _ = run_external_peer_session(editor, bridge, Duration::from_millis(50));
    });

    let mut client = TcpStream::connect(editor_addr).expect("editor connect");
    client.set_read_timeout(Some(Duration::from_secs(3))).ok();

    // A syntactically-framed header with a non-numeric Content-Length: the framer
    // returns an error but drains the bad block, so parsing can continue.
    client.write_all(b"Content-Length: notanumber\r\n\r\n").expect("write bad frame");
    // Then a valid initialize request.
    let body = serde_json::to_vec(
        &serde_json::json!({ "seq": 1, "type": "request", "command": "initialize", "arguments": { "adapterID": "perl" } }),
    )
    .expect("ser");
    client.write_all(&frame(&body)).expect("write");
    client.flush().expect("flush");

    let mut framer = ContentLengthFramer::new();
    let mut buf = [0u8; 4096];
    let mut saw_initialize = false;
    let deadline = Instant::now() + Duration::from_secs(3);
    'outer: while Instant::now() < deadline {
        let n = match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        framer.push(&buf[..n]);
        while let Ok(Some(body)) = framer.try_next() {
            let v: serde_json::Value = serde_json::from_slice(&body).expect("json");
            if v["type"] == "response" && v["command"] == "initialize" {
                assert_eq!(v["success"], true);
                saw_initialize = true;
                break 'outer;
            }
        }
    }
    assert!(saw_initialize, "session survived the malformed frame and answered initialize");

    drop(client);
    let _ = server.join();
    let _ = peer.join();
}
