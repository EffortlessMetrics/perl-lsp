# ADR-0353: LSP 3.18 `testingProvider` Capability via Top-Level JSON Injection

## Status
Proposed

## Context

Issue #3532 requests implementing the LSP 3.18 `textDocument/test` endpoint for test discovery. The existing `perl-tdd-support` crate provides working test discovery logic, but it is exposed via the non-standard `experimental/testDiscovery` endpoint which VS Code Test Explorer and other LSP clients cannot use.

The LSP 3.18 Testing Protocol specifies `testingProvider` as a top-level `ServerCapabilities` field. However, the codebase uses `lsp-types` 0.97.0 which predates LSP 3.18 and does not include the `TestingProvider` type.

Two injection patterns exist in the codebase for LSP features missing from `lsp-types`:
- **Pattern A**: `typeHierarchyProvider` — injected via `caps.experimental` field
- **Pattern B**: `documentRangesFormattingProvider` — injected at top-level in `capabilities_json()`

The architectural decision is: which pattern applies to `testingProvider`?

## Decision

**`testingProvider` MUST use Pattern B (top-level JSON injection via `capabilities_json()`)**, NOT Pattern A (experimental field injection).

Rationale:
- `testingProvider` is a **top-level** `ServerCapabilities` field in LSP 3.18, not an experimental field
- `documentRangesFormattingProvider` is the correct precedent because it is also a top-level field in LSP 3.18
- `typeHierarchyProvider` goes through `experimental` because it was originally an experimental extension
- Injecting `testingProvider` via `experimental` would be semantically incorrect and could confuse LSP clients

The implementation must also:
1. **Keep `experimental/testDiscovery`** for backward compatibility with existing clients
2. Default the `testing_provider` feature flag to `false` until the feature is mature
3. Update capability contract lock file tests when the new capability is added

## Alternatives Considered

### Alternative 1: Use `typeHierarchyProvider` pattern (experimental field injection)
**Rejected**: `typeHierarchyProvider` is injected via `experimental` because it was historically an experimental extension. `testingProvider` is a first-class top-level field in LSP 3.18 and should not be treated as experimental. Using this pattern would be semantically incorrect.

### Alternative 2: Skip capability advertisement, only add method routing
**Rejected**: LSP clients (VS Code Test Explorer) advertise capabilities to determine which protocol methods to call. Without `testingProvider` in server capabilities, clients will not send `testing/textDocument/test` requests. Capability advertisement is required for the feature to work.

### Alternative 3: Upgrade `lsp-types` to a version with `TestingProvider`
**Rejected**: Upgrading `lsp-types` is a significant change affecting many parts of the codebase. The LSP 3.18 types may have other breaking changes. The JSON injection pattern is already established and appropriate for incremental adoption.

## Consequences

### Benefits
- Enables VS Code Test Explorer and other LSP 3.18-compliant clients to discover tests via the standard `testing/textDocument/test` endpoint
- Reuses existing `TestRunner::discover_tests()` logic without reimplementation
- Maintains backward compatibility with existing `experimental/testDiscovery` clients
- Follows established codebase patterns for LSP 3.18 features missing from `lsp-types`

### Tradeoffs
- `testingProvider` is injected as a raw JSON field rather than a typed `lsp-types` struct, reducing compile-time type safety
- The capability contract lock file must be updated to include the new capability, which may cause CI friction
- Feature flag defaults to `false`, so the capability is not advertised unless explicitly enabled in builds

### Risks
- **Risk**: Capability contract lock file conflicts — mitigated by explicitly updating lock file test expectations before implementation
- **Risk**: `lsp-types` upgrade in the future may conflict with the JSON injection — mitigated by keeping the injection localized to `capabilities_json()`

## Decision Details

### Protocol Constant
- `TEXT_DOCUMENT_TEST = "testing/textDocument/test"` in `methods.rs`

### Feature Infrastructure
- Feature flag: `testing_provider: bool` in `BuildFlags`, default `false`
- Feature ID: `LSP_TEXT_DOCUMENT_TEST = "lsp.text_document_test"`
- Feature catalog entry in `features.toml`

### Capability Registration
- Inject `testingProvider: true` at top-level of `ServerCapabilities` JSON in `capabilities_json()`
- Do NOT inject via `experimental` field
- Follow `documentRangesFormattingProvider` pattern (`capabilities.rs:329-337`)

### Request Routing
- Route `testing/textDocument/test` to new `handle_text_document_test()` handler
- Keep `experimental/testDiscovery` routing unchanged

### Handler Implementation
- `handle_text_document_test()` reuses `TestRunner::discover_tests()` from `perl-tdd-support`
- Response format follows LSP 3.18 `TestItem` schema

## References

- Issue #3532: https://github.com/EffortlessMetrics/perl-lsp/issues/3532
- LSP 3.18 Testing Protocol specification
- `documentRangesFormattingProvider` pattern: `capabilities.rs:329-337`
- `typeHierarchyProvider` pattern: `capabilities.rs:297-307`
- `experimental/testDiscovery` current implementation: `dispatch/mod.rs:294`
