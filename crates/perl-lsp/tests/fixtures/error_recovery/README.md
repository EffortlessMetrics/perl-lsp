# LSP Error Recovery Test Fixtures for Issue #178

This directory contains Perl code fixtures for testing LSP server graceful degradation and error recovery as specified in Issue #178 (AC9).

## Directory Structure

```
error_recovery/
├── missing_semicolon.pl      # Missing semicolon syntax errors
├── unterminated_string.pl    # Unterminated string/heredoc errors
├── invalid_token.pl          # Lexer-level invalid token errors
├── partial_valid.pl          # Mixed valid/invalid code for partial AST testing
└── README.md                 # This file
```

## Fixture Purpose

These fixtures test **LSP graceful degradation** during parse and lexer errors:
- LSP server should remain responsive after encountering errors
- Diagnostics should be published instead of server crashes
- LSP features should work on valid portions of partial AST
- Session continuity should be maintained across multiple errors

## Fixture Descriptions

### `missing_semicolon.pl`

Tests LSP behavior with missing semicolons (common syntax error).

**Features to Test:**
- Diagnostic publication for missing semicolons
- Completion on valid code after errors
- Definition navigation to valid subroutines
- Workspace symbols from valid definitions

**Expected LSP Behavior:**
- Publish diagnostics with severity ERROR
- Continue indexing valid subroutines (`valid_function`, `another_function`, `calculate`)
- Provide completion for `%config` hash keys
- Navigate to subroutine definitions despite syntax errors

### `unterminated_string.pl`

Tests LSP behavior with unterminated strings, heredocs, and regexes.

**Features to Test:**
- Multiple string error recovery
- Partial AST construction with valid code between errors
- Subroutine indexing despite string errors
- Array/hash completion after errors

**Expected LSP Behavior:**
- Publish multiple diagnostics for each unterminated string
- Index valid subroutines (`process_string`, `another_sub`, `final_function`)
- Provide completion for `@array` elements
- Maintain session continuity across multiple errors

### `invalid_token.pl`

Tests LSP behavior with lexer-level invalid tokens.

**Features to Test:**
- Error token handling from lexer
- Recovery from invalid operators and sigils
- Partial AST features (hover, completion, navigation)
- Diagnostic collection for multiple lexer errors

**Expected LSP Behavior:**
- Convert error tokens to LSP diagnostics
- Index valid symbols despite lexer errors
- Provide semantic tokens for valid code portions
- Support find references on valid symbols

### `partial_valid.pl`

Tests LSP feature availability with partial valid code (comprehensive test).

**Features to Test:**
- Hover information on package variables
- Definition navigation to valid subroutines
- Completion for hash keys
- Semantic tokens for valid portions
- Folding ranges on valid blocks
- Call hierarchy for valid function calls
- Find references across valid code

**Expected LSP Behavior:**
- Provide hover for `$PACKAGE_VAR`
- Navigate to `valid_function`, `connect_database`, `call_other_functions`
- Offer completion for `%database_config` keys
- Generate semantic tokens for valid syntax
- Create folding ranges for valid blocks
- Build call hierarchy from `call_other_functions`
- Find all references to `$PACKAGE_VAR`

## Usage in Tests

### LSP Error Recovery Tests (`lsp_error_recovery_behavioral_tests.rs`)

```rust
#[test]
fn test_lsp_server_session_continuity_on_parse_error() {
    let fixture = include_str!("fixtures/error_recovery/missing_semicolon.pl");

    // Initialize LSP server
    let mut harness = LspTestHarness::new();
    harness.open_document("test.pl", fixture);

    // Should publish diagnostics, not crash
    let diagnostics = harness.wait_for_diagnostics();
    assert!(!diagnostics.is_empty());
    assert!(diagnostics.iter().any(|d| d.message.contains("semicolon")));

    // Server should remain responsive
    let symbols = harness.request_workspace_symbols();
    assert!(symbols.iter().any(|s| s.name == "valid_function"));
}

#[test]
fn test_partial_ast_lsp_feature_availability() {
    let fixture = include_str!("fixtures/error_recovery/partial_valid.pl");

    let mut harness = LspTestHarness::new();
    harness.open_document("partial.pl", fixture);

    // Completion should work on valid portions
    let completions = harness.request_completion(line_with_config_hash);
    assert!(completions.iter().any(|c| c.label == "host"));

    // Navigation should work for valid subroutines
    let definition = harness.request_definition("valid_function");
    assert!(definition.is_some());

    // Hover should work on valid variables
    let hover = harness.request_hover("$PACKAGE_VAR");
    assert!(hover.is_some());
}
```

## Related Documentation

- [LSP_IMPLEMENTATION_GUIDE.md](../../../../docs/LSP_IMPLEMENTATION_GUIDE.md)
- [PARSER_ERROR_HANDLING_SPEC.md](../../../../docs/PARSER_ERROR_HANDLING_SPEC.md)
- [THREADING_CONFIGURATION_GUIDE.md](../../../../docs/THREADING_CONFIGURATION_GUIDE.md)
- [issue-178-spec.md](../../../../docs/issue-178-spec.md)

## Acceptance Criteria Coverage

| AC | Feature | Fixture Coverage |
|----|---------|------------------|
| AC9 | LSP session continuity | All fixtures test server responsiveness |
| AC9 | Diagnostic publication | All fixtures trigger diagnostic conversion |
| AC9 | Partial AST features | `partial_valid.pl` tests comprehensive feature availability |
| AC9 | Error recovery performance | All fixtures validate <1ms diagnostic publication |

## Performance Targets

- **Error response**: <50ms end-to-end
- **Diagnostic publication**: <1ms LSP update target
- **Session recovery**: <100ms after error
- **Adaptive threading**: Compatible with RUST_TEST_THREADS=2

## LSP Protocol Compliance

Fixtures validate:
- JSON-RPC 2.0 error responses with correct error codes
- LSP 3.17+ diagnostic standards with severity mapping
- Source attribution ("perl-parser" vs "perl-lexer")
- Accurate Range calculation from byte positions
- DiagnosticSeverity::ERROR for parse/lexer errors

## Quality Assurance

All fixtures designed to:
- Include both valid and invalid Perl code
- Test incremental diagnostic updates
- Validate cross-file error correlation
- Support workspace indexing continuity
- Enable semantic token generation with errors
- Test thread-safe diagnostic publication
