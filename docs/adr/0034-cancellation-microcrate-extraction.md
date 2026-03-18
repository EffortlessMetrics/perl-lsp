# ADR-0034: Cancellation Subsystem as a Reusable Microcrate

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0008](0008-microcrate-architecture.md), [ADR-0031](0031-async-runtime-concurrent-dispatch.md), [ADR-006](ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md)

## Context

Cancellation is a cross-cutting concern in the current LSP runtime. The same token and registry
abstractions are used in at least three layers:

1. **Runtime dispatch** for request registration and `$/cancelRequest` handling.
2. **Language feature handlers** for provider-specific cancellation checks and cleanup.
3. **Public integration surface** for tests and downstream crates that need explicit access to the
   cancellation API.

The codebase already reflects this separation:

- The workspace contains a dedicated `perl-lsp-cancellation` crate.
- The `perl-lsp` crate depends on it directly.
- `perl-lsp/src/cancellation.rs` is now only a re-export shim, preserving the existing import path.

Without formalizing this as an architectural decision, future contributors could easily pull the
logic back into `perl-lsp`, duplicate token/registry types in other crates, or accidentally break
external callers that still import through `perl_lsp::cancellation`.

## Decision

**Keep the cancellation subsystem in its own published microcrate, `perl-lsp-cancellation`, and
preserve `perl-lsp` compatibility by re-exporting that API from `perl-lsp::cancellation`.**

### What moves into the microcrate

The microcrate is the canonical home for:

- `PerlLspCancellationToken`
- `CancellationRegistry`
- `ProviderCleanupContext`
- cancellation metrics and error types
- hot-path atomic cancellation checks used by runtime and providers

### Compatibility strategy

To avoid forcing a flag day across the repository, `perl-lsp` keeps a thin shim:

```rust
//! Re-exported cancellation microcrate API.
pub use perl_lsp_cancellation::*;
```

This gives the project two supported import styles during the transition:

- direct dependency on `perl-lsp-cancellation`
- legacy-compatible imports through `perl_lsp::cancellation`

### Architectural rationale

This follows the repository's SRP microcrate direction:

- cancellation logic is **protocol/runtime infrastructure**, not feature logic;
- it has a **distinct test surface** focused on concurrency and atomic behavior;
- it can evolve independently while remaining version-aligned in the workspace;
- it reduces pressure on the top-level `perl-lsp` crate to be the home of every subsystem.

## Alternatives Considered

### 1. Keep cancellation inside `perl-lsp`

Rejected because the subsystem is already shared conceptually across runtime, tests, and potential
external integrations. Keeping it embedded would make reuse harder and would cut against the
existing microcrate architecture.

### 2. Move cancellation into `perl-parser`

Rejected because cancellation is an LSP/runtime concern, not a parser concern. The parser should
remain usable without taking on JSON-RPC request identity, cleanup hooks, or request-lifecycle
bookkeeping.

### 3. Split into multiple smaller crates (`token`, `registry`, `metrics`)

Rejected for now because the subsystem is cohesive. Further splitting would add Cargo and versioning
overhead without a clear compile-time or ownership benefit.

### 4. Remove the `perl-lsp` re-export and require direct imports everywhere

Rejected because it would create unnecessary churn in existing code and external users. The re-export
keeps migration incremental and low-risk.

## Consequences

### Positive

- **Clear ownership**: cancellation logic has one canonical crate.
- **Reuse**: internal crates and external integrations can depend on the subsystem without pulling in
  the full LSP server.
- **Stable migration path**: re-export preserves existing `perl_lsp::cancellation` callers.
- **Focused testing**: atomic and concurrency-heavy tests can live with the implementation.
- **Architectural consistency**: aligns with the repository's SRP-oriented microcrate design.

### Negative / Trade-offs

- **Extra crate boundary**: simple refactors may span two crates instead of one.
- **Version coupling**: the re-exporting crate and the microcrate need to remain compatible.
- **Discoverability risk**: contributors may miss the real implementation if they only open
  `perl-lsp/src/cancellation.rs`.

### Mitigations

- Keep the shim file explicitly documented as a re-export.
- Reference this ADR from the ADR index and architecture docs.
- Prefer edits in `crates/perl-lsp-cancellation/` when changing cancellation behavior.

## Codebase Signals That Motivated This ADR

- `Cargo.toml` includes `crates/perl-lsp-cancellation` in the workspace.
- `crates/perl-lsp/Cargo.toml` depends on `perl-lsp-cancellation` directly.
- `crates/perl-lsp/src/cancellation.rs` re-exports the microcrate instead of defining types.
- runtime dispatch and provider handlers consume those shared types through the re-exported API.

## References

- `Cargo.toml` — workspace membership and shared dependency wiring
- `crates/perl-lsp/Cargo.toml` — `perl-lsp-cancellation` dependency
- `crates/perl-lsp/src/cancellation.rs` — compatibility re-export shim
- `crates/perl-lsp-cancellation/src/lib.rs` — canonical implementation
- [ADR-0008: Microcrate Architecture](0008-microcrate-architecture.md)
- [ADR-0031: Async Runtime Migration with Concurrent Dispatch](0031-async-runtime-concurrent-dispatch.md)
- [ADR-006: LSP Cancellation Infrastructure](ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md)
