# ADR-0035: Synthetic Fallback URIs at LSP Boundaries

**Status**: Accepted
**Date**: 2026-03-18
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0012](0012-error-handling-strategy.md), [ADR-0013](0013-utf16-position-tracking.md), [ADR-0034](0034-custom-lsp-runtime.md), [CODEBASE_CURIOSITIES.md](../project/CODEBASE_CURIOSITIES.md)

## Context

Several components in the workspace must convert user-controlled or reconstructed URI strings into
`lsp_types::Uri` values even when the source data is not trustworthy.

Two current examples are:

- `crates/perl-lsp-uri/src/lib.rs`, where `parse_uri()` accepts arbitrary strings and is used by
  LSP-facing code that prefers a typed URI over a hard parse failure.
- `crates/perl-position-tracking/src/wire.rs`, where `WireLocation -> lsp_types::Location`
  conversion must produce an LSP-compatible URI even if the stored `uri: String` field is invalid.

In both places the implementation follows the same pattern:

1. Try to parse the provided URI string.
2. If parsing fails, try a short list of known-safe fallback URIs such as `file:///unknown`,
   `about:blank`, and `urn:perl-lsp:unknown`.
3. If parser behavior ever changes and those literals also fail, keep generating
   `http://localhost/<n>` values until one parses.

This behavior is intentionally surprising. Most codebases would return `Result`, drop the location,
log an error, or panic under the assumption that a fallback URI literal can never fail. Here,
production code instead guarantees that LSP conversion returns *some* valid URI object.

### Problem Statement

The project needed a policy for URI parsing failures that aligns with existing system goals:

1. **No panic in production paths** per ADR-0012.
2. **Protocol compliance** for LSP structures that require a URI field.
3. **Graceful degradation** when upstream data is malformed or reconstructed imperfectly.
4. **Minimal dependency coupling** between boundary crates that need local URI recovery helpers.

Without an explicit ADR, contributors may reasonably “simplify” this code into direct `.parse()?`,
`unwrap()`, or “impossible” assumptions and accidentally remove a deliberate resilience policy.

## Decision

**At LSP conversion boundaries, the project will prefer a guaranteed-valid synthetic URI over
propagating parse failure or panicking when the input URI string is invalid.**

### Chosen Policy

When a boundary must yield an `lsp_types::Uri` or an LSP object that contains one:

- first return the original URI if parsing succeeds
- otherwise use a stable fallback URI literal from a small allowlist
- if all fallback literals ever fail, generate synthetic `http://localhost/<n>` URIs until parsing
  succeeds

This is a **resilience-first boundary policy**, not a claim that the original URI was valid.

### Why This Was Chosen

1. **LSP structures are often more useful than total failure.**
   For diagnostics, locations, and other editor responses, a placeholder URI is frequently better
   than crashing the server or abandoning the entire response.

2. **The project already favors degraded-but-safe behavior.**
   This decision matches existing architecture patterns such as degraded index access, text-based
   fallbacks, and no-panic production constraints.

3. **Boundary crates need local recovery.**
   `perl-lsp-uri` and `perl-position-tracking` both sit near protocol edges. Duplicating a tiny,
   explicit recovery helper is preferable to adding a broader dependency edge solely for fallback
   construction.

4. **It future-proofs against parser behavior changes.**
   The open-ended `http://localhost/<n>` loop is intentionally defensive: even if assumptions about
   “known good” literals break, the code still avoids panicking.

## Alternatives Considered

### Option 1: Return `Result<Uri, _>` everywhere and force callers to decide

**Pros**:
- Makes invalid input explicit
- Avoids synthetic placeholders
- Conventional Rust API design

**Cons**:
- Pushes repetitive recovery logic into many call sites
- Risks more dropped responses or inconsistent fallback policy
- Does not help APIs that must already produce concrete LSP structures at the boundary

**Decision**: Rejected as the default boundary policy.

### Option 2: Panic or `unwrap()` after trying "obviously valid" fallback literals

**Pros**:
- Simple implementation
- Would expose unexpected parser regressions loudly in testing

**Cons**:
- Violates the production no-panic policy
- Turns malformed user input or library behavior drift into server termination
- Removes graceful degradation exactly where protocol-facing code should be defensive

**Decision**: Rejected.

### Option 3: Drop invalid locations/diagnostics instead of synthesizing URIs

**Pros**:
- Avoids invented identifiers
- Keeps invalid data from crossing the boundary

**Cons**:
- Loses partially useful results
- Makes failures silent and harder to debug from editor behavior alone
- Creates inconsistent behavior across features depending on who remembered to drop vs recover

**Decision**: Rejected as the general policy.

### Option 4: Synthesize fallback URIs locally at the boundary

**Pros**:
- Keeps protocol-facing code resilient
- Preserves no-panic behavior
- Allows narrow, dependency-light helpers
- Ensures concrete LSP values are always constructible

**Cons**:
- Placeholder URIs can obscure the original parse failure unless separately logged
- The `http://localhost/<n>` loop looks odd to unfamiliar readers
- Duplicated helpers require documentation to avoid accidental divergence

**Decision**: Accepted.

## Consequences

### Positive

- **Server resilience**: invalid URI strings do not crash protocol conversion paths.
- **Consistent degradation policy**: boundary code follows the same “return something safe” design
  used elsewhere in the system.
- **Protocol continuity**: editor-facing responses can still be constructed when URI strings are
  malformed.
- **Explicitly documented oddity**: future maintainers now have an ADR explaining why the fallback
  code looks unusual.

### Negative / Trade-offs

- **Potential ambiguity**: consumers may receive a placeholder URI instead of a hard error.
- **Observability gap**: if callers do not log parse failures, malformed upstream URIs may be less
  visible than they would be with a strict `Result` flow.
- **Implementation duplication**: multiple crates currently carry near-identical fallback logic.

## Revisit Triggers

Review this ADR if any of the following become true:

- the project adopts a single shared protocol-boundary crate for all URI conversion
- editor-facing features start requiring strict provenance of every URI value
- malformed URI inputs become common enough that silent placeholder substitution hurts debugging
- `lsp_types::Uri` parsing semantics or protocol expectations change materially

## References

- `crates/perl-lsp-uri/src/lib.rs`
- `crates/perl-position-tracking/src/wire.rs`
- `docs/project/CODEBASE_CURIOSITIES.md`
