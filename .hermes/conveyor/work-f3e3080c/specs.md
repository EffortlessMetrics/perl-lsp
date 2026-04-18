# Specification: LSP 3.18 `textDocument/test` Endpoint for Test Discovery

## Feature Description

Implement the LSP 3.18 `testing/textDocument/test` request handler for test discovery in perl-lsp. This enables VS Code Test Explorer and other LSP 3.18-compliant clients to discover tests in Perl files via the standardized Testing Protocol endpoint.

The implementation reuses the existing test discovery logic in `perl-tdd-support::test_runner::TestRunner::discover_tests()` and exposes it via the new LSP 3.18 endpoint while maintaining backward compatibility with the existing `experimental/testDiscovery` endpoint.

## Acceptance Criteria

### AC1: Protocol Infrastructure
- [ ] Method constant `TEXT_DOCUMENT_TEST = "testing/textDocument/test"` exists in `perl-lsp-protocol/src/methods.rs`
- [ ] `TEXT_DOCUMENT_TEST` is used in dispatch routing (not raw string literals)

### AC2: Feature Flag Infrastructure
- [ ] `testing_provider: bool` field exists in `BuildFlags` struct in `perl-lsp-feature-flags/src/lib.rs`
- [ ] `testing_provider` defaults to `false` in `BuildFlags::default()`
- [ ] `testing_provider` is included in `ga_lock()` feature set
- [ ] `testing_provider` is included in `to_advertised_features()` mapping
- [ ] `testing_provider` is included in `to_feature_ids()` mapping

### AC3: Feature ID Registration
- [ ] `LSP_TEXT_DOCUMENT_TEST = "lsp.text_document_test"` constant exists in `perl-lsp-feature-ids/src/lib.rs`

### AC4: Capability Registration
- [ ] `testingProvider: true` is injected at top-level of server capabilities JSON (not in `experimental` field)
- [ ] Injection follows `documentRangesFormattingProvider` pattern (top-level in `capabilities_json()`, not `capabilities_for()`)
- [ ] `testingProvider` is only advertised when `testing_provider` feature flag is `true`

### AC5: Feature Catalog Entry
- [ ] Feature entry for `lsp.text_document_test` exists in `features.toml`
- [ ] Entry includes appropriate maturity status (experimental/alpha)

### AC6: Request Routing
- [ ] `testing/textDocument/test` requests are routed to a handler (not unhandled)
- [ ] `experimental/testDiscovery` routing remains unchanged (backward compatibility)

### AC7: Handler Implementation
- [ ] `handle_text_document_test()` handler exists in `misc.rs` (or appropriate module)
- [ ] Handler reuses `TestRunner::discover_tests()` from `perl-tdd-support`
- [ ] Response format is compatible with LSP 3.18 TestItem schema
- [ ] Handler returns empty array for non-Perl files or files with no discovered tests

### AC8: Capability Contract Tests
- [ ] Capability contract lock file includes `testingProvider` when `testing_provider` is enabled
- [ ] Lock file is updated to expect `testingProvider` presence (not CI failure)

### AC9: Backward Compatibility
- [ ] `experimental/testDiscovery` continues to work after changes
- [ ] No regression in existing test discovery functionality

## Non-Goals

The following are explicitly out of scope for this implementation:
- `testing/textDocument/test/run` — test execution (separate concern)
- `testing/testController/publish` — test result reporting (separate concern)
- `testing/testController/register` — test controller registration (separate concern)
- VS Code Test Explorer UI integration — tracked in issue #3433
- Test execution via code lenses — already exists via `perl-lsp-code-lens`

## Dependencies

- `perl-tdd-support` crate (already a dependency of `perl-lsp`)
- `lsp-types` 0.97.0 (capability injection via JSON, no type upgrade needed)
- Existing `experimental/testDiscovery` infrastructure (for backward compatibility reference)

## Implementation Notes

### Capability Injection Pattern
Unlike `typeHierarchyProvider` (which goes through `experimental`), `testingProvider` must be injected at the top-level of `ServerCapabilities` JSON because it is a first-class LSP 3.18 field. Follow the `documentRangesFormattingProvider` pattern in `capabilities_json()`.

### Feature Flag Behavior
When `testing_provider: false` (default), the `testingProvider` capability is NOT advertised. This allows builds to exclude the feature until it is stable. When `testing_provider: true`, the capability is advertised.

### Backward Compatibility
The existing `experimental/testDiscovery` handler is preserved unchanged. Existing clients using that endpoint will continue to work. The new `testing/textDocument/test` endpoint provides the same functionality via the standardized LSP 3.18 protocol.
