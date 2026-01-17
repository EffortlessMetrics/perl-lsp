# Error Handling API Contracts (*Diataxis: Reference*)

**Issue**: #178 (GitHub #204) - Eliminate Fragile unreachable!() Macros
**Related Specs**: [issue-178-spec.md](issue-178-spec.md), [PARSER_ERROR_HANDLING_SPEC.md](PARSER_ERROR_HANDLING_SPEC.md), [LEXER_ERROR_HANDLING_SPEC.md](LEXER_ERROR_HANDLING_SPEC.md)
**LSP Workflow**: Parse → Index → Navigate → Complete → Analyze
**Crate Scope**: perl-parser, perl-lexer, tree-sitter-perl-rs

---

## 1. Executive Summary

This specification defines the comprehensive error handling API contracts for the Perl parser/lexer ecosystem. It establishes consistent error types, result patterns, and LSP diagnostic mapping standards to ensure compile-time safety and graceful runtime degradation across all parser components.

**Key Contracts**:
- **Parser Result Types**: `Result<AstNode, String>` for simple parsers, `Result<AstNode, Simple<Token>>` for combinators
- **Lexer Token Types**: `Token` with `TokenType::Error(Arc<str>)` for diagnostic emission
- **LSP Error Mapping**: `ParseError` → `lsp_types::Diagnostic` with JSON-RPC 2.0 error codes
- **Anti-Pattern Handling**: Panic with descriptive messages OR fallback diagnostics

---

## 2. Parser Error Contracts

### 2.1 Simple Parser Result Type

**Contract**: `Result<AstNode, String>`

**Definition**:
```rust
/// Parser result type for simple recursive descent parsers.
///
/// # Type Parameters
/// - `T`: Success type (typically `AstNode` or specialized AST node)
/// - `E`: Error type (always `String` for simple parsers)
///
/// # Error Format
/// Error strings MUST include:
/// 1. **Expected construct**: What the parser expected to find
/// 2. **Found construct**: What was actually encountered
/// 3. **Position information**: Byte offset or line:column
///
/// # Examples
/// ```rust
/// fn parse_declaration(&mut self) -> Result<AstNode, String> {
///     match self.current_token() {
///         Token::My => Ok(/* ... */),
///         unexpected => Err(format!(
///             "Expected 'my/our/local/state', found {:?} at position {}",
///             unexpected,
///             self.current_position()
///         ))
///     }
/// }
/// ```
pub type SimpleParserResult<T> = Result<T, String>;
```

**API Requirements**:
- ✅ **Error Message Format**: `"Expected {expected}, found {found} at position {pos}"`
- ✅ **Position Inclusion**: Always include byte offset in error messages
- ✅ **Context Preservation**: Error messages provide enough context for LSP diagnostics
- ✅ **Recovery Strategy**: Errors should suggest valid alternatives

**Usage Contexts**:
- Variable declaration parsing (simple_parser_v2.rs, simple_parser.rs)
- Lexical construct validation
- Token matching with explicit error handling

### 2.2 Parser Combinator Result Type

**Contract**: `Result<AstNode, Simple<Token>>`

**Definition**:
```rust
use chumsky::error::Simple;

/// Parser combinator result type using Chumsky error infrastructure.
///
/// # Type Parameters
/// - `T`: Success type (typically `AstNode`)
/// - `E`: Error type (always `Simple<Token>` for combinator parsers)
///
/// # Error Construction
/// ```rust
/// Err(Simple::custom(
///     span,
///     format!(
///         "Invalid {construct}: {explanation}. \
///          Expected: {valid_forms}, Found: {actual} at position {}",
///         span.start
///     )
/// ))
/// ```
pub type CombinatorParserResult<T> = Result<T, Simple<Token>>;
```

**API Requirements**:
- ✅ **Span Information**: Include `Span` for LSP range calculation
- ✅ **Custom Error Messages**: Use `Simple::custom()` for descriptive errors
- ✅ **Structural Context**: Explain valid vs invalid structural forms
- ✅ **Position Tracking**: Span provides start/end byte offsets

**Usage Contexts**:
- For-loop tuple validation (token_parser.rs:284)
- Complex expression parsing
- Control flow structure validation

### 2.3 ParseError Enum Contract

**Contract**: `ParseError` enum with comprehensive error variants

**Definition**:
```rust
use thiserror::Error;

/// Comprehensive error types for Perl parsing operations.
///
/// # Variants
/// - `UnexpectedEof`: Incomplete input during parsing
/// - `UnexpectedToken`: Token mismatch with expected/found context
/// - `SyntaxError`: General syntax errors with position
/// - `LexerError`: Tokenization failures
/// - `RecursionLimit`: Parser recursion depth exceeded
/// - `InvalidNumber`: Malformed numeric literals
/// - `InvalidString`: Malformed string literals
/// - `UnclosedDelimiter`: Unclosed delimiter detection
/// - `InvalidRegex`: Regex syntax errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Unexpected token: expected {expected}, found {found} at {location}")]
    UnexpectedToken {
        expected: String,
        found: String,
        location: usize,
    },

    #[error("Invalid syntax at position {location}: {message}")]
    SyntaxError {
        message: String,
        location: usize,
    },

    #[error("Lexer error: {message}")]
    LexerError {
        message: String,
    },

    #[error("Maximum recursion depth exceeded")]
    RecursionLimit,

    #[error("Invalid number literal: {literal}")]
    InvalidNumber {
        literal: String,
    },

    #[error("Invalid string literal")]
    InvalidString,

    #[error("Unclosed delimiter: {delimiter}")]
    UnclosedDelimiter {
        delimiter: char,
    },

    #[error("Invalid regex: {message}")]
    InvalidRegex {
        message: String,
    },
}

/// Result type alias for parser operations
pub type ParseResult<T> = Result<T, ParseError>;
```

**API Requirements**:
- ✅ **thiserror Integration**: Use `#[error]` attribute for Display implementation
- ✅ **Clone + PartialEq**: Support error comparison and cloning for diagnostics
- ✅ **Location Context**: Include byte offsets in position-sensitive variants
- ✅ **Message Clarity**: Error messages explain what went wrong and why

**Constructor Patterns**:
```rust
impl ParseError {
    /// Create syntax error with position context
    pub fn syntax(message: impl Into<String>, location: usize) -> Self {
        ParseError::SyntaxError {
            message: message.into(),
            location,
        }
    }

    /// Create unexpected token error
    pub fn unexpected_token(
        expected: impl Into<String>,
        found: impl Into<String>,
        location: usize
    ) -> Self {
        ParseError::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            location,
        }
    }

    /// Create lexer error
    pub fn lexer(message: impl Into<String>) -> Self {
        ParseError::LexerError {
            message: message.into(),
        }
    }
}
```

---

## 3. Lexer Error Contracts

### 3.1 Token Error Type

**Contract**: `Token` with `TokenType::Error(Arc<str>)`

**Definition**:
```rust
use std::sync::Arc;

/// Token type for lexer error reporting.
///
/// # Error Token Structure
/// ```rust
/// Token {
///     token_type: TokenType::Error(Arc::from(message)),
///     start: byte_offset,
///     end: byte_offset + length,
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // ... other token types ...

    /// Error token with diagnostic message
    Error(Arc<str>),
}
```

**API Requirements**:
- ✅ **Arc<str> Storage**: Efficient shared string storage for error messages
- ✅ **Position Range**: `start` and `end` byte offsets for LSP range
- ✅ **Error Message Format**: Same as parser errors - "Unexpected X: expected Y at position Z"
- ✅ **Continuation**: Lexer continues after emitting error token

**Error Token Creation**:
```rust
/// Create diagnostic error token.
///
/// # Arguments
/// * `message` - Error message with context
/// * `start` - Starting byte offset
/// * `end` - Ending byte offset
///
/// # Returns
/// Token with `TokenType::Error` variant
fn create_error_token(
    message: impl Into<String>,
    start: usize,
    end: usize
) -> Token {
    Token {
        token_type: TokenType::Error(Arc::from(message.into())),
        start,
        end,
    }
}

/// Example usage:
fn tokenize_operator(&mut self, text: &str, start: usize) -> Token {
    match text {
        "s" => self.parse_substitution(start),
        "tr" | "y" => self.parse_transliteration(start),
        unexpected => create_error_token(
            format!(
                "Unexpected substitution operator '{}': expected 's', 'tr', or 'y' at position {}",
                unexpected,
                start
            ),
            start,
            self.position
        )
    }
}
```

### 3.2 Lexer Error Recovery Contract

**Contract**: Continue tokenization after error emission

**Pattern**:
```rust
/// Tokenize with error recovery.
///
/// # Error Recovery Strategy
/// 1. Emit error token for invalid construct
/// 2. Advance position past error
/// 3. Resume tokenization from next valid position
/// 4. Collect multiple errors for comprehensive diagnostics
pub fn tokenize(&mut self, source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    while !self.is_at_end() {
        match self.next_token() {
            Ok(token) => {
                tokens.push(token);
            },
            Err(error_message) => {
                // Emit error token
                let error_token = create_error_token(
                    error_message,
                    self.position,
                    self.position + 1
                );
                tokens.push(error_token);

                // Advance past error to prevent infinite loop
                self.advance();

                // Continue tokenization
            }
        }
    }

    tokens
}
```

**API Requirements**:
- ✅ **Non-Panicking**: Lexer never panics, only emits error tokens
- ✅ **Position Advancement**: Always advance position after error to prevent loops
- ✅ **Multiple Errors**: Collect all lexer errors in single pass
- ✅ **Token Stream Validity**: Error tokens integrate seamlessly with valid tokens

---

## 4. LSP Error Mapping Contracts

### 4.1 Parser Error to LSP Diagnostic

**Contract**: `ParseError` → `lsp_types::Diagnostic`

**Conversion Function**:
```rust
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Convert parser errors to LSP diagnostics.
///
/// # Arguments
/// * `error` - Parser error variant
/// * `source` - Source code for position calculation
///
/// # Returns
/// LSP Diagnostic with severity, range, and message
pub fn parser_error_to_diagnostic(
    error: &ParseError,
    source: &str
) -> Diagnostic {
    let (severity, range, message) = match error {
        ParseError::UnexpectedEof => (
            DiagnosticSeverity::ERROR,
            calculate_eof_range(source),
            "Unexpected end of input".to_string()
        ),

        ParseError::UnexpectedToken { expected, found, location } => (
            DiagnosticSeverity::ERROR,
            byte_offset_to_range(source, *location),
            format!("Expected {}, found {}", expected, found)
        ),

        ParseError::SyntaxError { message, location } => (
            DiagnosticSeverity::ERROR,
            byte_offset_to_range(source, *location),
            message.clone()
        ),

        ParseError::LexerError { message } => (
            DiagnosticSeverity::ERROR,
            Range::default(),  // Lexer errors may not have specific position
            message.clone()
        ),

        ParseError::RecursionLimit => (
            DiagnosticSeverity::WARNING,
            Range::default(),
            "Maximum recursion depth exceeded".to_string()
        ),

        ParseError::InvalidNumber { literal } => (
            DiagnosticSeverity::ERROR,
            Range::default(),  // Position inferred from context
            format!("Invalid number literal: {}", literal)
        ),

        ParseError::InvalidString => (
            DiagnosticSeverity::ERROR,
            Range::default(),
            "Invalid string literal".to_string()
        ),

        ParseError::UnclosedDelimiter { delimiter } => (
            DiagnosticSeverity::ERROR,
            Range::default(),
            format!("Unclosed delimiter: {}", delimiter)
        ),

        ParseError::InvalidRegex { message } => (
            DiagnosticSeverity::ERROR,
            Range::default(),
            format!("Invalid regex: {}", message)
        ),
    };

    Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        code_description: None,
        source: Some("perl-parser".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Calculate LSP Range from byte offset
fn byte_offset_to_range(source: &str, offset: usize) -> Range {
    let position = byte_offset_to_lsp_position(source, offset);
    Range::new(position, position)
}

/// Convert byte offset to LSP Position (line, character)
fn byte_offset_to_lsp_position(source: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position::new(line as u32, character as u32)
}
```

**API Requirements**:
- ✅ **Severity Mapping**: Errors → ERROR, Warnings → WARNING
- ✅ **Range Calculation**: Convert byte offsets to line:character positions
- ✅ **Source Attribution**: Always set `source: Some("perl-parser")`
- ✅ **Message Preservation**: Use original error messages

### 4.2 Lexer Error to LSP Diagnostic

**Contract**: `Token::Error` → `lsp_types::Diagnostic`

**Conversion Function**:
```rust
/// Convert lexer error tokens to LSP diagnostics.
///
/// # Arguments
/// * `token` - Error token with `TokenType::Error(message)`
/// * `source` - Source code for position calculation
///
/// # Returns
/// LSP Diagnostic with ERROR severity
pub fn lexer_error_to_diagnostic(
    token: &Token,
    source: &str
) -> Diagnostic {
    if let TokenType::Error(message) = &token.token_type {
        let start_position = byte_offset_to_lsp_position(source, token.start);
        let end_position = byte_offset_to_lsp_position(source, token.end);

        Diagnostic {
            range: Range::new(start_position, end_position),
            severity: Some(DiagnosticSeverity::ERROR),
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

**API Requirements**:
- ✅ **Range Precision**: Use token start/end for accurate ranges
- ✅ **Source Attribution**: Always set `source: Some("perl-lexer")`
- ✅ **Error Severity**: All lexer errors are DiagnosticSeverity::ERROR
- ✅ **Message Format**: Preserve Arc<str> message content

### 4.3 JSON-RPC Error Code Mapping

**Contract**: Parser errors → JSON-RPC 2.0 error codes

**Mapping Table**:

| Parser Error Type | JSON-RPC Error Code | LSP Severity | Description |
|-------------------|---------------------|--------------|-------------|
| **UnexpectedEof** | -32603 (Internal Error) | ERROR | Incomplete input during parsing |
| **UnexpectedToken** | -32603 (Internal Error) | ERROR | Token mismatch with expected/found |
| **SyntaxError** | -32603 (Internal Error) | ERROR | General syntax errors |
| **LexerError** | -32700 (Parse Error) | ERROR | Tokenization failures |
| **RecursionLimit** | -32603 (Internal Error) | WARNING | Parser recursion depth exceeded |
| **InvalidNumber** | -32602 (Invalid Params) | ERROR | Malformed numeric literals |
| **InvalidString** | -32602 (Invalid Params) | ERROR | Malformed string literals |
| **UnclosedDelimiter** | -32603 (Internal Error) | ERROR | Unclosed delimiter detection |
| **InvalidRegex** | -32602 (Invalid Params) | ERROR | Regex syntax errors |

**LSP Error Response**:
```rust
use crate::lsp_server::JsonRpcError;
use crate::lsp_errors::error_codes;

/// Convert ParseError to JSON-RPC error response.
pub fn parser_error_to_jsonrpc(error: &ParseError) -> JsonRpcError {
    let (code, message) = match error {
        ParseError::LexerError { message } => (
            error_codes::PARSE_ERROR,  // -32700
            format!("Lexer error: {}", message)
        ),

        ParseError::InvalidNumber { literal } => (
            error_codes::INVALID_PARAMS,  // -32602
            format!("Invalid number literal: {}", literal)
        ),

        ParseError::InvalidString => (
            error_codes::INVALID_PARAMS,  // -32602
            "Invalid string literal".to_string()
        ),

        ParseError::InvalidRegex { message } => (
            error_codes::INVALID_PARAMS,  // -32602
            format!("Invalid regex: {}", message)
        ),

        _ => (
            error_codes::INTERNAL_ERROR,  // -32603
            error.to_string()
        ),
    };

    JsonRpcError {
        code,
        message,
        data: None,
    }
}
```

---

## 5. Anti-Pattern Detector Error Contracts

### 5.1 Diagnostic Panic Contract (Recommended)

**Contract**: Panic with descriptive message for programming errors

**Pattern**:
```rust
/// Anti-pattern detector diagnose method with type-safe panic.
///
/// # Arguments
/// * `pattern` - Anti-pattern enum variant specific to this detector
///
/// # Returns
/// Diagnostic for the anti-pattern
///
/// # Panics
/// Panics if pattern type doesn't match detector expectations.
/// This indicates a programming error in the detection pipeline.
fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
    let AntiPattern::FormatHeredoc { format_name, location } = pattern else {
        // Descriptive panic for programming errors
        panic!(
            "FormatHeredocDetector received incompatible pattern type: {:?}. \
             This indicates a bug in the anti-pattern detection pipeline. \
             Expected: AntiPattern::FormatHeredoc, Found: {:?}",
            pattern,
            std::mem::discriminant(pattern)
        );
    };

    Diagnostic {
        severity: Severity::Warning,
        pattern: pattern.clone(),
        message: format!("Format '{}' uses heredoc syntax", format_name),
        explanation: "Perl formats are deprecated since Perl 5.8...".to_string(),
        suggested_fix: Some("Consider using sprintf, printf...".to_string()),
        references: vec![
            "perldoc perlform".to_string(),
            "https://perldoc.perl.org/perldiag...".to_string(),
        ],
    }
}
```

**API Requirements**:
- ✅ **let-else Pattern**: Use Rust 1.65+ let-else for exhaustive matching
- ✅ **Descriptive Panic**: Explain detector mismatch is a programming bug
- ✅ **Discriminant Logging**: Include `std::mem::discriminant()` for debugging
- ✅ **Expected vs Found**: Clearly state expected pattern type

### 5.2 Fallback Diagnostic Contract (Ultra-Defensive)

**Contract**: Return fallback diagnostic for pattern type mismatch

**Pattern**:
```rust
/// Anti-pattern detector with fallback diagnostic.
///
/// # Arguments
/// * `pattern` - Anti-pattern enum variant
///
/// # Returns
/// Diagnostic for the anti-pattern or internal error diagnostic
fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
    match pattern {
        AntiPattern::FormatHeredoc { format_name, location } => {
            Diagnostic {
                severity: Severity::Warning,
                pattern: pattern.clone(),
                message: format!("Format '{}' uses heredoc syntax", format_name),
                explanation: "Perl formats are deprecated since Perl 5.8...".to_string(),
                suggested_fix: Some("Consider using sprintf, printf...".to_string()),
                references: vec![
                    "perldoc perlform".to_string(),
                    "https://perldoc.perl.org/perldiag...".to_string(),
                ],
            }
        },
        unexpected => {
            // Fallback diagnostic for programming errors
            Diagnostic {
                severity: Severity::Error,
                pattern: pattern.clone(),
                message: format!(
                    "Internal error: FormatHeredocDetector received incompatible pattern: {:?}",
                    unexpected
                ),
                explanation: "This is a bug in the anti-pattern detection system.".to_string(),
                suggested_fix: None,
                references: vec![],
            }
        }
    }
}
```

**API Requirements**:
- ✅ **Match Exhaustiveness**: Use match with wildcard for all pattern types
- ✅ **Fallback Diagnostic**: Return internal error diagnostic instead of panicking
- ✅ **Severity Escalation**: Use Severity::Error for internal errors
- ✅ **No Suggested Fix**: Internal errors don't have user-actionable fixes

---

## 6. Error Message Format Standards

### 6.1 Parser Error Message Template

**Format**: `"Expected {expected}, found {found} at position {pos}"`

**Components**:
1. **Expected Construct**: What the parser expected (e.g., "variable declaration keyword", "'my/our/local/state'")
2. **Found Construct**: What was actually encountered (e.g., "Token::Return", "'return'")
3. **Position Information**: Byte offset or line:column (e.g., "at position 42", "at line 5, column 10")

**Examples**:
- `"Expected variable declaration keyword (my/our/local/state), found Token::Return at position 42"`
- `"Expected ';' to end statement, found Token::RBrace at line 5, column 10"`
- `"Expected expression after '=', found end of input at position 100"`

### 6.2 Lexer Error Message Template

**Format**: `"Unexpected {construct_type} '{actual}': expected {valid_alternatives} at position {pos}"`

**Components**:
1. **Construct Type**: Type of token/operator being parsed (e.g., "substitution operator", "delimiter")
2. **Actual Value**: Invalid text/character encountered (e.g., "'m'", "'%'")
3. **Valid Alternatives**: What the lexer expected (e.g., "'s', 'tr', or 'y'", "'/', '#', or balanced delimiter")
4. **Position Information**: Byte offset (e.g., "at position 5")

**Examples**:
- `"Unexpected substitution operator 'm': expected 's', 'tr', or 'y' at position 5"`
- `"Unexpected delimiter '%': expected '/', '#', or balanced delimiter at position 12"`
- `"Unexpected character '€': expected ASCII operator at position 20"`

### 6.3 Control Flow Error Message Template

**Format**: `"Invalid {structure_type}: {explanation}. Expected: {valid_forms}, Found: {actual_form} at position {pos}"`

**Components**:
1. **Structure Type**: Type of control flow construct (e.g., "for-loop structure", "conditional expression")
2. **Explanation**: Why the structure is invalid (e.g., "for-loops require either (init; condition; update) or (variable in list)")
3. **Valid Forms**: Describe valid structural alternatives
4. **Actual Form**: What was found in the code
5. **Position Information**: Byte offset

**Examples**:
- `"Invalid for-loop structure: for-loops require either (init; condition; update) for C-style loops or (variable in list) for foreach loops, but found incompatible combination at position 50"`
- `"Invalid conditional expression: ternary operator '?' requires ':' in else branch, found end of expression at position 75"`

---

## 7. Performance Contracts

### 7.1 Happy Path Performance

**Contract**: Zero overhead in valid parsing paths

**Guarantees**:
- ✅ No additional allocations for error handling in valid code
- ✅ No conditional checks added to hot paths
- ✅ Compiler optimizations preserve existing performance
- ✅ Error handling code only executes on malformed input

**Validation**:
```bash
# Benchmark before and after error handling changes
cargo bench --bench parser_benchmarks
cargo bench --bench lexer_benchmarks

# Expected: <1% variance in happy path performance
```

### 7.2 Error Path Performance Budget

**Contract**: Bounded error handling overhead

**Budgets**:
- **Parser Error Path**: <12μs per error (string formatting + Result construction)
- **Lexer Error Path**: <5μs per error (Arc allocation + Token creation)
- **LSP Diagnostic Conversion**: <10μs per diagnostic (position calculation + struct creation)

**Total Error Budget**: <27μs per error (well within 1ms LSP update target)

**Memory Budgets**:
- **Parser Error**: <1KB per error (String + position data)
- **Lexer Error Token**: <200 bytes per token (Arc shared)
- **LSP Diagnostic**: <2KB per diagnostic (Range + message)

---

## 8. Testing Contracts

### 8.1 Error Path Testing Requirements

**Contract**: 100% coverage of error handling paths

**Test Categories**:
1. **Unit Tests**: Each error variant triggered directly
2. **Regression Tests**: Previously-unreachable paths exercised
3. **Property-Based Tests**: Error message quality validation
4. **LSP Integration Tests**: Error-to-diagnostic conversion

**Example Test Structure**:
```rust
/// Test error handling for specific AC
#[test]
fn test_ac1_variable_declaration_error_handling() {
    // Given: Parser with invalid token
    let mut parser = SimpleParserV2::new();
    parser.tokens = vec![Token::Return];

    // When: Parsing variable declaration
    let result = parser.parse_variable_declaration();

    // Then: Should return descriptive error
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Expected variable declaration keyword"));
    assert!(error.contains("my/our/local/state"));
    assert!(error.contains("position"));
}
```

### 8.2 Mutation Testing Contracts

**Contract**: Error messages must survive mutation testing

**Property Tests**:
```rust
use proptest::prelude::*;

proptest! {
    /// Property: Error messages must contain essential keywords
    #[test]
    fn test_error_message_keywords(
        invalid_token in prop::sample::select(vec![
            Token::Return, Token::If, Token::While
        ])
    ) {
        let mut parser = SimpleParserV2::new();
        parser.tokens = vec![invalid_token];

        let result = parser.parse_variable_declaration();
        prop_assert!(result.is_err());

        let error = result.unwrap_err();
        prop_assert!(error.contains("Expected"));
        prop_assert!(
            error.contains("my") || error.contains("our") ||
            error.contains("local") || error.contains("state")
        );
        prop_assert!(error.contains("position"));
    }
}
```

---

## 9. Documentation Contracts

### 9.1 Function Documentation Requirements

**Contract**: All error-returning functions must document error conditions

**Template**:
```rust
/// Function description.
///
/// # Arguments
/// * `self` - Parser state
/// * `param` - Parameter description
///
/// # Returns
/// * `Ok(AstNode)` - Success case
/// * `Err(String)` - Error with descriptive message
///
/// # Errors
/// Returns an error if:
/// - Condition 1: Specific error scenario
/// - Condition 2: Another error scenario
/// - Error message includes: expected, found, position
///
/// # Performance
/// - Happy path: Zero overhead
/// - Error path: <Xμs overhead
///
/// # LSP Integration
/// Errors are converted to LSP diagnostics with severity::ERROR
///
/// # Examples
/// ```rust
/// // Success case
/// let result = parser.parse_construct("valid input");
/// assert!(result.is_ok());
///
/// // Error case
/// let result = parser.parse_construct("invalid input");
/// assert!(result.is_err());
/// ```
fn parse_construct(&mut self) -> Result<AstNode, String> {
    // Implementation
}
```

### 9.2 Error Type Documentation Requirements

**Contract**: Error enums must document each variant's context

**Template**:
```rust
/// Error type for parser operations.
///
/// # Variants
/// Each variant represents a specific parsing failure:
/// - `VariantA`: Context and recovery strategy
/// - `VariantB`: Context and recovery strategy
///
/// # Recovery Strategies
/// See individual variant documentation for recovery approaches.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Variant documentation with:
    /// - When it occurs
    /// - How to recover
    /// - LSP mapping
    #[error("Error message template")]
    VariantA {
        /// Field documentation
        field: Type,
    },
}
```

---

## 10. Acceptance Criteria Validation

**AC1**: Parser Variable Declaration Error Handling ✅
- Result Type: `Result<AstNode, String>`
- Error Format: `"Expected {expected}, found {found} at position {pos}"`
- Validation: Test coverage + error message property tests

**AC2**: Lexer Substitution Operator Error Handling ✅
- Token Type: `Token` with `TokenType::Error(Arc<str>)`
- Error Format: `"Unexpected {operator}: expected {alternatives} at position {pos}"`
- Validation: Test coverage + continuation after error

**AC3**: For-Loop Parser Error Handling ✅
- Result Type: `Result<AstNode, Simple<Token>>`
- Error Format: Structural explanation with valid forms
- Validation: Test coverage + span information

**AC4**: Question Token Defensive Handling ✅
- Result Type: `Result<AstNode, Simple<Token>>`
- Error Format: Pratt parser explanation with internal error context
- Validation: Defensive error + documentation

**AC5**: Anti-Pattern Detector Exhaustive Matching ✅
- Pattern: let-else with descriptive panic OR match with fallback
- Error Format: Internal error diagnostic OR panic message
- Validation: Unit tests + panic catching

**AC9**: LSP Graceful Degradation ✅
- LSP Mapping: All errors → LSP diagnostics
- JSON-RPC Codes: Appropriate error codes per error type
- Validation: LSP behavioral tests + session continuity

---

## 11. References

**Related Specifications**:
- [issue-178-spec.md](issue-178-spec.md) - Feature specification
- [ISSUE_178_TECHNICAL_ANALYSIS.md](ISSUE_178_TECHNICAL_ANALYSIS.md) - Technical analysis
- [PARSER_ERROR_HANDLING_SPEC.md](PARSER_ERROR_HANDLING_SPEC.md) - Parser error handling
- [LEXER_ERROR_HANDLING_SPEC.md](LEXER_ERROR_HANDLING_SPEC.md) - Lexer error handling

**Implementation References**:
- [crates/perl-parser/src/error.rs](../crates/perl-parser/src/error.rs) - ParseError enum
- [crates/perl-parser/src/lsp_errors.rs](../crates/perl-parser/src/lsp_errors.rs) - LSP error codes
- [crates/perl-lexer/src/lib.rs](../crates/perl-lexer/src/lib.rs) - Lexer error tokens

**LSP Documentation**:
- [LSP_ERROR_HANDLING_MONITORING_GUIDE.md](LSP_ERROR_HANDLING_MONITORING_GUIDE.md) - Error monitoring
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture

---

**End of Error Handling API Contracts Specification**
