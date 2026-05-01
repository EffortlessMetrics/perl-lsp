#[cfg(test)]
mod tests {
    use crate::{PerlLexer, TokenType};

    /// Test that format body parsing doesn't hang on unterminated input.
    ///
    /// This test verifies that the lexer returns an error token when given
    /// format content without a proper terminator (single `.` on a line).
    /// The test will hang if there's an infinite loop, which will be caught
    /// by the test framework's timeout or CI job timeout.
    #[test]
    fn test_format_body_infinite_loop() {
        let input = "format body without terminator\nthis goes on forever\n";
        let mut lexer = PerlLexer::new(input);

        // Manually enter format mode to test
        lexer.enter_format_mode();

        // This should complete quickly and return an error token
        // (not hang in an infinite loop)
        let token = lexer.next_token();

        // Should get an error token for unterminated format
        assert!(matches!(token, Some(t) if matches!(t.token_type, TokenType::Error(_))));
    }
}
