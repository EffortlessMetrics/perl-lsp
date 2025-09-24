# Issue #48: Fix LSP cancellation test failures and cleanup unused test helpers

## Context

The perl-lsp crate's test infrastructure contains three failing LSP cancellation tests that impact test suite reliability and potentially indicate LSP request cancellation protocol compliance issues. These failures affect the LSP workflow's ability to handle client request cancellations properly, which is critical for responsive editor integration and protocol specification adherence. Additionally, unused test helper functions in the test support modules create code quality issues and build warnings that need systematic cleanup.

The failing tests (`test_cancel_multiple_requests`, `test_cancel_request_handling`, `test_cancel_request_no_response`) directly impact LSP protocol compliance for the JSON-RPC cancellation specification, affecting editor responsiveness during long-running operations like workspace symbol searches, cross-file navigation, and comprehensive parsing tasks.

## User Story

As a Perl developer using LSP-compatible editors, I want the LSP server to properly handle request cancellations so that my editor remains responsive when I cancel long-running operations like workspace searches or parsing tasks, and the test infrastructure remains reliable for continuous integration.

## Acceptance Criteria

AC1: All three LSP cancellation tests (`test_cancel_multiple_requests`, `test_cancel_request_handling`, `test_cancel_request_no_response`) pass consistently in both single-threaded and multi-threaded test environments

AC2: LSP cancellation protocol compliance is verified through proper JSON-RPC `$/cancelRequest` message handling according to LSP specification section 4.5

AC3: Request cancellation properly terminates ongoing operations including workspace symbol searches, cross-file navigation, and parsing tasks without resource leaks

AC4: Cancelled requests return appropriate error responses with `RequestCancelled` error code (-32800) as per JSON-RPC specification

AC5: Multiple concurrent request cancellations are handled correctly without race conditions or deadlock scenarios

AC6: All unused test helper functions in `crates/perl-lsp/tests/support/test_helpers.rs` are identified and removed systematically

AC7: Build warnings related to unused functions in test support modules are eliminated completely

AC8: Remaining test helper functions have comprehensive documentation explaining their purpose and usage patterns

AC9: Test infrastructure maintains backward compatibility while improving code quality and reducing maintenance overhead

AC10: Cancellation handling works correctly with adaptive threading configuration (RUST_TEST_THREADS=2) for CI environments

AC11: Integration tests validate cancellation behavior across all LSP providers (completion, hover, definition, references, symbols)

AC12: Performance impact of cancellation handling is minimal (< 1ms overhead) and does not affect normal request processing latency

## Technical Implementation Notes

- **Affected crates**: perl-lsp (primary), perl-parser (LSP provider integration)
- **LSP workflow stages**: All stages affected - cancellation must work across Parse → Index → Navigate → Complete → Analyze operations
- **Performance considerations**: Cancellation overhead < 1ms, proper cleanup of cancelled operations, memory leak prevention
- **Protocol compliance requirements**:
  - JSON-RPC 2.0 `$/cancelRequest` notification handling
  - LSP specification section 4.5 compliance for request cancellation
  - Proper error code responses (-32800 RequestCancelled)
- **Threading considerations**: Thread-safe cancellation with adaptive threading support (RUST_TEST_THREADS=2)
- **Test infrastructure quality**: Systematic cleanup of unused functions, comprehensive documentation for remaining helpers
- **Error handling patterns**: Proper `anyhow::Result<T>` patterns with cancellation context preservation
- **Integration requirements**: Cancellation support across all LSP providers (textDocument/completion, textDocument/hover, textDocument/definition, textDocument/references, workspace/symbol)
- **Testing strategy**:
  - TDD with `// AC:ID` tags for each acceptance criterion
  - LSP protocol compliance testing with real JSON-RPC message validation
  - Performance regression testing for cancellation overhead
  - Multi-threaded stress testing for race condition detection
  - Integration testing across all LSP provider methods
  - Test helper cleanup validation with build warning elimination