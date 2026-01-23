impl<'a> Parser<'a> {
    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.with_recursion_guard(|s| s.parse_comma())
    }
}
