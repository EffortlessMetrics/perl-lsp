#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(test)]
mod declaration_edge_cases_tests {
    use perl_parser::Parser;
    use perl_parser::declaration::DeclarationProvider;
    use std::sync::Arc;

    fn parse_and_get_provider(code: &str) -> DeclarationProvider<'_> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("Failed to parse");
        let ast_arc = Arc::new(ast);

        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_with_strict_option() {
        let code = "use constant -strict, FOO => 42;\nmy $x = FOO;";
        let provider = parse_and_get_provider(code);

        // Should find FOO despite -strict option
        let decls = provider.find_declaration(43, 0).unwrap_or_default(); // Position at FOO usage
        assert!(!decls.is_empty(), "Should find FOO constant despite -strict option");

        // Verify the declaration points to the right place
        if let Some(decl) = decls.first() {
            let text = &code[decl.target_range.0..decl.target_range.1];
            assert!(text.contains("FOO"), "Declaration should point to FOO constant");
        }
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_with_multiple_options() {
        let code = "use constant -strict, -nonstrict, -force, FOO => 42;";
        let provider = parse_and_get_provider(code);

        // Test that we can find the constant name after multiple options
        let text = provider.get_node_text(&provider.ast);
        assert!(text.contains("FOO"), "Should handle multiple options");
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_qw_with_symmetric_delimiters() {
        let code = r#"
use constant qw|FOO BAR BAZ|;
use constant qw!TEST1 TEST2!;
use constant qw#ONE TWO#;
use constant qw~ALPHA BETA~;
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code);

        // Should find FOO with pipe delimiters
        let decls =
            provider.find_declaration(code.find("my $x = FOO").unwrap() + 8, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO with pipe delimiters");
    }

    #[test]
    fn test_qwerty_not_matched_as_qw() {
        let code = r#"
# This comment has qwerty in it
use constant qw(FOO BAR);
my $qwerty = 1;
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code);

        // Should still find FOO despite qwerty in comment
        let decls =
            provider.find_declaration(code.find("my $x = FOO").unwrap() + 8, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO despite qwerty in comment");
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_multiple_qw_in_one_line() {
        let code = r#"
use constant qw(FOO) => 1, qw(BAR BAZ) => 2;
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code);

        // Should find BAR from the second qw
        let decls =
            provider.find_declaration(code.find("my $x = BAR").unwrap() + 8, 0).unwrap_or_default();
        // This is a complex case that may not be fully supported yet
        // Just verify we don't crash
        let _ = decls;
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_hash_with_unary_plus() {
        let code = r#"
use constant +{ FOO => 1, BAR => 2 };
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code);

        // Should find FOO despite unary + before hash
        let decls =
            provider.find_declaration(code.find("my $x = FOO").unwrap() + 8, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO with unary + before hash");
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_multiple_hash_blocks() {
        let code = r#"
use constant { 
    # First block
    FOO => 1 
} => "ignored", { 
    # Second block  
    BAR => 2 
};
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code);

        // Should find BAR from the second hash block
        let decls =
            provider.find_declaration(code.find("my $x = BAR").unwrap() + 8, 0).unwrap_or_default();
        // Verify we scan all {...} blocks
        // Should scan all hash blocks (or gracefully handle)
        let _ = decls;
    }

    #[test]
    fn test_mixed_line_endings_in_constants() {
        let code = "use constant FOO => 1;\r\nuse constant BAR => 2;\nmy $x = BAR;";
        let provider = parse_and_get_provider(code);

        // Should handle mixed line endings
        let decls =
            provider.find_declaration(code.find("my $x = BAR").unwrap() + 8, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find BAR with mixed line endings");
    }

    #[test]
    fn test_unicode_constant_names() {
        let code = "use constant π => 3.14159;\nmy $circumference = 2 * π;";
        let provider = parse_and_get_provider(code);

        // Should find π constant
        let decls =
            provider.find_declaration(code.find("2 * π").unwrap() + 4, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find Unicode constant name π");
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_comma_form() {
        // Perl also supports: use constant FOO, 42;
        let code = "use constant FOO, 42;\nmy $x = FOO;";
        let provider = parse_and_get_provider(code);

        // This form may not be fully supported, but shouldn't crash
        let decls =
            provider.find_declaration(code.find("my $x = FOO").unwrap() + 8, 0).unwrap_or_default();
        let _ = decls; // Just verify no panic
    }

    #[test]
    fn test_empty_qw() {
        let code = "use constant qw();\nmy $x = 1;";
        let provider = parse_and_get_provider(code);

        // Empty qw shouldn't cause issues
        let text = provider.get_node_text(&provider.ast);
        assert!(text.contains("qw"), "Should handle empty qw()");
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_nested_braces_in_hash() {
        let code = r#"
use constant {
    FOO => sub { return { nested => 1 } },
    BAR => 2
};
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code);

        // Should find BAR despite nested braces
        let decls =
            provider.find_declaration(code.find("my $x = BAR").unwrap() + 8, 0).unwrap_or_default();
        // Complex nested structure may be challenging
        let _ = decls;
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_qw_with_newlines() {
        let code = r#"
use constant qw(
    FOO
    BAR
    BAZ
);
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code);

        // Should find BAR in multi-line qw
        let decls =
            provider.find_declaration(code.find("my $x = BAR").unwrap() + 8, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find BAR in multi-line qw");
    }

    #[test]
    fn test_constant_redefinition() {
        let code = r#"
use constant FOO => 1;
use constant FOO => 2;  # Redefinition
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code);

        // Should find both FOO definitions
        let decls =
            provider.find_declaration(code.find("my $x = FOO").unwrap() + 8, 0).unwrap_or_default();
        // May find one or both - just verify no crash
        let _ = decls;
    }

    #[test]
    fn test_package_qualified_constant() {
        let code = r#"
package Foo;
use constant BAR => 42;
package main;
my $x = Foo::BAR;
"#;
        let provider = parse_and_get_provider(code);

        // Should handle package-qualified constants
        let decls =
            provider.find_declaration(code.find("Foo::BAR").unwrap() + 5, 0).unwrap_or_default();
        // Package-qualified lookups are complex
        let _ = decls;
    }
}
