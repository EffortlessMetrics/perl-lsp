# How to write a Perl debug backend

`perl-dap` drives debugger engines through the `perl_dap::backend::DebugBackend`
trait, expressed in the canonical, backend-neutral `perl_dap::model` vocabulary
(never DAP wire types). This guide shows how to add a new backend.

## The contract

```rust
use perl_dap::backend::{DebugBackend, DebugBackendCapabilities, BackendResult, /* param/result types */};
use perl_dap::model::{/* DebugSource, DebugBreakpoint, ResolvedBreakpoint, DebugEvent, ... */};

pub struct MyBackend { /* engine handle */ }

impl DebugBackend for MyBackend {
    fn name(&self) -> &str { "my-backend" }
    fn capabilities(&self) -> DebugBackendCapabilities { /* what your engine can do */ }
    fn initialize(&mut self, params: /*..*/) -> BackendResult<()> { /* ... */ }
    // launch / attach / set_breakpoints / continue_thread / next / step_in /
    // step_out / pause / stack_trace / scopes / variables / evaluate / disconnect
    fn drain_events(&mut self) -> Vec<DebugEvent> { /* async engine events */ }
    // ...
}
```

Everything is in terms of the model. If your engine speaks some other
representation (a wire protocol, a native API), translate at the boundary — the
rest of `perl-dap` never sees your engine's shape.

## Rules that make a backend well-behaved

1. **Report capabilities honestly.** `capabilities()` is intersected with the
   feature catalog to decide what the editor is told
   (`perl_dap::backend::intersect_dap_capabilities`). Never claim a capability you
   cannot deliver — the editor will surface a control the user cannot use.

2. **Guard against un-negotiated calls.** If a method is invoked for a capability
   you did not advertise, return `BackendError::Unsupported(...)` rather than
   panicking or faking a result.

3. **Never block forever.** Any call that waits on an external engine must have a
   timeout and return `BackendError::Timeout(..)`. A dropped/So-crashed engine
   must surface as `BackendError::NotConnected` or `Transport`, not a hang.

4. **No banned patterns.** Production code must not use `unwrap`, `expect`,
   `panic!`, `todo!`, or `unimplemented!` (see `/coding-standards`). Recover from
   lock poisoning with `PoisonError::into_inner` rather than unwrapping.

5. **Surface async events through `drain_events`.** Stops, output, and
   termination that originate from the engine (not a request/response) are queued
   and drained non-blockingly by the frontend.

## Two worked examples in-tree

- **`perl_dap::backend::external_peer::ExternalDebuggerPeerBackend`** — a socket
  peer backend. Shows the full pattern: blocking `std::net` transport with a
  reader thread, `std::sync::mpsc` request/response correlation, capability
  negotiation from a handshake, and wire↔model translation. This is the reference
  implementation.

- **`perl_dap::backend::native_perldb::NativePerlDbBackend`** — wraps the existing
  native `perl -d` adapter, translating model calls to the adapter's DAP request
  handlers.

## Testing a backend

Drive it with an in-process fake engine. For a socket backend, bind an ephemeral
`TcpListener` as the fake peer, have the backend `connect` to it, and script the
peer's responses/events. See `crates/perl-dap/tests/external_peer_conformance.rs`
for a complete fake-peer harness covering handshake, events, and
request/response round-trips.

## Capability negotiation reference

`DebugBackendCapabilities` fields map to DAP capabilities via
`intersect_dap_capabilities(catalog, backend)`; the negotiated flags are what the
editor sees. Keep your `capabilities()` in sync with what your methods actually
implement — a test that asserts the two agree is worth writing.
