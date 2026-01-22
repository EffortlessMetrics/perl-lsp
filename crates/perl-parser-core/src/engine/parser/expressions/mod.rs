impl<'a> Parser<'a> {
    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult<Node> {
        self.check_recursion()?;

        // RAII guard to ensure exit_recursion is called even on error
        struct RecursionGuard<'a, 'b>(&'a mut Parser<'b>);
        impl<'a, 'b> Drop for RecursionGuard<'a, 'b> {
            fn drop(&mut self) {
                self.0.exit_recursion();
            }
        }
        let _guard = RecursionGuard(self);

        _guard.0.parse_comma()
    }

}
