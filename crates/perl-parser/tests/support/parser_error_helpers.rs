//! Parser Error Detection Helpers
//!
//! This module provides utilities for detecting parse errors in an IDE-friendly
//! parser that uses error recovery.
//!
//! ## Background
//!
//! The perl-parser uses an **IDE-friendly error recovery** model:
//!
//! - **Old (compiler) semantics**: Bad code → `Err(ParseError)`
//! - **New (IDE) semantics**: Bad code → `Ok(ast)` with ERROR nodes embedded
//!
//! This shift means that `result.is_err()` is no longer the correct way to test
//! "did the parser detect an error?" because the parser may successfully return
//! an AST that contains ERROR nodes representing recovered parse failures.
//!
//! ## The Correct Pattern
//!
//! Use `has_parse_error()` or `assert_parse_error()` which check for:
//! - `Err(_)` return (catastrophic failures)
//! - `Ok(ast)` where AST contains ERROR nodes (recoverable failures)
//!
//! ```rust,ignore
//! // Old pattern (WRONG for IDE-friendly parser):
//! assert!(result.is_err(), "Expected error");
//!
//! // New pattern (CORRECT):
//! assert_parse_error(code);
//! // or
//! assert!(has_parse_error(code), "Expected error");
//! ```

use perl_parser::Parser;

/// Check if parsing the given code produces an error signal.
///
/// This function handles the IDE-friendly parser that recovers from errors by
/// returning `Ok(ast)` with ERROR nodes rather than `Err`.
///
/// Returns `true` if:
/// - Parser returns `Err` (catastrophic failure)
/// - Parser returns `Ok(ast)` where AST contains ERROR nodes (recovered failure)
///
/// # Example
///
/// ```rust,ignore
/// use support::parser_error_helpers::has_parse_error;
///
/// // Invalid Perl code should produce an error signal
/// assert!(has_parse_error("s/foo/bar/invalid_flag"));
///
/// // Valid Perl code should not produce an error signal
/// assert!(!has_parse_error("my $x = 1;"));
/// ```
pub fn has_parse_error(code: &str) -> bool {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Err(_) => true,
        Ok(ast) => {
            // Check for ERROR nodes in the AST via S-expression representation
            // This is the most reliable way to detect recovered parse errors
            ast.to_sexp().contains("ERROR")
        }
    }
}

/// Assert that parsing the given code produces an error signal.
///
/// This is the preferred way to test that invalid Perl code is detected
/// as invalid by the parser.
///
/// # Panics
///
/// Panics if the parser does NOT produce an error (neither `Err` return
/// nor ERROR nodes in the AST).
///
/// # Example
///
/// ```rust,ignore
/// use support::parser_error_helpers::assert_parse_error;
///
/// // These should all produce parse errors
/// assert_parse_error("s/foo/bar/z");     // Invalid modifier
/// assert_parse_error("s/pattern/");      // Missing replacement
/// assert_parse_error("sub ( { }");       // Malformed signature
/// ```
pub fn assert_parse_error(code: &str) {
    assert!(has_parse_error(code), "Expected error (Err or ERROR node) for: {}", code);
}

/// Assert that parsing the given code does NOT produce an error signal.
///
/// This is the preferred way to test that valid Perl code parses without errors.
///
/// # Panics
///
/// Panics if the parser produces an error (either `Err` return or ERROR
/// nodes in the AST).
///
/// # Example
///
/// ```rust,ignore
/// use support::parser_error_helpers::assert_parse_success;
///
/// // These should all parse without errors
/// assert_parse_success("my $x = 1;");
/// assert_parse_success("s/foo/bar/gi;");
/// assert_parse_success("sub foo { return 1; }");
/// ```
pub fn assert_parse_success(code: &str) {
    assert!(
        !has_parse_error(code),
        "Expected successful parse (no Err, no ERROR nodes) for: {}",
        code
    );
}

/// Check if parsing produces an error and return the result for further inspection.
///
/// This is useful when you need to both check for errors AND inspect the AST structure.
///
/// Returns `(has_error, Option<ast>)` where:
/// - `has_error` is true if Err or ERROR nodes present
/// - `ast` is Some if parse returned Ok, None if Err
///
/// # Example
///
/// ```rust,ignore
/// use support::parser_error_helpers::parse_with_error_check;
///
/// let (has_error, maybe_ast) = parse_with_error_check("s/foo/bar/z");
/// assert!(has_error);
/// // Can still inspect the recovered AST if needed
/// if let Some(ast) = maybe_ast {
///     // ERROR node is in the AST
///     assert!(ast.to_sexp().contains("ERROR"));
/// }
/// ```
pub fn parse_with_error_check(code: &str) -> (bool, Option<perl_parser::ast::Node>) {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Err(_) => (true, None),
        Ok(ast) => {
            let has_error = ast.to_sexp().contains("ERROR");
            (has_error, Some(ast))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_parse_error_detects_invalid_modifiers() {
        // Invalid substitution modifiers should be detected
        assert!(has_parse_error("s/foo/bar/z"));
        assert!(has_parse_error("s/foo/bar/invalid"));
    }

    #[test]
    fn test_has_parse_error_allows_valid_code() {
        // Valid code should not trigger error detection
        assert!(!has_parse_error("my $x = 1;"));
        assert!(!has_parse_error("s/foo/bar/gi;"));
        assert!(!has_parse_error("print 'hello';"));
    }

    #[test]
    fn test_assert_parse_error_works() {
        // Should not panic for invalid code
        assert_parse_error("s/foo/bar/z");
    }

    #[test]
    fn test_assert_parse_success_works() {
        // Should not panic for valid code
        assert_parse_success("my $x = 1;");
    }

    #[test]
    fn test_parse_with_error_check_returns_ast() {
        let (has_error, maybe_ast) = parse_with_error_check("s/foo/bar/z");
        assert!(has_error);
        // Parser should still return an AST (with ERROR nodes) due to recovery
        assert!(maybe_ast.is_some());
    }
}
