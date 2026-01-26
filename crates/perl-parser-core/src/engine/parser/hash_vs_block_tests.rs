#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_ambiguous_brace_context() {
        // Hash reference: { key => 'value' }
        let code_hash = "my $ref = { key => 'value' };";
        let mut parser = Parser::new(code_hash);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to parse hash reference");
        let ast = must(result);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("(hash"), "Should parse as hash: {}", sexp);

        // Code block: { print "hello"; }
        let code_block = "my $code = { print 'hello'; };";
        let mut parser2 = Parser::new(code_block);
        let result2 = parser2.parse();
        assert!(result2.is_ok(), "Failed to parse code block");
        let ast2 = result2.unwrap();
        let sexp2 = ast2.to_sexp();
        assert!(sexp2.contains("(block"), "Should parse as block: {}", sexp2);
    }

    #[test]
    fn test_nested_ambiguity() {
        let code = r#"
sub my_sub {
    { key => 'value' }
}
"#;
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = must(result);
        let sexp = ast.to_sexp();

        // In statement context, { ... } is a block.
        // If it contains key => value, it's a block with a hash inside or expression statement?
        // Actually, { key => value } in statement context is a block containing a statement.
        // The statement is `key => 'value'`, which is `key, 'value'`.
        // Wait, `=>` is fat comma. So it's `key, 'value'`.
        // This is a valid statement (expression statement with comma operator).
        // However, `+` is often used to disambiguate: `+{ key => value }` forces hash ref.
        // Without `+` or assignment, it's a block.

        assert!(sexp.contains("(block"), "Should parse as block in statement context: {}", sexp);
    }

    #[test]
    fn test_map_grep_sort_blocks() {
        // map { ... } @list - always a block
        let code = "map { $_ * 2 } @list;";
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok());
        let ast = must(result);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("(block"), "map should take a block: {}", sexp);

        // map { key => value } @list - block returning list
        let code2 = "map { key => 'value' } @list;";
        let mut parser2 = Parser::new(code2);
        let result2 = parser2.parse();
        assert!(result2.is_ok());
        let ast2 = result2.unwrap();
        let sexp2 = ast2.to_sexp();
        assert!(
            sexp2.contains("(block"),
            "map should take a block even with hash-like content: {}",
            sexp2
        );
    }
}
