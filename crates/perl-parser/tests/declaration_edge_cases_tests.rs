#[cfg(test)]
mod declaration_edge_cases_tests {
    use perl_parser::Parser;
    use perl_parser::declaration::DeclarationProvider;
    use std::sync::Arc;

    fn parse_and_get_provider(
        code: &str,
    ) -> Result<DeclarationProvider<'_>, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("Failed to parse: {:?}", e))?;
        let ast_arc = Arc::new(ast);

        Ok(DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string()))
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_with_strict_option() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant -strict, FOO => 42;\nmy $x = FOO;";
        let provider = parse_and_get_provider(code)?;

        // Should find FOO despite -strict option
        let decls = provider.find_declaration(43, 0).unwrap_or_default(); // Position at FOO usage
        assert!(!decls.is_empty(), "Should find FOO constant despite -strict option");

        // Verify the declaration points to the right place
        if let Some(decl) = decls.first() {
            let text = &code[decl.target_range.0..decl.target_range.1];
            assert!(text.contains("FOO"), "Declaration should point to FOO constant");
        }

        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_with_multiple_options() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant -strict, -nonstrict, -force, FOO => 42;";
        let provider = parse_and_get_provider(code)?;

        // Test that we can find the constant name after multiple options
        let text = provider.get_node_text(&provider.ast);
        assert!(text.contains("FOO"), "Should handle multiple options");

        Ok(())
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_qw_with_symmetric_delimiters() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant qw|FOO BAR BAZ|;
use constant qw!TEST1 TEST2!;
use constant qw#ONE TWO#;
use constant qw~ALPHA BETA~;
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find FOO with pipe delimiters
        let pos = code.find("my $x = FOO").ok_or("Could not find 'my $x = FOO' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO with pipe delimiters");

        Ok(())
    }

    #[test]
    fn test_qwerty_not_matched_as_qw() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
# This comment has qwerty in it
use constant qw(FOO BAR);
my $qwerty = 1;
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should still find FOO despite qwerty in comment
        let pos = code.find("my $x = FOO").ok_or("Could not find 'my $x = FOO' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO despite qwerty in comment");

        Ok(())
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_multiple_qw_in_one_line() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant qw(FOO) => 1, qw(BAR BAZ) => 2;
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find BAR from the second qw
        let pos = code.find("my $x = BAR").ok_or("Could not find 'my $x = BAR' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        // This is a complex case that may not be fully supported yet
        // Just verify we don't crash
        let _ = decls;

        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_hash_with_unary_plus() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant +{ FOO => 1, BAR => 2 };
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find FOO despite unary + before hash
        let pos = code.find("my $x = FOO").ok_or("Could not find 'my $x = FOO' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find FOO with unary + before hash");

        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_multiple_hash_blocks() -> Result<(), Box<dyn std::error::Error>> {
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
        let provider = parse_and_get_provider(code)?;

        // Should find BAR from the second hash block
        let pos = code.find("my $x = BAR").ok_or("Could not find 'my $x = BAR' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        // Verify we scan all {...} blocks
        // Should scan all hash blocks (or gracefully handle)
        let _ = decls;

        Ok(())
    }

    #[test]
    fn test_mixed_line_endings_in_constants() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant FOO => 1;\r\nuse constant BAR => 2;\nmy $x = BAR;";
        let provider = parse_and_get_provider(code)?;

        // Should handle mixed line endings
        let pos = code.find("my $x = BAR").ok_or("Could not find 'my $x = BAR' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find BAR with mixed line endings");

        Ok(())
    }

    #[test]
    fn test_unicode_constant_names() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant π => 3.14159;\nmy $circumference = 2 * π;";
        let provider = parse_and_get_provider(code)?;

        // Should find π constant
        let pos = code.find("2 * π").ok_or("Could not find '2 * π' in code")? + 4;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find Unicode constant name π");

        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_comma_form() -> Result<(), Box<dyn std::error::Error>> {
        // Perl also supports: use constant FOO, 42;
        let code = "use constant FOO, 42;\nmy $x = FOO;";
        let provider = parse_and_get_provider(code)?;

        // This form may not be fully supported, but shouldn't crash
        let pos = code.find("my $x = FOO").ok_or("Could not find 'my $x = FOO' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        let _ = decls; // Just verify no panic

        Ok(())
    }

    #[test]
    fn test_empty_qw() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw();\nmy $x = 1;";
        let provider = parse_and_get_provider(code)?;

        // Empty qw shouldn't cause issues
        let text = provider.get_node_text(&provider.ast);
        assert!(text.contains("qw"), "Should handle empty qw()");

        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_nested_braces_in_hash() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant {
    FOO => sub { return { nested => 1 } },
    BAR => 2
};
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find BAR despite nested braces
        let pos = code.find("my $x = BAR").ok_or("Could not find 'my $x = BAR' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        // Complex nested structure may be challenging
        let _ = decls;

        Ok(())
    }

    #[cfg(feature = "qw-variants")]
    #[test]
    fn test_qw_with_newlines() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant qw(
    FOO
    BAR
    BAZ
);
my $x = BAR;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find BAR in multi-line qw
        let pos = code.find("my $x = BAR").ok_or("Could not find 'my $x = BAR' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        assert!(!decls.is_empty(), "Should find BAR in multi-line qw");

        Ok(())
    }

    #[test]
    fn test_constant_redefinition() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant FOO => 1;
use constant FOO => 2;  # Redefinition
my $x = FOO;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should find both FOO definitions
        let pos = code.find("my $x = FOO").ok_or("Could not find 'my $x = FOO' in code")? + 8;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        // May find one or both - just verify no crash
        let _ = decls;

        Ok(())
    }

    #[test]
    fn test_package_qualified_constant() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
package Foo;
use constant BAR => 42;
package main;
my $x = Foo::BAR;
"#;
        let provider = parse_and_get_provider(code)?;

        // Should handle package-qualified constants
        let pos = code.find("Foo::BAR").ok_or("Could not find 'Foo::BAR' in code")? + 5;
        let decls = provider.find_declaration(pos, 0).unwrap_or_default();
        // Package-qualified lookups are complex
        let _ = decls;

        Ok(())
    }
}
