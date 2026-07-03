# External Debugger Peer Seam — Design Decisions & Scope

**Branch:** `claude/perl-debugger-peer-protocol-hlfa6q`
**Goal:** Make `perl-dap` a good *host* for external Perl debugger frontends/backends, with `Devel::ptkdb` as the first real partner — by exposing a small, stable, well-tested seam so a future ptkdb-side patch can be thin and not forced to swallow all of DAP.

This document records the decisions taken while building the seam, so reviewers
and future maintainers can see *why* the shape is what it is, and what was
deliberately deferred.

## Architecture landed

```
IDE / editor
   │  DAP  (crate::protocol, unchanged production path)
   ▼
perl-dap DAP frontend (DebugAdapter — unchanged)
   │
   │  canonical model  (crate::model)         ← backend-neutral vocabulary
   ▼
crate::backend::DebugBackend  (trait — the authored seam)
   ├── NativePerlDbBackend        (wraps existing DebugAdapter; catalog caps + AST breakpoints)
   ├── (LegacyBridgeBackend)      (existing BridgeAdapter path — not re-homed here)
   └── ExternalDebuggerPeerBackend  (crate::backend::external_peer)
             │
             │  Perl Debugger Peer Protocol v1  (crate::peer_protocol)
             │  Content-Length framed JSON, reusing perl_lsp_rs_core framing
             ▼
      Devel::ptkdb future PR (thin: hello + events + optional control)
```

## Decisions

### D1 — The backend contract is **model-typed**, not DAP-typed.
The whole point is that ptkdb should not implement DAP. So `DebugBackend`
speaks the canonical [`crate::model`] vocabulary (`DebugSource`,
`DebugBreakpoint`, `ResolvedBreakpoint`, `DebugStackFrame`, `DebugVariable`,
`DebugEvent`, `StopReason`, …). DAP↔model translation happens only at the DAP
frontend; peer↔model translation happens only in the external peer backend.
Reasoning from the diff alone: the existing dispatch uses
`fn(seq, request_seq, args: Option<Value>) -> DapMessage`; adopting *that* as
the backend contract would leak DAP into every backend and defeat the goal.

### D2 — Everything lands **inside `crates/perl-dap/src/`**, no new workspace crate.
The design offered `crates/perl-debug-model/` *or* inline modules. A new
workspace member touches root `Cargo.toml`, CI membership, semver tracking, and
the 39-member count. The seam is cohesive with `perl-dap` and has no other
consumer yet, so the lower-churn inline option was taken. Promotion to its own
crate is a mechanical follow-up if a second consumer appears.

### D3 — The peer transport uses **blocking `std::net` sockets + a reader thread**, not tokio.
`perl_lsp_rs_core::transport::framing` (`ContentLengthFramer`, `frame`) is a
*synchronous* byte-level API. The `DebugBackend` trait methods are synchronous.
Using blocking sockets with a dedicated reader thread and `std::sync::mpsc`
correlation keeps the peer backend fully decoupled from any ambient tokio
runtime, avoids blocking a tokio worker, and adds **zero new dependencies**. The
reader thread parses frames, routes responses to per-request oneshots and events
to a queue drained by `drain_events()`.

### D4 — Reuse the existing framing, not a new one.
Peer messages are framed with the *same* `Content-Length` family DAP uses, via
`perl_lsp_rs_core::transport::{frame, ContentLengthFramer}`. This is why the
future ptkdb side can reuse ordinary DAP-style framing libraries.

### D5 — `BreakpointOracle` is a **blanket impl over the existing `BreakpointValidator`**.
The AST validator (`AstBreakpointValidator`) is already the breakpoint truth
layer. `BreakpointOracle` is defined as a small superset trait and blanket-impl'd
so both the native path and the peer/session-packet path share one truth layer
rather than re-deriving breakable-line logic.

### D6 — Capabilities are **negotiated and intersected**, honestly.
DAP capabilities advertised to the editor are the intersection of what the
feature catalog supports and what the selected backend supports
(`intersect_dap_capabilities`). For a ptkdb peer in `mirror` mode we do **not**
claim control commands the peer did not offer. The dead `protocol::Capabilities`
struct is untouched (the live payload is the `json!` in `process.rs`); the
translation layer is additive and tested in isolation.

## Deliberately deferred (inventory, not product)

These are called out so the "done" claim is scoped honestly (closure discipline).

- **DF1 — Live dispatch migration.** Rehoming the production `dispatch_request`
  funnel onto `Box<dyn DebugBackend>` is a large, high-regression-risk refactor
  (spec "PR 2"). The seam is instead *proven by tests* (mock backend + fake
  ptkdb peer conformance harness). The existing native DAP path is untouched, so
  no current behavior regresses. Migration is its own follow-up.
- **DF2 — End-to-end editor↔peer live session — NOW CLOSED (socket path).**
  A DAP frontend over the backend (`crate::backend::peer_bridge::DapPeerBridge`)
  translates DAP requests → model calls → DAP responses and pumps
  `drain_events()` → DAP events, driven by `run_external_peer_session` over a
  socket editor connection and reachable from the binary via
  `perl-dap --socket --port N --external-peer HOST:PORT`. Proven end-to-end by
  `tests/peer_bridge_e2e.rs`: a real `ExternalDebuggerPeerBackend` connected to a
  fake ptkdb peer is driven through a full DAP session (initialize → setBreakpoints
  → continue → the peer's `debugger/stopped` surfaces as a DAP `stopped` event →
  stackTrace → disconnect), plus a socket-transport driver test. This is a
  **parallel** path — the native `DapServer`/`DebugAdapter` dispatch funnel is
  untouched (DF1 stays deferred). Both editor transports are now covered:
  `run_external_peer_session` (socket) and `run_external_peer_session_stdio`
  (stdin/stdout, via a reader thread + channel so async events interleave without
  a stdin read timeout). `perl-dap --external-peer HOST:PORT` uses stdio by
  default and the socket path when `--socket`/`--port` is given. The VS Code
  extension passes it through: a debug config with `externalPeer: "HOST:PORT"`
  launches the adapter in bridge mode (`buildDapExecutableArgs`), and an
  "External Debugger Peer (ptkdb)" launch.json template + wizard entry make it
  discoverable. *Residual:* validation against a live `Devel::ptkdb` build (vs.
  the faithful fake peer) remains a follow-up.
- **DF3 — `NativePerlDbBackend` full delegation.** The native backend implements
  the model-typed contract for the surface that does not require a live `perl -d`
  process (capabilities from the catalog, AST-backed `set_breakpoints`), and
  documents the delegation path for process-dependent methods. A full live
  delegate is gated on the DF1 migration.
- **DF4 — `cooperative`/`dapControlled` control modes.** Only `mirror` is fully
  exercised end-to-end (the friendliest first integration). The other modes are
  modeled in types and negotiated, but their bidirectional control paths are
  future work.

## Adversarial-review hardening

An independent correctness/concurrency review (seam-anchored, opposite direction
to the producer) confirmed the handshake Condvar, seq numbering, event draining,
and frame handling are sound, and surfaced three real defects — all fixed with
test coverage:

- **Write timeout (was: a stalled-but-open peer could block `write_all` under the
  write mutex, wedging `request()` and `Drop::join()`).** The write half now sets
  `set_write_timeout`, and any write failure calls `mark_closed()` so subsequent
  ops fail fast — honoring the documented "never hangs" guarantee even when the
  peer stops draining (flow control, not a clean close).
- **Protocol-version validation (was: `peer/hello` accepted any version).** The
  handshake now rejects a mismatched `protocolVersion` with `success: false` and
  surfaces a clear `BackendError::Protocol` to `initialize()` instead of an
  opaque timeout.
- **Pause capability honesty (was: `pause` inferred from `can_step`).** Added a
  dedicated `canPause` peer capability; a peer that can step but did not advertise
  async pause is never sent a `debugger/pause`.

A later automated review (Codex, on the bridge/reachability layer) surfaced three
more claim-vs-reality gaps — all fixed with test coverage:

- **Resume-without-step honesty (was: `continue` gated on `stepping`).** A peer
  advertising `canContinue` but not `canStep` was wrongly refused DAP `continue`.
  Added a dedicated `continue_execution` backend capability mapped from
  `can_continue`; `continue_thread` now gates on it, independent of `stepping`.
- **`terminate` actually handled (was: advertised, then swallowed).** The bridge
  advertised `supportsTerminateRequest` but had no `terminate` dispatch arm, so a
  client's Stop fell through the lenient ack without disconnecting the peer. Added
  a `terminate` arm that calls `disconnect(true)` and emits a `terminated` event.
- **VS Code config actually drives the bridge (was: two divergent shapes).** The
  shipped `debuggerBackend: "external"` + `externalDebugger` config was ignored by
  the descriptor (which only read a flat `externalPeer` string), so selecting it
  ran the native adapter. `buildDapExecutableArgs` now resolves both shapes; the
  shipped ptkdb config uses the implemented `connect` mode + a concrete port, and
  `listen`/`launchPeer`/`port: 0` (not yet wired) fall back rather than fabricate
  an unconnectable address.

## Closure receipt

- repo: `effortlessmetrics/perl-lsp-swarm`
- production_entrypoint: additive modules under `crates/perl-dap/src/`; the live
  DAP production path (`DebugAdapter`/`DapServer`) is **unchanged**.
- independent_expected_behavior: peer protocol + backend proven against an
  in-repo fake ptkdb peer conformance harness (handshake, capability
  negotiation, breakpoints, stopped/output events, stack/scopes/variables/
  evaluate, timeout, mirror-mode capability honesty, peer crash).
- user_visible_effect: **none yet** for end users (DF2) — this is the host-side
  seam. The user-visible partner integration is the future ptkdb PR.
- fallback_remaining: `.ptkdbrc` bootstrap renderer + session-packet emitter are
  the working, shippable surfaces today.
- uncertainty: real `Devel::ptkdb` conformance is validated against the protocol
  spec + a faithful fake peer, not against a live ptkdb build.
