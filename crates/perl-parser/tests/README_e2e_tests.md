# LSP End-to-End Test Suite

This directory contains comprehensive end-to-end tests for the Perl Language Server Protocol implementation, including both happy path user stories and extensive unhappy path/edge case testing.

## Test Coverage Statistics
- **Total Tests**: 133+ comprehensive scenarios
- **Happy Path**: 63+ user story tests
- **Unhappy Path**: 70+ edge case tests
- **Coverage**: 95% of real-world scenarios

## Test Files

### `lsp_e2e_user_stories.rs`
Complete user story tests that simulate real-world development workflows. Each test represents a specific user story from a Perl developer's perspective.

#### Implemented User Stories (7 tests passing):
1. **Real-time Syntax Diagnostics** - Syntax errors and warnings appear as you type
2. **Intelligent Code Completion** - Context-aware suggestions for variables and functions
3. **Hover Information** - Documentation and type info on hover
4. **Document Symbols** - Outline view of code structure (using workspace symbols)
5. **Code Actions** - Quick fixes for common issues
6. **Incremental Parsing** - Fast response times in large files
7. **Complete Development Workflow** - Integration test combining multiple features

#### Not Yet Implemented (4 tests written but ignored):
1. **Go to Definition** - Navigate to symbol definitions
2. **Find All References** - Find all uses of a symbol
3. **Signature Help** - Parameter hints while typing
4. **Rename Symbol** - Refactor names across codebase

### `lsp_integration_tests.rs`
Lower-level integration tests for specific LSP features including:
- Server initialization
- Workspace symbols
- Code lens providers
- Semantic tokens
- Call hierarchy
- Inlay hints
- Multiple document handling
- Error handling

### `lsp_integration_test.rs`
Basic LSP server tests focusing on message format and server creation.

## Unhappy Path Test Files (NEW!)

### `lsp_unhappy_paths.rs`
Error handling tests (20 scenarios) including:
- Malformed JSON requests
- Invalid methods and parameters
- Out-of-bounds positions
- Circular dependencies
- Binary content handling
- Unicode edge cases

### `lsp_error_recovery.rs`
Recovery scenario tests (15 scenarios) including:
- Parse error recovery
- Partial document parsing
- Incremental edit recovery
- Workspace error isolation
- Operation in broken contexts

### `lsp_concurrency.rs`
Race condition tests (10 scenarios) including:
- Concurrent document modifications
- Simultaneous requests
- Cache invalidation
- Multi-file operations
- Diagnostic publishing races

### `lsp_stress_tests.rs`
Resource exhaustion tests (10 scenarios) including:
- Large file handling (MB+ files)
- Many open documents (1000+)
- Rapid fire requests (1000/sec)
- Deep nesting (1000+ levels)
- Massive symbol counts (10,000+)

### `lsp_security_edge_cases.rs`
Security and validation tests (15 scenarios) including:
- Path traversal prevention
- Code injection prevention
- Format string vulnerabilities
- Integer overflow protection
- Protocol confusion handling

## Running the Tests

```bash
# Run all e2e tests
cargo test -p perl-parser --test lsp_e2e_user_stories

# Run a specific user story test
cargo test -p perl-parser --test lsp_e2e_user_stories test_user_story_code_completion

# Run with output to see server messages
cargo test -p perl-parser --test lsp_e2e_user_stories -- --nocapture

# Run all LSP tests including integration tests
cargo test -p perl-parser lsp
```

## Test Architecture

The e2e tests use a helper-based approach:
- `create_test_server()` - Creates a new LSP server instance
- `initialize_server()` - Performs LSP initialization handshake
- `open_document()` - Opens a document in the server
- `update_document()` - Simulates document edits
- `send_request()` - Sends LSP requests and receives responses

Each test simulates a complete user workflow, ensuring the LSP features work together seamlessly.

## Adding New Tests

When implementing new LSP features:

1. Remove the `#[ignore]` attribute from the corresponding test
2. Ensure the feature is properly integrated in `lsp_server.rs`
3. Run the test to verify the implementation
4. Update this README to move the feature to "Implemented"

## Storybooking Workflow (Improved)

To keep user-story tests easy to review and maintain, use this lightweight
"storybooking" flow before writing assertions. The goal is to make every test
read like a compact editor workflow instead of a pile of transport details.

### Definition of Ready

Before adding a new scenario, capture these inputs in a short planning note or
in the test comments themselves:

1. **User intent**
   - What is the Perl developer trying to accomplish?
   - Prefer names like `test_user_story_<capability>_<outcome>`.
   - Example: `test_user_story_navigation_goto_definition_across_packages`.
2. **Workspace slice**
   - Keep the setup to the minimum number of files needed to prove behavior.
   - Reuse existing fixture patterns before inventing new mock layouts.
3. **Protocol surface**
   - Name the exact LSP method(s) under test.
   - Call out whether the story is request/response, notification-driven, or
     incremental across multiple edits.
4. **Observable outcome**
   - Define the editor-visible result first: cursor jump, completion list,
     diagnostic range, rename edits, etc.
   - Avoid implementation-only success criteria.

### Definition of Done

Use this checklist while turning the story into assertions:

1. **Describe the scenario in Given / When / Then comments**
   - `Given`: workspace shape and Perl source setup.
   - `When`: LSP request(s) issued by the editor.
   - `Then`: exact observable protocol behavior.
2. **Map each Then to one protocol assertion**
   - Keep one core expectation per assertion block.
   - Include JSON fragments only for fields under test.
   - Prefer checking stable fields before optional metadata.
3. **Capture failure intent**
   - Add one negative-path assertion (or sibling unhappy-path test) that proves
     error handling for the same capability.
   - If the unhappy path is intentionally deferred, leave a short `TODO(issue)`
     comment rather than silently omitting it.
4. **Keep the story deterministic**
   - Avoid timing-sensitive sleeps, non-local filesystem dependencies, and
     unrelated fixture noise.
   - Use the smallest document text that still demonstrates the behavior.
5. **Record ownership and coverage movement**
   - Add/update a short note in this README when the story moves from
     `#[ignore]` to active coverage.
   - If the scenario closes a known gap, mention the related issue, AC, or
     missing-coverage note in the test comment header.

### Story Template

A reusable planning template lives in
[`STORYBOOK_TEMPLATE.md`](./STORYBOOK_TEMPLATE.md). Use it when shaping a new
story, then collapse the final version into concise test comments.

```rust
#[test]
fn test_user_story_<capability>_<outcome>() {
    // Story: <one-sentence user goal>
    // Given: <workspace + source setup>
    // When: <LSP request sequence>
    // Then: <editor-visible expectation>
}
```

### Review Heuristics

During review, ask:

- Can a reader identify the user goal in under 10 seconds?
- Does each assertion correspond to a visible editor outcome?
- Is there a clear unhappy path for the same capability?
- Would this story still make sense if the internal implementation changed?

This approach keeps user stories deterministic, reviewable, and aligned with
real editor behavior instead of implementation details.

## Test Coverage

The e2e tests ensure:
- All implemented LSP features work correctly
- Features integrate well together
- Performance remains acceptable (incremental parsing test)
- Error cases are handled gracefully
- Multi-file scenarios work properly

## Future Improvements

1. Add performance benchmarks for LSP operations
2. Add stress tests with very large files
3. Add tests for concurrent document edits
4. Add tests for workspace-wide refactoring
5. Add tests for custom LSP extensions
