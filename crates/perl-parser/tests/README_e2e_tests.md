# LSP End-to-End Test Suite

This directory contains comprehensive end-to-end tests for the Perl Language Server Protocol implementation.

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