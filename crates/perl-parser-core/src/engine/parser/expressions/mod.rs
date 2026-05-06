impl<'a> Parser<'a> {
    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| s.parse_comma())
    }

    /// Validate parser-owned regex bodies while preserving Perl syntax that is
    /// risky but valid. Embedded code is represented on the AST, and nested
    /// quantifiers stay as non-fatal parser diagnostics.
    fn analyze_regex_body_for_ast(&mut self, pattern: &str, start: usize) -> ParseResult<bool> {
        let validator = crate::engine::regex_validator::RegexValidator::new();
        let has_embedded_code = validator.find_code_execution(pattern, start).is_some();
        let nested_quantifier = validator.find_nested_quantifier(pattern, start);

        if !has_embedded_code && nested_quantifier.is_none() {
            validator.validate(pattern, start).map_err(|error| match error {
                crate::engine::regex_validator::RegexError::Syntax { message, offset } => {
                    ParseError::syntax(message, offset)
                }
            })?;
        }

        if let Some(finding) = nested_quantifier {
            self.record_error(ParseError::syntax(
                "Nested quantifiers detected (possible backtracking risk)",
                finding.offset,
            ));
        }

        Ok(has_embedded_code)
    }
}
