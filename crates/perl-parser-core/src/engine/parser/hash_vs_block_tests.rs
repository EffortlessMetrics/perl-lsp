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
        let ast2 = must(result2);
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
        let ast2 = must(result2);
        let sexp2 = ast2.to_sexp();
        assert!(
            sexp2.contains("(block"),
            "map should take a block even with hash-like content: {}",
            sexp2
        );

        // map/grep/sort block with a statement list followed by list args
        let code3 = "map { my $x = $_; $x * 2 } @list;";
        let mut parser3 = Parser::new(code3);
        let result3 = parser3.parse();
        assert!(result3.is_ok(), "map block+list should parse: {:?}", result3.err());

        let code4 = "grep { my $x = $_; $x > 0 } @list;";
        let mut parser4 = Parser::new(code4);
        let result4 = parser4.parse();
        assert!(result4.is_ok(), "grep block+list should parse: {:?}", result4.err());

        let code5 = "sort { my $x = $a <=> $b; $x } @list;";
        let mut parser5 = Parser::new(code5);
        let result5 = parser5.parse();
        assert!(result5.is_ok(), "sort block+list should parse: {:?}", result5.err());

        let code6 = "my @m = map { my $x = $_; $x * 2 } @list;";
        let mut parser6 = Parser::new(code6);
        let result6 = parser6.parse();
        assert!(result6.is_ok(), "map expression block+list should parse: {:?}", result6.err());

        let code7 = "my @g = grep { my $x = $_; $x > 0 } @list;";
        let mut parser7 = Parser::new(code7);
        let result7 = parser7.parse();
        assert!(result7.is_ok(), "grep expression block+list should parse: {:?}", result7.err());

        let code8 = "my @s = sort { my $x = $a <=> $b; $x } @list;";
        let mut parser8 = Parser::new(code8);
        let result8 = parser8.parse();
        assert!(result8.is_ok(), "sort expression block+list should parse: {:?}", result8.err());
    }
}
