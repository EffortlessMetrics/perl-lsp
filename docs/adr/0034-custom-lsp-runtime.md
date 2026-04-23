# ADR-0034: Custom LSP Runtime over Framework Adoption

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0031](0031-async-runtime-concurrent-dispatch.md), [ADR-0016](0016-feature-governance.md), [ADR-0019](0019-security-first-dap.md), [CUSTOM_LSP_RUNTIME.md](../project/CUSTOM_LSP_RUNTIME.md)

## Context

`perl-lsp` implements its own protocol, transport, cancellation, and dispatch stack instead of
building on a general-purpose Rust LSP framework such as `tower-lsp`. This decision is visible in
both the workspace structure and the server entrypoints:

- dedicated runtime crates such as `perl-lsp-protocol`, `perl-lsp-transport`,
  `perl-content-length-framing`, and `perl-lsp-cancellation`
- direct `serve_async()` dispatch in `crates/perl-lsp-rs/src/main.rs`
- explicit scheduler, outbound writer, and cancellation routing modules under
  `crates/perl-lsp-rs/src/runtime/`

This architecture is apparent in the codebase, but until now it was primarily described in project
narrative documentation rather than recorded as an ADR.

### Problem Statement

The project needed an LSP runtime that could satisfy four constraints at the same time:

1. **Capability governance**: feature profiles and catalog-driven capability advertisement must be a
   first-class runtime concern.
2. **Transport control**: stdio and TCP should share the same framing and dispatch path.
3. **Cancellation and responsiveness**: the runtime must support explicit request classification,
   inline cancellation handling, and later bounded concurrency.
4. **Cross-protocol reuse**: content-length framing and related runtime pieces should be reusable by
   the DAP stack.

General-purpose frameworks reduce boilerplate, but they also impose lifecycle, dispatch, and
capability-advertisement models that do not align cleanly with these constraints.

## Decision

**The project will continue to use a bespoke LSP runtime composed of focused microcrates and local
runtime modules, rather than adopting `tower-lsp` or a similar framework as the core server
architecture.**

### Chosen Architecture

The runtime is split across intentionally small components:

| Layer | Primary components | Responsibility |
|------|---------------------|----------------|
| Protocol | `perl-lsp-protocol` | JSON-RPC/LSP message types, method constants, errors, capabilities |
| Framing | `perl-content-length-framing`, `perl-lsp-transport` | Content-Length parsing, serialization, stdio/TCP transport glue |
| Cancellation | `perl-lsp-cancellation` | request tokens, registry, cleanup, hot-path cancellation checks |
| Server runtime | `crates/perl-lsp-rs/src/runtime/*` | ingress classification, scheduler, outbound writer, lifecycle routing |
| Launch/config | `perl-lsp-launcher` | CLI parsing, transport selection, feature-profile selection |

### Why This Was Chosen

1. **Feature governance is architectural, not incidental.**
   The server's advertised capability set depends on feature profiles, catalog metadata, and build
   flags. That coupling is easier to express directly in the runtime than through framework
   abstractions.

2. **The project wants explicit dispatch semantics.**
   The runtime classifies requests into control, mutation/lifecycle, and read-only lanes. This made
   it possible to evolve from the original synchronous design to the bounded-concurrency scheduler
   recorded in ADR-0031 without replacing the protocol stack.

3. **Transport and framing are shared infrastructure.**
   Content-Length framing is used beyond the LSP binary, so a project-owned implementation provides
   reuse that a framework-embedded transport layer would not.

4. **Security and error policy are easier to enforce locally.**
   The codebase's no-panic production policy, input validation, and cancellation cleanup model are
   implemented directly in project-owned layers rather than adapted around framework behavior.

## Alternatives Considered

### Option 1: Adopt `tower-lsp`

**Pros**:
- Less boilerplate for JSON-RPC dispatch and handler wiring
- Conventional ecosystem choice
- Built-in async handler model

**Cons**:
- Capability advertisement model is less aligned with profile-based governance
- Less direct control over transport/framing reuse across LSP and DAP
- Harder to keep request classification and cancellation policy explicit in project terms
- Would require adapting large amounts of already-stable project infrastructure

**Decision**: Rejected as the primary runtime architecture.

### Option 2: Adopt a lower-level generic server crate for transport/dispatch only

**Pros**:
- Could reduce some framing/dispatch maintenance burden
- Leaves higher-level semantics inside the project

**Cons**:
- Still introduces an external abstraction boundary in the most performance- and policy-sensitive
  path
- Reduces reuse between LSP and DAP unless the abstraction also matches both protocols
- Would offer only partial simplification while still requiring substantial local glue

**Decision**: Rejected for now. Revisit only if maintenance cost materially outweighs control.

### Option 3: Continue with bespoke runtime microcrates

**Pros**:
- Full control over capabilities, transport, cancellation, and scheduling
- Reuse of framing and transport infrastructure across protocols
- Straightforward integration with feature governance and server-specific policies

**Cons**:
- More code to maintain in-house
- More architectural surface area for contributors to learn
- Requires explicit documentation to prevent the design from becoming tribal knowledge

**Decision**: Accepted.

## Consequences

### Positive

- **Architectural coherence**: protocol, transport, cancellation, governance, and scheduling use the
  same vocabulary and boundaries.
- **Evolutionary flexibility**: ADR-0031's concurrent dispatch could be layered on top of the
  existing runtime instead of requiring a framework migration.
- **Cross-protocol reuse**: content-length framing remains reusable by the DAP stack and other
  protocol-facing components.
- **Operational clarity**: stdio and TCP both flow through the same `serve_async()` path.

### Negative / Trade-offs

- **Maintenance burden**: the project owns transport and dispatch code that frameworks would usually
  supply.
- **Documentation burden**: contributors need ADRs and architecture docs to understand why a custom
  runtime exists.
- **Reconsideration threshold**: if feature-governance needs, transport reuse, or cancellation
  control become less important than maintenance simplicity, this decision should be revisited.

## Revisit Triggers

Review this ADR if any of the following become true:

- the project plans to replace feature-profile-based capability governance
- the DAP/LSP transport layers no longer share framing infrastructure
- runtime maintenance cost becomes a recurring source of defects or delivery slowdown
- a framework can support current governance, transport, and scheduling requirements without major
  adaptation layers

## References

- `crates/perl-lsp-rs/src/main.rs`
- `crates/perl-lsp-rs/src/runtime/serving.rs`
- `crates/perl-lsp-rs/src/runtime/scheduler.rs`
- `crates/perl-lsp-rs/src/runtime/outbound.rs`
- `docs/project/CUSTOM_LSP_RUNTIME.md`
