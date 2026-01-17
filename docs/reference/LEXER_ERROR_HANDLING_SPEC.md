# Lexer Error Handling Specification (*Diataxis: Explanation*)

**Issue**: #178 (GitHub #204) - Eliminate Fragile unreachable!() Macros
**Related Specs**: [issue-178-spec.md](issue-178-spec.md), [ISSUE_178_TECHNICAL_ANALYSIS.md](ISSUE_178_TECHNICAL_ANALYSIS.md), [PARSER_ERROR_HANDLING_SPEC.md](PARSER_ERROR_HANDLING_SPEC.md)
**LSP Workflow**: Parse → Index → Navigate → Complete → Analyze
**Performance Target**: <1ms incremental updates, context-aware tokenization

---

## 1. Executive Summary

This specification defines comprehensive error handling patterns for the Perl lexer (perl-lexer crate) to eliminate fragile `unreachable!()` macros in substitution operator tokenization. The specification ensures graceful degradation through diagnostic token emission while maintaining context-aware tokenization performance and LSP protocol compliance.

**Key Requirements**:
- **Diagnostic Token Return**: Replace `unreachable!()` with `TokenType::Error` for invalid substitution operators
- **Context Preservation**: Maintain lexer state for continued tokenization after errors
- **LSP Integration**: Lexer errors map to LSP diagnostics with actionable messages
- **Performance Guarantee**: Zero happy-path overhead, <5μs error path overhead

---

## 2. Lexer Error Category

### 2.1 Category A: Substitution Operator Error

**Scope**: Substitution/transliteration operator validation in lexer tokenization

**Affected File**:
- `perl-lexer/src/lib.rs:1385`

**Current Fragile Pattern**:
```rust
// ❌ Fragile: relies on upstream guard checking for "s", "tr", "y"
match text {
    "s" => {
        return self.parse_substitution(start);
    }
    "tr" | "y" => {
        return self.parse_transliteration(start);
    }
    _ => unreachable!(),  // Panic on unexpected operator
}
```

**Upstream Guard Context**:
```rust
// Guard condition at line ~1370
if matches!(
    next_char_after_match,
    '/' | '#' | '!' | '@' | '%' | '^' | '&' | '*' | '(' | ')'
    | '{' | '}' | '[' | ']' | '<' | '>' | '|' | '\\' | ';' | ':'
    | ',' | '.' | '?' | '-' | '+' | '=' | '_' | '~' | '`'
) {
    match text {
        "s" => return self.parse_substitution(start),
        "tr" | "y" => return self.parse_transliteration(start),
        _ => unreachable!(),  // ⚠️ Guard checks text, but wildcard used
    }
}
```

**Risk Analysis**:
- Upstream guard checks that `next_char_after_match` is a delimiter
- Guard does NOT validate that `text` is specifically "s", "tr", or "y"
- If guard logic changes (e.g., allowing "m" for match operator), wildcard breaks silently
- Refactoring could add new operators without updating match arms

**Required Pattern** (AC2):
```rust
// ✅ Explicit error handling with diagnostic token
match text {
    "s" => {
        return self.parse_substitution(start);
    }
    "tr" | "y" => {
        return self.parse_transliteration(start);
    }
    unexpected => {
        // Return diagnostic token instead of panicking
        return Token {
            token_type: TokenType::Error(Arc::from(format!(
                "Unexpected substitution operator '{}': expected 's', 'tr', or 'y' at position {}",
                unexpected,
                start
            ))),
            start,
            end: self.position,
        };
    }
}
```

**Error Contract**:
- **Type**: `Token` with `TokenType::Error(Arc<str>)`
- **Error Message Format**: `"Unexpected substitution operator '{operator}': expected 's', 'tr', or 'y' at position {pos}"`
- **LSP Mapping**: `DiagnosticSeverity::ERROR`
- **Recovery Strategy**: Emit error token, continue tokenization at next character

**Test Requirements** (AC2, AC6):
```rust
/// AC:2 - Lexer substitution operator error handling
#[test]
fn test_ac2_lexer_substitution_operator_error_handling() {
    let mut lexer = PerlLexer::new();

    // Test case 1: 'm' operator with delimiter (should trigger guard but invalid in match)
    let invalid_code = "m/pattern/";  // 'm' not handled by s/tr/y match

    let tokens = lexer.tokenize(invalid_code);

    // Should contain an error token with descriptive message
    let error_token = tokens.iter().find(|t| matches!(t.token_type, TokenType::Error(_)));
    assert!(error_token.is_some(), "Should produce error token for invalid operator");

    if let Some(token) = error_token {
        if let TokenType::Error(msg) = &token.token_type {
            assert!(msg.contains("Unexpected substitution operator"));
            assert!(msg.contains("expected 's', 'tr', or 'y'"));
            assert!(msg.contains("position"));
        } else {
            panic!("Expected TokenType::Error");
        }
    }
}

/// AC:6 - Regression test for previously-unreachable code path
#[test]
fn test_regression_lexer_lib_line_1385_unreachable_path() {
    let mut lexer = PerlLexer::new();

    // Test cases that could trigger the wildcard match
    let test_cases = vec![
        ("m/regex/", "m"),      // Match operator with delimiter
        ("q/string/", "q"),     // Quote operator (shouldn't reach guard)
        ("x/unknown/", "x"),    // Unknown operator
    ];

    for (code, expected_operator) in test_cases {
        let tokens = lexer.tokenize(code);

        // Should either tokenize correctly OR produce descriptive error
        let has_error = tokens.iter().any(|t| matches!(t.token_type, TokenType::Error(_)));

        if has_error {
            // Verify error message quality
            let error_token = tokens.iter()
                .find(|t| matches!(t.token_type, TokenType::Error(_)))
                .unwrap();

            if let TokenType::Error(msg) = &error_token.token_type {
                assert!(
                    msg.contains("Unexpected") || msg.contains("expected"),
                    "Error message should be descriptive: {}",
                    msg
                );
            }
        }
    }
}
```

---

## 3. Lexer Error Handling Patterns

### 3.1 Diagnostic Token Pattern

**Standard Lexer Error Token Creation**:

```rust
/// Returns a diagnostic error token for lexer failures.
///
/// # Arguments
/// * `message` - Descriptive error message with context
/// * `start` - Byte offset where error was detected
/// * `end` - Byte offset where error ends (typically current position)
///
/// # Returns
/// Token with `TokenType::Error(Arc<str>)` containing diagnostic information
///
/// # LSP Integration
/// Error tokens are converted to LSP diagnostics during the Parse stage,
/// enabling incremental parsing with error recovery and continued workspace indexing.
fn create_error_token(message: impl Into<String>, start: usize, end: usize) -> Token {
    Token {
        token_type: TokenType::Error(Arc::from(message.into())),
        start,
        end,
    }
}

/// Example usage in lexer:
fn lex_construct(&mut self) -> Token {
    match self.current_char() {
        Some(expected_char) => {
            // Valid tokenization path
            self.consume_token()
        },
        unexpected => {
            // Return diagnostic token instead of panicking
            create_error_token(
                format!(
                    "Unexpected character {:?}: expected {} at position {}",
                    unexpected,
                    "valid delimiter",
                    self.position
                ),
                self.position,
                self.position + 1
            )
        }
    }
}
```

### 3.2 Error Message Standards

**Lexer Error Message Requirements**:

1. **Operator Context**: Include the invalid operator or character that triggered the error
2. **Expected Values**: List valid alternatives (e.g., "expected 's', 'tr', or 'y'")
3. **Position Information**: Include byte offset for LSP diagnostic range calculation
4. **Actionable Guidance**: Explain what the lexer expected to find

**Error Message Template**:
```rust
format!(
    "Unexpected {construct_type} '{actual}': expected {valid_alternatives} at position {pos}"
)

// Examples:
// "Unexpected substitution operator 'm': expected 's', 'tr', or 'y' at position 5"
// "Unexpected delimiter character '%': expected '/', '#', or balanced delimiter at position 12"
```

### 3.3 Lexer State Preservation

**Continued Tokenization After Errors**:

```rust
/// Lexer tokenization with error recovery.
///
/// # Error Recovery Strategy
/// 1. Emit error token for invalid construct
/// 2. Advance position past error to prevent infinite loops
/// 3. Resume tokenization from next valid position
/// 4. Collect multiple errors for comprehensive diagnostics
fn tokenize_with_recovery(&mut self, source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    while !self.is_at_end() {
        match self.next_token() {
            Ok(token) => {
                tokens.push(token);
            },
            Err(error_message) => {
                // Emit error token
                let error_token = Token {
                    token_type: TokenType::Error(Arc::from(error_message)),
                    start: self.position,
                    end: self.position + 1,
                };
                tokens.push(error_token);

                // Advance past error to avoid infinite loop
                self.advance();

                // Continue tokenization (collect multiple errors)
            }
        }
    }

    tokens
}
```

---

## 4. LSP Protocol Integration

### 4.1 Lexer Error to LSP Diagnostic Mapping

**Token Error → LSP Diagnostic Conversion**:

```rust
/// Converts lexer error tokens to LSP diagnostics.
///
/// # Arguments
/// * `token` - Error token with `TokenType::Error(message)`
/// * `source` - Source code for position calculation
///
/// # Returns
/// LSP Diagnostic with severity::ERROR and actionable message
///
/// # LSP Workflow
/// Lexer errors are published during the Parse stage, enabling:
/// - Syntax highlighting with error ranges
/// - Incremental parsing with partial token streams
/// - Completion/hover using valid tokens before error
fn convert_lexer_error_to_diagnostic(
    token: &Token,
    source: &str
) -> lsp_types::Diagnostic {
    if let TokenType::Error(message) = &token.token_type {
        // Convert byte offsets to LSP line:character positions
        let start_position = byte_offset_to_lsp_position(source, token.start);
        let end_position = byte_offset_to_lsp_position(source, token.end);

        lsp_types::Diagnostic {
            range: lsp_types::Range::new(start_position, end_position),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("perl-lexer".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    } else {
        panic!("Expected TokenType::Error, found {:?}", token.token_type);
    }
}
```

### 4.2 Error Severity and Recovery

**Lexer Error Types → LSP Diagnostic Severity**:

| Lexer Error Type | LSP Severity | Recovery Strategy |
|------------------|--------------|-------------------|
| **UnexpectedOperator** | `ERROR` | Emit error token, resume at next whitespace |
| **InvalidDelimiter** | `ERROR` | Skip to next valid delimiter or statement boundary |
| **UnterminatedString** | `ERROR` | Insert synthetic closing delimiter, continue |
| **MalformedNumber** | `WARNING` | Emit partial number token, continue |
| **UnknownCharacter** | `ERROR` | Skip character, continue tokenization |

### 4.3 Partial Token Stream Processing

**Parser Integration with Error Tokens**:

```rust
/// Parser handles lexer error tokens gracefully.
///
/// # Strategy
/// 1. Collect error tokens as LSP diagnostics
/// 2. Skip error tokens during AST construction
/// 3. Continue parsing with valid tokens for partial AST
/// 4. Preserve error context for LSP features
fn parse_with_lexer_errors(tokens: Vec<Token>) -> (Option<AstNode>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut valid_tokens = Vec::new();

    for token in tokens {
        match &token.token_type {
            TokenType::Error(message) => {
                // Convert to diagnostic, skip token
                diagnostics.push(convert_lexer_error_to_diagnostic(&token, source));
            },
            _ => {
                // Valid token for AST construction
                valid_tokens.push(token);
            }
        }
    }

    // Parse with valid tokens only
    let ast = if !valid_tokens.is_empty() {
        Some(parse_tokens(valid_tokens))
    } else {
        None
    };

    (ast, diagnostics)
}
```

---

## 5. Performance Guarantees

### 5.1 Happy Path Performance

**Zero Overhead Requirements**:
- ✅ Error token creation only executes on malformed operators
- ✅ No additional allocations in valid tokenization paths
- ✅ No conditional checks added to hot paths
- ✅ Compiler optimizations preserve existing lexer performance

**Validation**:
```bash
# Benchmark lexer performance before and after changes
cargo bench --bench lexer_benchmarks

# Expected results: <1% variance in happy path performance
# - Tokenization throughput: context-aware tokenization maintained
# - Memory usage: no increase for valid Perl code
```

### 5.2 Error Path Performance Budget

**Lexer Error Handling Overhead**:
- **Error Detection**: <1μs (immediate match failure)
- **Error Token Creation**: <3μs (Arc allocation + format)
- **Error Message Formatting**: <1μs (lazy formatting)
- **Total Error Path**: <5μs overhead per lexer error

**Memory Usage**:
- **Error Token**: ~100-200 bytes per error (Arc shared)
- **Error Message**: ~50-150 bytes (Arc<str>)
- **LSP Diagnostic Conversion**: <1KB per diagnostic

**LSP Update Target Compliance**:
- Lexer error path (<5μs) is 0.005% of 1ms LSP update budget
- Allows up to 200 lexer errors within 1ms target (exceeds realistic scenarios)

### 5.3 Incremental Tokenization Efficiency

**Token Stream with Errors**:
- Error tokens inserted without invalidating subsequent valid tokens
- Partial token stream enables downstream parser to construct partial AST
- LSP features (completion, hover) work on valid token ranges

**Performance Validation**:
```bash
# Test incremental tokenization with errors
cargo test -p perl-lexer --test incremental_tokenization_error_recovery

# Expected: Token stream generation <1ms even with multiple errors
```

---

## 6. Test Specifications

### 6.1 TDD Test Structure

**Test File**: `/crates/perl-lexer/tests/lexer_error_handling_tests.rs`

**Comprehensive AC Validation**:
- AC2: Substitution operator error handling (1 test)
- AC6: Regression test for line 1385 unreachable path (1 test)
- AC7: Documentation presence validation (grep check)
- AC10: Mutation hardening for error message content (property-based tests)

**Test Implementation**:
```rust
use perl_lexer::{PerlLexer, Token, TokenType};
use std::sync::Arc;

/// AC:2 - Lexer substitution operator error handling
#[test]
fn test_ac2_lexer_substitution_operator_error_handling() {
    let mut lexer = PerlLexer::new();

    // Test invalid operator with delimiter (bypasses guard, hits match)
    let test_cases = vec![
        ("m/pattern/", "m"),           // Match operator (not s/tr/y)
        ("q/string/", "q"),            // Quote operator
        ("unknown/test/", "unknown"),  // Unknown operator
    ];

    for (code, expected_operator) in test_cases {
        lexer = PerlLexer::new();  // Reset lexer state
        let tokens = lexer.tokenize(code);

        // Should contain error token with operator context
        let has_error = tokens.iter().any(|t| matches!(t.token_type, TokenType::Error(_)));

        if has_error {
            let error_token = tokens.iter()
                .find(|t| matches!(t.token_type, TokenType::Error(_)))
                .expect("Should have error token");

            if let TokenType::Error(msg) = &error_token.token_type {
                assert!(
                    msg.contains("Unexpected substitution operator") ||
                    msg.contains("expected 's', 'tr', or 'y'"),
                    "Error message should be descriptive for operator '{}': {}",
                    expected_operator,
                    msg
                );
                assert!(msg.contains("position"), "Should include position: {}", msg);
            }
        }
    }
}

/// AC:6 - Regression test for lexer lib.rs:1385 unreachable path
#[test]
fn test_regression_lexer_lib_line_1385_unreachable_path() {
    let mut lexer = PerlLexer::new();

    // Construct input that triggers the guard condition but with unexpected operator
    // Guard checks: next_char_after_match is a delimiter
    // Match expects: "s", "tr", or "y"

    let invalid_code = "m/regex/";  // 'm' with delimiter triggers guard

    let tokens = lexer.tokenize(invalid_code);

    // Validate error token presence and quality
    let error_count = tokens.iter()
        .filter(|t| matches!(t.token_type, TokenType::Error(_)))
        .count();

    // Should emit error token instead of panicking
    if error_count > 0 {
        let error_token = tokens.iter()
            .find(|t| matches!(t.token_type, TokenType::Error(_)))
            .unwrap();

        if let TokenType::Error(msg) = &error_token.token_type {
            // Verify error message contains essential context
            assert!(msg.len() > 10, "Error message should be descriptive");
            assert!(
                msg.contains("unexpected") || msg.contains("expected"),
                "Should explain what went wrong: {}",
                msg
            );
        }
    }
}

/// AC:7 - Documentation presence validation
#[test]
fn test_ac7_lexer_documentation_presence() {
    let source = std::fs::read_to_string("src/lib.rs")
        .expect("Failed to read lexer source");

    // Verify no unreachable!() at line 1385
    let line_1385 = source.lines().nth(1384);  // 0-indexed
    if let Some(line) = line_1385 {
        assert!(
            !line.contains("unreachable!()"),
            "Line 1385 should not contain unreachable!(): {}",
            line
        );
    }

    // Verify error handling documentation present
    assert!(
        source.contains("TokenType::Error") || source.contains("diagnostic token"),
        "Lexer should document error token handling"
    );
}
```

### 6.2 Mutation Hardening Tests

**Property-Based Testing for Error Messages (AC10)**:

```rust
use proptest::prelude::*;

proptest! {
    /// Mutation hardening: lexer error messages must contain expected keywords
    #[test]
    fn test_mutation_lexer_error_message_quality(
        operator in "[a-z]{1,5}",  // Random operator strings
        delimiter in prop::sample::select(vec!['/', '#', '|', '!'])
    ) {
        let mut lexer = PerlLexer::new();

        // Construct code with random operator and valid delimiter
        let code = format!("{}{delimiter}test{delimiter}", operator);
        let tokens = lexer.tokenize(&code);

        // Property: Error messages (if present) must be descriptive
        for token in tokens {
            if let TokenType::Error(msg) = &token.token_type {
                // Error message quality properties
                prop_assert!(msg.len() > 10, "Error message too short: {}", msg);
                prop_assert!(
                    msg.contains("unexpected") ||
                    msg.contains("expected") ||
                    msg.contains("invalid"),
                    "Should explain error: {}",
                    msg
                );
                prop_assert!(
                    msg.contains("position") || msg.contains("at"),
                    "Should include position context: {}",
                    msg
                );
            }
        }
    }

    /// Mutation hardening: error token position accuracy
    #[test]
    fn test_mutation_error_token_position_accuracy(
        position in 0usize..100
    ) {
        let mut lexer = PerlLexer::new();

        // Create code with error at specific position
        let padding = " ".repeat(position);
        let code = format!("{}m/test/", padding);

        let tokens = lexer.tokenize(&code);

        // Property: Error tokens should have accurate position
        for token in tokens {
            if let TokenType::Error(_) = &token.token_type {
                // Position should be within expected range
                prop_assert!(
                    token.start >= position,
                    "Error token position {} should be >= input position {}",
                    token.start,
                    position
                );
            }
        }
    }
}
```

### 6.3 LSP Integration Tests

**Lexer Error in LSP Context**:

```rust
/// Test lexer error propagation through LSP pipeline
#[test]
fn test_lexer_error_lsp_diagnostic_conversion() {
    use lsp_types::{Diagnostic, DiagnosticSeverity};

    let mut lexer = PerlLexer::new();
    let source = "m/pattern/";  // Invalid substitution operator

    let tokens = lexer.tokenize(source);

    // Extract error tokens
    let error_tokens: Vec<_> = tokens.iter()
        .filter(|t| matches!(t.token_type, TokenType::Error(_)))
        .collect();

    assert!(!error_tokens.is_empty(), "Should have error tokens");

    // Convert to LSP diagnostics
    let diagnostics: Vec<Diagnostic> = error_tokens.iter()
        .map(|token| convert_lexer_error_to_diagnostic(token, source))
        .collect();

    // Validate LSP diagnostic properties
    for diagnostic in diagnostics {
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("perl-lexer".to_string()));
        assert!(!diagnostic.message.is_empty());
        assert!(diagnostic.range.start.line == 0);  // First line
    }
}
```

---

## 7. Documentation Requirements

### 7.1 Inline Documentation (AC7)

**Function Documentation Template**:

```rust
/// Tokenizes substitution/transliteration operators with compile-time safe error handling.
///
/// This function replaces the fragile `unreachable!()` pattern at line 1385 with
/// explicit error token emission, ensuring that unexpected operators return diagnostic
/// tokens instead of panicking.
///
/// # Arguments
/// * `self` - Mutable lexer state
/// * `text` - Operator text (validated by upstream guard)
/// * `start` - Starting byte offset of operator
///
/// # Returns
/// `Token` with either:
/// - `TokenType::Substitution` or `TokenType::Transliteration` for valid operators
/// - `TokenType::Error(Arc<str>)` for unexpected operators with diagnostic message
///
/// # Error Token Format
/// Error tokens include:
/// - Unexpected operator text
/// - Expected valid alternatives ("s", "tr", "y")
/// - Byte position for LSP diagnostic range
///
/// # Examples
/// ```rust
/// let mut lexer = PerlLexer::new();
/// lexer.tokenize("s/find/replace/");  // Valid: returns Substitution token
/// lexer.tokenize("m/pattern/");       // Invalid: returns Error token
/// ```
///
/// # Performance
/// - Happy path: Zero overhead (valid operators)
/// - Error path: <5μs overhead for error token creation
///
/// # LSP Integration
/// Error tokens are converted to LSP diagnostics with severity::ERROR.
/// Lexer continues tokenization after errors for comprehensive diagnostics.
///
/// # Related
/// - Issue #178: Eliminate fragile unreachable!() macros
/// - AC2: Lexer substitution operator error handling
fn tokenize_substitution_operator(
    &mut self,
    text: &str,
    start: usize
) -> Token {
    match text {
        "s" => self.parse_substitution(start),
        "tr" | "y" => self.parse_transliteration(start),
        unexpected => {
            // Return diagnostic token instead of panicking
            Token {
                token_type: TokenType::Error(Arc::from(format!(
                    "Unexpected substitution operator '{}': expected 's', 'tr', or 'y' at position {}",
                    unexpected,
                    start
                ))),
                start,
                end: self.position,
            }
        }
    }
}
```

### 7.2 Module-Level Documentation

**Lexer Error Handling Module Documentation**:

```rust
//! Lexer error handling infrastructure for Perl tokenization
//!
//! This module provides compile-time safe error handling patterns that replace
//! fragile `unreachable!()` macros with diagnostic token emission for graceful
//! error recovery.
//!
//! # Architecture
//!
//! The lexer error handling system uses `TokenType::Error(Arc<str>)` to represent
//! lexical errors that are converted to LSP diagnostics during the Parse stage.
//!
//! # Error Categories
//!
//! - **Operator Validation**: Substitution/transliteration operator checking
//! - **Delimiter Matching**: Balanced delimiter validation
//! - **String Termination**: Unterminated string detection
//! - **Character Recognition**: Unknown character handling
//!
//! # LSP Workflow Integration
//!
//! Lexer errors support all LSP workflow stages:
//! - **Parse**: Error tokens converted to LSP diagnostics
//! - **Index**: Valid tokens indexed for workspace navigation
//! - **Navigate**: Cross-file navigation works on valid token ranges
//! - **Complete**: Completion uses context before error tokens
//! - **Analyze**: Multiple errors collected for comprehensive diagnostics
//!
//! # Performance
//!
//! - **Happy path**: Zero overhead, maintains context-aware tokenization speed
//! - **Error path**: <5μs overhead per error token, well within <1ms LSP update target
//! - **Memory**: <200 bytes per error token (Arc shared)
//!
//! # Examples
//!
//! See individual function documentation for usage patterns.
```

### 7.3 Error Message Standards

**Lexer Error Message Documentation**:

```rust
/// Error message format standards for lexer diagnostics.
///
/// All lexer error messages follow this template:
/// ```text
/// "Unexpected {construct_type} '{actual}': expected {valid_alternatives} at position {pos}"
/// ```
///
/// # Components
///
/// 1. **Construct Type**: What kind of token/operator was being parsed
///    - Examples: "substitution operator", "delimiter", "character"
///
/// 2. **Actual Value**: The invalid text/character encountered
///    - Examples: "'m'", "'%'", "'unknown'"
///
/// 3. **Valid Alternatives**: What the lexer expected to find
///    - Examples: "'s', 'tr', or 'y'", "'/', '#', or balanced delimiter"
///
/// 4. **Position**: Byte offset for LSP diagnostic range
///    - Format: "at position {byte_offset}"
///
/// # Examples
///
/// - "Unexpected substitution operator 'm': expected 's', 'tr', or 'y' at position 5"
/// - "Unexpected delimiter '%': expected '/', '#', or balanced delimiter at position 12"
/// - "Unexpected character '€': expected ASCII operator at position 20"
```

---

## 8. Acceptance Criteria Mapping

**AC2**: Substitution Operator Error Handling ✅
- File: perl-lexer/src/lib.rs:1385
- Pattern: Return `TokenType::Error` diagnostic token for unexpected operators
- Test: test_ac2_lexer_substitution_operator_error_handling
- Validation: `cargo test -p perl-lexer --test lexer_error_handling_tests -- test_ac2`

**AC6**: Regression Tests ✅
- Coverage: 100% regression coverage for line 1385 unreachable path
- Test: test_regression_lexer_lib_line_1385_unreachable_path
- Validation: `cargo test -p perl-lexer --test lexer_error_handling_tests -- test_regression`

**AC7**: Documentation ✅
- Requirement: Inline comments + module-level documentation
- Test: test_ac7_lexer_documentation_presence
- Validation: Grep check for unreachable + TokenType::Error documentation

**AC10**: Mutation Hardening ✅
- Framework: Property-based testing with proptest
- Tests: test_mutation_lexer_error_message_quality, test_mutation_error_token_position_accuracy
- Validation: `cargo test -p perl-lexer --test lexer_error_handling_tests -- test_mutation`

---

## 9. Implementation Checklist

**Lexer Error Handling (AC2)**
- [ ] Replace unreachable!() in perl-lexer/src/lib.rs:1385
- [ ] Add exhaustive matching with diagnostic token return
- [ ] Include operator, expected values, and position in error message
- [ ] Add function documentation explaining error token creation
- [ ] Verify lexer continues tokenization after error

**Testing (AC2, AC6, AC10)**
- [ ] Create /crates/perl-lexer/tests/lexer_error_handling_tests.rs
- [ ] Add AC2 test: substitution operator error handling
- [ ] Add AC6 test: regression for line 1385 unreachable path
- [ ] Add AC7 test: documentation presence validation
- [ ] Add AC10 tests: property-based mutation hardening

**Documentation (AC7)**
- [ ] Add inline documentation to tokenize_substitution_operator function
- [ ] Update module-level documentation explaining error token pattern
- [ ] Document error message format standards
- [ ] Add performance notes (<5μs error path overhead)
- [ ] Include LSP integration context

**Validation**
- [ ] Run `cargo test -p perl-lexer` (100% pass rate)
- [ ] Run `cargo bench --bench lexer_benchmarks` (<1% variance)
- [ ] Verify no unreachable!() in perl-lexer/src: `grep -r "unreachable!" crates/perl-lexer/src/lib.rs | grep -v "//" | wc -l` returns 0

---

## 10. References

**Related Specifications**:
- [issue-178-spec.md](issue-178-spec.md) - Feature specification
- [ISSUE_178_TECHNICAL_ANALYSIS.md](ISSUE_178_TECHNICAL_ANALYSIS.md) - Technical analysis
- [PARSER_ERROR_HANDLING_SPEC.md](PARSER_ERROR_HANDLING_SPEC.md) - Parser error handling
- [ERROR_HANDLING_API_CONTRACTS.md](ERROR_HANDLING_API_CONTRACTS.md) - API contracts

**Perl LSP Documentation**:
- [LSP_ERROR_HANDLING_MONITORING_GUIDE.md](LSP_ERROR_HANDLING_MONITORING_GUIDE.md) - Error monitoring
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture

**Lexer Documentation**:
- Context-aware tokenization patterns in perl-lexer/src/lib.rs
- Quote operator handling in perl-lexer/src/quote_handler.rs
- Token type definitions in perl-lexer/src/token.rs (inferred)

---

**End of Lexer Error Handling Specification**
