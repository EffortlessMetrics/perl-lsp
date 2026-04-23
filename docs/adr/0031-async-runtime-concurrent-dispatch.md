# ADR-0031: Async Runtime Migration with Concurrent Dispatch

**Status**: Accepted
**Date**: 2026-03-16
**Decision Makers**: Perl LSP Architecture Team
**Related**: PR #1555, [LSP Implementation Guide](../reference/LSP_IMPLEMENTATION_GUIDE.md), [ADR-0018](0018-adaptive-threading-tests.md)

## Context

The perl-lsp server originally used a synchronous, single-threaded request handler where all
LSP method handlers took `&mut self` on `LspServer`. This design worked correctly but created
two problems:

1. **Responsiveness**: Slow requests (workspace indexing, large completions) blocked the main
   loop, making the server unresponsive to other requests including `$/cancelRequest`.
2. **Scalability**: A single thread cannot serve concurrent read-only requests (hover, goto-def,
   completions) that could safely run in parallel since they only read shared state.

### Prior Architecture

```
stdin → ingress loop → dispatch(&mut self) → stdout
```

All requests ran sequentially on one thread. `$/cancelRequest` was queued behind whatever
long-running request it was trying to cancel.

### Alternatives Considered

1. **Keep synchronous, add timeouts**: Add per-request timeouts with a watchdog thread. Rejected
   because timeouts hide latency rather than fix it, and cancellation still cannot be inline.

2. **One thread per request**: Spawn an OS thread per request. Rejected because unbounded thread
   creation is a DoS vector for malicious language clients and wastes resources for short requests.

3. **Tokio async throughout**: Migrate all handler code to async/await. Rejected at this stage
   because async Rust requires either `async` propagation throughout the call tree or explicit
   `block_in_place`, and the parser/indexer code is deeply synchronous CPU-bound work not suited
   to async.

4. **Chosen: typed worker queues with bounded thread pools** (see Decision).

## Decision

Migrate `LspServer` from `&mut self` handlers to `&self` (shared reference) handlers backed by
interior mutability, and route requests through a two-queue scheduler:

### Phase 0 — Interior Mutability

Replace mutable state fields on `LspServer` with:
- `AtomicBool` for lifecycle flags (initialized, shutdown)
- `Mutex<ClientCapabilities>` for negotiated client state

This makes `LspServer: Sync` without unsafe code for those fields.

### Phase 1 — Outbound Channel

Extract stdout writes to a dedicated `outbound.rs` channel. Handlers no longer write directly
to stdout; they send `OutboundMessage` values through an `mpsc` sender. A single writer thread
owns `stdout`. This eliminates interleaved output without requiring a mutex on every write.

### Phase 2 — Scheduler

New `scheduler.rs` implements a two-lane dispatch model:

| Lane | Workers | Request Types |
|------|---------|---------------|
| **Exclusive** | 1 | `initialize`, `shutdown`, and any `&mut`-requiring mutation |
| **Read pool** | 4 | All read-only requests (hover, goto-def, completion, etc.) |

The ingress loop classifies each incoming message and enqueues it to the appropriate lane.
`$/cancelRequest` is processed **inline in the ingress loop** before enqueuing, so cancellation
has zero queue latency.

```text
stdin → ingress loop → classify → exclusive queue (1 worker)
                               ↘ read pool (4 workers)
                                              ↓
                                    outbound channel → stdout
```

### Phase 3 — Centralized Unsafe

`LspServer` contains `ParentMap` which holds raw pointers into AST nodes. These cannot be made
`Send`/`Sync` by the compiler. Rather than scatter `unsafe` blocks or work around raw pointers
(which are performance-critical for AST traversal), the decision is to add a single
`unsafe impl Send for LspServer` and `unsafe impl Sync for LspServer` behind `Arc<Mutex>` on the
`ParentMap` field. This is a **centralised, documented** unsafe contract rather than ad-hoc casts.

The safety invariant: `ParentMap` pointers are only accessed while the `Arc<Mutex>` lock is held,
and the AST they point into is not dropped while any pointer lives.

### Phase 4 — Notification Batching

The writer thread coalesces burst writes from the read pool: when multiple `OutboundMessage`
values are queued simultaneously, they are flushed in a single `write_all` call to reduce
syscall overhead on large workspace notifications (e.g., `textDocument/publishDiagnostics` after
indexing).

### Unified stdio/TCP Paths

Both stdio and TCP modes now enter the same `serve_async()` entry point, with blocking reader
threads bridging synchronous I/O to the async scheduler. This removes a maintenance split where
TCP mode previously had a separate handler path.

## Consequences

### Positive

- **Cancel responsiveness**: `$/cancelRequest` is processed inline before any queued work runs.
- **Concurrent reads**: Up to 4 hover/goto-def/completion requests can execute simultaneously.
  On a 4-core machine this eliminates the common "slow hover blocks fast goto-def" symptom.
- **Unified I/O paths**: stdio and TCP share one code path, halving the surface area for I/O bugs.
- **Notification coalescing**: Burst diagnostic publishes are batched, reducing churn during
  heavy indexing.

### Negative / Trade-offs

- **Test signature debt**: ~150 test call sites still use `&mut LspServer` signatures. These
  produce clippy warnings (not errors) and do not affect correctness, but are tracked as
  follow-up work.
- **Unsafe contract**: `unsafe impl Send/Sync` for `LspServer` is now present. The invariant is
  documented and localized, but requires future maintainers to understand the `ParentMap` raw
  pointer model before modifying `LspServer` field layout.
- **4-worker cap**: The read pool is capped at 4 workers. Very large monorepos with many
  simultaneous file opens may saturate the pool. The cap is configurable in `scheduler.rs` but
  not yet exposed as an LSP initialization option.

### Follow-up Work

| Item | Tracking |
|------|---------|
| Clean up `&mut LspServer` test helper signatures (~150 sites, ~26 files) | Task #6 |
| Replace `unsafe impl Send/Sync` with index-based `ParentMap` | Future ADR |
| Async test harness for concurrency verification | Future work |
| Expose worker pool size as LSP initialization option | Future work |
| Benchmarking: concurrent read throughput, cancellation latency | Future work |

## References

- PR #1555: `feat(lsp): async runtime migration with concurrent dispatch`
- [ADR-0018: Adaptive Threading for Tests](0018-adaptive-threading-tests.md) — test-side threading constraints
- [ADR-0012: Error Handling Strategy](0012-error-handling-strategy.md) — no-panic production code policy
