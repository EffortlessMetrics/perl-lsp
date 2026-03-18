# ADR-0034: Custom LSP Runtime and Protocol Stack

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0012](0012-error-handling-strategy.md), [ADR-0016](0016-feature-governance.md), [ADR-0021](0021-lsp-capability-contract.md), [ADR-0031](0031-async-runtime-concurrent-dispatch.md), [Custom LSP Runtime](../project/CUSTOM_LSP_RUNTIME.md)

## Context

The codebase implements its own LSP runtime instead of depending on `tower-lsp` or another
framework-managed server stack. That decision is visible in multiple places:

- `crates/perl-lsp/src/runtime/serving.rs` owns the ingress loop and request classification.
- `crates/perl-lsp/src/runtime/routing.rs` centralizes index-aware fallback policy.
- `perl-lsp-protocol`, `perl-lsp-transport`, `perl-content-length-framing`, and
  `perl-lsp-cancellation` decompose protocol, framing, and cancellation into focused crates.
- `.github/dependabot.yml` still tracks `tower-lsp` updates as a defensive watch item, but the
  workspace does not use `tower-lsp` as a production dependency.

This runtime decision already shaped multiple accepted ADRs:

- ADR-0012 requires explicit error propagation and graceful recovery rather than framework-driven
  panics.
- ADR-0016 and ADR-0021 require profile-aware capability advertisement and enforcement.
- ADR-0031 adds bounded concurrent dispatch and inline cancellation on top of the custom runtime.

What was missing was a single ADR that states the higher-level architectural choice: the project
owns the LSP transport and dispatch stack itself, and other runtime decisions build on that base.

## Alternatives Considered

1. **Adopt `tower-lsp` as the primary runtime**.
   Rejected because the project needs direct control over capability advertisement, cancellation,
   malformed-frame handling, and cross-crate transport reuse.

2. **Use `lsp-server` or another thin transport crate for framing/dispatch while keeping custom
   feature governance**.
   Rejected because it would still split ownership of core runtime behavior and make profile-driven
   dispatch policy harder to reason about.

3. **Keep the current architecture but leave the rationale scattered across implementation docs**.
   Rejected because contributors can see the custom runtime in code, but not the explicit decision
   drivers, boundaries, and trade-offs in the ADR index.

## Decision

The project will continue to own a **custom LSP runtime and protocol stack**, implemented as
focused workspace crates and `perl-lsp` runtime modules, rather than adopting an external LSP
server framework as the primary execution model.

### Runtime boundaries

The custom runtime includes:

- **Protocol types and method constants** in `perl-lsp-protocol`
- **Content-Length framing and JSON-RPC transport** in `perl-lsp-transport` and
  `perl-content-length-framing`
- **Cancellation registry and cleanup contracts** in `perl-lsp-cancellation`
- **Input validation and path hardening** in `perl-lsp-input-validation`
- **Feature-profile-to-capability mapping** in the `perl-lsp-feature-*` crates
- **Request ingress, classification, routing, and egress integration** in `crates/perl-lsp/src/runtime/`

### Decision drivers

1. **Capability governance is a first-class architectural concern**.
   The server must advertise different capability sets based on build flags and runtime profile.
   That requirement is directly tied to ADR-0016 and ADR-0021.

2. **Cancellation must be inline and low-overhead**.
   Control requests such as `$/cancelRequest` are processed according to explicit runtime policy,
   now reinforced by ADR-0031's worker-queue design.

3. **Transport behavior must be shared across products**.
   `perl-content-length-framing` is reusable between LSP and DAP, which would be harder if framing
   lived inside an external LSP framework.

4. **Malformed input handling must match the project's no-panic policy**.
   The server prefers explicit recovery, bounded failure, and predictable fallbacks over opaque
   framework behavior.

5. **The project values inspectable control flow**.
   A direct match-based dispatch path and explicit routing helpers make policy visible in code and
   easier to test at protocol boundaries.

## Consequences

### Positive

- **Policy is owned locally**: transport, dispatch, cancellation, and capability rules evolve with
  the rest of the workspace instead of around a framework abstraction.
- **Cross-protocol reuse**: Content-Length framing stays shareable between LSP and DAP.
- **Strong alignment with existing ADRs**: error handling, feature governance, capability contract,
  and concurrent dispatch all compose cleanly on top of the custom runtime.
- **Better contributor orientation**: the ADR index now explicitly explains why the codebase has a
  runtime subsystem instead of a framework adapter layer.

### Negative / Trade-offs

- **Higher maintenance cost**: the team owns message framing, dispatch, runtime evolution, and
  integration testing.
- **Framework features are not free**: new middleware-style behavior must be designed in-house.
- **Architectural drift risk**: implementation docs can become stale if the ADR is not updated when
  runtime architecture changes.

## Guardrails

To keep this decision sustainable:

- New runtime behavior should be implemented in the existing runtime crates/modules rather than by
  introducing an overlapping framework.
- Any proposal to add `tower-lsp` or similar as a production dependency should include a superseding
  ADR that explains how feature governance, cancellation, and shared framing would be preserved.
- Runtime documentation should be kept aligned with ADR-0031 and this ADR whenever request
  scheduling or transport ownership changes.

## References

- `crates/perl-lsp/src/runtime/serving.rs`
- `crates/perl-lsp/src/runtime/routing.rs`
- `docs/project/CUSTOM_LSP_RUNTIME.md`
- `.github/dependabot.yml`
