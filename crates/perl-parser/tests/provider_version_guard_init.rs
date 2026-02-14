#[cfg(test)]
mod provider_version_guard_init {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use std::sync::Arc;

    #[test]
    #[should_panic(expected = "used after AST refresh")]
    #[cfg(debug_assertions)]
    fn provider_panics_without_doc_version() {
        let code = "my $x = 42;";
        let mut parser = Parser::new(code);
        use perl_tdd_support::must;
        let ast = Arc::new(must(parser.parse()));

        // Create provider WITHOUT calling with_doc_version
        let provider =
            DeclarationProvider::new(ast, code.to_string(), "file:///test.pl".to_string());

        // This should panic because doc_version is still i32::MIN
        provider.find_declaration(3, 1);
    }

    #[test]
    fn provider_works_with_doc_version() {
        let code = "my $x = 42;";
        let mut parser = Parser::new(code);
        use perl_tdd_support::must;
        let ast = Arc::new(must(parser.parse()));

        // Create provider WITH with_doc_version
        let provider =
            DeclarationProvider::new(ast, code.to_string(), "file:///test.pl".to_string())
                .with_doc_version(1);

        // This should work fine (doesn't panic)
        let _result = provider.find_declaration(3, 1);
        // Don't care about the result, just that it doesn't panic
    }
}
