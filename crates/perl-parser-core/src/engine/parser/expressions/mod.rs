impl<'a> Parser<'a> {
    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;
        let result = self.parse_comma();
        self.exit_recursion();
        result
    }

}
