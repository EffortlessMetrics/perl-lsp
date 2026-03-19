# ADR-0037: Guaranteed-Valid Synthetic URI Fallbacks

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0012](0012-error-handling-strategy.md), [ADR-0034](0034-custom-lsp-runtime.md), [CODEBASE_CURIOSITIES.md](../project/CODEBASE_CURIOSITIES.md)

## Context

The codebase contains an unusual but intentional pattern at the LSP wire boundary: URI parsing
helpers do not propagate parse failures upward and do not panic when given malformed URI strings.
Instead, they synthesize a known-valid fallback URI.

This behavior currently appears in more than one place:

- `crates/perl-lsp-uri/src/lib.rs` exposes `parse_uri()` for LSP-facing components
- `crates/perl-position-tracking/src/wire.rs` converts wire locations into `lsp_types::Location`
  using the same fallback strategy when URI parsing fails

The fallback sequence is deliberately defensive:

1. try a short list of stable literals such as `file:///unknown`, `file:///`, `about:blank`, and
   `urn:perl-lsp:unknown`
2. if parser behavior changes unexpectedly, generate `http://localhost/<n>` candidates in a loop
   until one parses

This is not a normal "strictly validate and return an error" design. It exists because malformed
URIs often arrive at the protocol boundary from partially indexed state, external tooling, or
stale workspace data, and the server's production policy strongly prefers graceful degradation over
request failure or process termination.

### Problem Statement

The project needs a consistent policy for malformed URIs crossing the LSP boundary:

- **Strict failure** preserves fidelity but can cause whole requests to fail even when the rest of
  the payload is usable
- **Panicking or assuming validity** violates the no-panic production policy and risks bringing
  down the server from malformed external data
- **Ad-hoc local fallbacks** create inconsistent behavior across crates and make debugging harder

The code had already converged on a resilient answer, but the rationale was only implicit in the
implementation and narrative docs.

## Decision

**The project will treat URI parsing at the LSP/wire boundary as a resilience boundary and will
return a guaranteed-valid synthetic URI when parsing fails, rather than panicking or failing the
entire higher-level operation.**

### Chosen Policy

1. **Prefer the original URI** whenever parsing succeeds.
2. **Use a deterministic fallback candidate list first** so the behavior remains stable and
   predictable.
3. **Guarantee a valid URI even if parser acceptance changes** by keeping an open-ended generated
   fallback path.
4. **Contain this policy in URI/wire helper layers** instead of scattering custom recovery logic
   through feature providers.

### Why This Was Chosen

1. **Protocol inputs are not fully trustworthy.**
   LSP payloads and derived workspace metadata are external inputs. The server should not assume
   they remain perfectly well-formed.

2. **Availability matters more than URI fidelity in degraded paths.**
   A synthetic URI is usually less harmful than dropping a whole response such as diagnostics,
   navigation results, or position-conversion output.

3. **This matches the repository's broader no-panic philosophy.**
   ADR-0012 already establishes that production code should avoid fatal behavior. Synthetic fallback
   URIs are the URI-specific expression of that same principle.

4. **Centralization reduces surprise.**
   By keeping the behavior inside dedicated helper crates/modules, callers can rely on a uniform
   contract instead of repeatedly deciding how to recover.

## Alternatives Considered

### Option 1: Return `Result<Uri, _>` everywhere and fail callers on parse errors

**Pros**:
- Preserves exact input validity information
- Makes malformed URIs visible to every caller
- Avoids inventing synthetic identifiers

**Cons**:
- Pushes repetitive recovery logic into many feature handlers
- Increases the chance that one malformed URI aborts an otherwise useful response
- Conflicts with the project preference for graceful degradation at integration boundaries

**Decision**: Rejected as the default LSP boundary policy.

### Option 2: Panic or use `unwrap()` because URIs are "supposed" to be valid

**Pros**:
- Minimal code at call sites
- Treats malformed URIs as programmer errors

**Cons**:
- Violates production no-panic rules
- Makes externally supplied malformed data a server-stability risk
- Turns recoverable boundary issues into process-level failures

**Decision**: Rejected.

### Option 3: Use optional URIs (`Option<Uri>`) and drop affected results

**Pros**:
- Avoids synthetic identifiers
- Makes malformed data explicit without panicking

**Cons**:
- Silently loses diagnostics/navigation payloads tied to bad URIs
- Forces every caller to define omission behavior
- Produces inconsistent user experience depending on which path chose to drop data

**Decision**: Rejected as the common policy.

### Option 4: Centralized guaranteed-valid synthetic fallback

**Pros**:
- Keeps the server available and responses structurally valid
- Localizes malformed-URI recovery to dedicated layers
- Aligns with existing implementation and no-panic design

**Cons**:
- Can mask the original malformed URI from downstream consumers
- Introduces synthetic identifiers that do not correspond to real files
- Requires documentation so contributors understand why the code is intentionally unusual

**Decision**: Accepted.

## Consequences

### Positive

- **Resilient protocol handling**: malformed URI strings do not crash the server or automatically
  abort higher-level LSP responses.
- **Uniform recovery behavior**: callers in navigation, diagnostics, and wire conversion can depend
  on the same fallback contract.
- **Tighter boundary ownership**: URI resilience lives in helper layers instead of leaking into many
  feature implementations.

### Negative / Trade-offs

- **Loss of fidelity**: downstream logic may see a synthetic URI instead of the original malformed
  string.
- **Debugging ambiguity**: if malformed inputs become common, synthetic URIs can hide the source of
  the data quality problem unless instrumentation is added.
- **Potential duplication pressure**: similar fallback helpers in multiple crates should be kept in
  sync or consolidated over time.

## Revisit Triggers

Review this ADR if any of the following become true:

- the project adopts stricter end-to-end validation that requires surfacing URI parse failures to
  clients
- malformed URIs become common enough that silent recovery materially harms debuggability
- helper duplication around URI fallback becomes costly enough to justify consolidation into a
  single shared crate/API
- the upstream LSP URI type changes in a way that makes a different resilience strategy preferable

## References

- `crates/perl-lsp-uri/src/lib.rs`
- `crates/perl-position-tracking/src/wire.rs`
- `docs/project/CODEBASE_CURIOSITIES.md`
- `docs/project/CLEAN_CODE_SHOWCASE.md`
