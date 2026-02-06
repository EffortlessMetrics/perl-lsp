//! Micro tests for declaration provider functionality.
//!
//! Feature-gated test organization:
//! - Core tests run by default
//! - `constant-advanced` feature enables advanced constant pragma tests
//! - `qw-variants` feature enables non-standard qw delimiter tests
//! - `parser-extras` feature enables unicode/emoji edge cases
//!
//! Run specific feature tests with:
//!   cargo test -p perl-parser --features constant-advanced
//!   cargo test -p perl-parser --features qw-variants
//!   cargo test -p perl-parser --features parser-extras

#[cfg(test)]
mod declaration_micro_tests {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    type ParentMap = FxHashMap<*const perl_parser::ast::Node, *const perl_parser::ast::Node>;

    fn parse_and_get_provider(
        code: &str,
    ) -> Result<
        (DeclarationProvider<'static>, ParentMap, Arc<perl_parser::ast::Node>),
        Box<dyn std::error::Error>,
    > {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let ast_arc = Arc::new(ast);

        // Build parent map
        let mut parent_map = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

        // Create provider - we need to leak to satisfy lifetime
        let leaked_map = Box::leak(Box::new(parent_map));
        let provider = unsafe {
            DeclarationProvider::new(ast_arc.clone(), code.to_string(), "test.pl".to_string())
                .with_parent_map(leaked_map)
                .with_doc_version(0)
        };

        Ok((provider, leaked_map.clone(), ast_arc))
    }

    // =========================================================================
    // Core tests (run by default)
    // =========================================================================

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_multiple_qw_on_same_line() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw(FOO); use constant qw(BAR); print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // BAR at print position
        let links = provider.find_declaration(51, 0);
        assert!(links.is_some(), "Should find BAR from second qw");
        let links = links.ok_or("Expected links for BAR")?;
        assert!(!links.is_empty(), "Links should not be empty");
        // Should point to the second qw, not the first
        assert!(links[0].target_selection_range.0 > 21, "Should point to second use constant");
        Ok(())
    }

    #[test]
    fn test_empty_qw() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw(); my $x = 1; print $x;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // $x at print position - should still work with empty qw
        let links = provider.find_declaration(37, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find $x despite empty qw()"
        );
        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_multiline_qw() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw(\n    FOO\n    BAR\n); print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // FOO at print position
        let links = provider.find_declaration(42, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find FOO in multi-line qw"
        );
        Ok(())
    }

    #[cfg(feature = "constant-advanced")]
    #[test]
    fn test_constant_single_arrow_form() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant FOO => 'value'; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // FOO at print position
        let links = provider.find_declaration(35, 0);
        assert!(links.is_some(), "Should find simple arrow constant");
        let links = links.ok_or("Expected links for FOO")?;
        assert!(!links.is_empty(), "Links should not be empty");
        assert_eq!(links[0].target_selection_range.0, 13); // Position of FOO in declaration
        Ok(())
    }

    #[test]
    fn crlf_with_trailing_emoji_clamp_and_roundtrip() {
        use perl_parser::position::LineStartsCache;

        // "A🐍\r\n" — UTF-16 columns: 'A'->1, '🐍'->+2, total line len = 3 before CR
        let text = "A🐍\r\n";
        let cache = LineStartsCache::new(text);

        // place caret far past EOL on line 0, ensure clamp then round-trip
        let off = cache.position_to_offset(text, 0, 999);
        let (line, col) = cache.offset_to_position(text, off);
        assert_eq!((line, col), (0, 3), "Should clamp to end of line which is column 3 in UTF-16");
    }

    // =========================================================================
    // Word boundary and comment handling tests
    // (Previously BUG category - fixed with dynamic position computation)
    // =========================================================================

    #[test]
    fn test_word_boundary_qwerty_not_matched() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $qwerty = 'test'; print $qwerty;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // Look up $qwerty at print position - parser should NOT confuse "qwerty" with "qw" operator
        let ref_pos = code.rfind("$qwerty").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(links.is_some(), "Should find qwerty variable");
        let links = links.ok_or("Expected links for qwerty")?;
        assert!(!links.is_empty(), "Links should not be empty");
        // The declaration span includes the sigil: "$qwerty" starts at position 3
        let decl_pos = code.find("$qwerty").ok_or("declaration position not found")?;
        assert_eq!(links[0].target_selection_range.0, decl_pos);
        Ok(())
    }

    #[test]
    fn test_comment_with_qw_in_it() -> Result<(), Box<dyn std::error::Error>> {
        let code = "# qw is used here\nmy $var = 1; print $var;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // Dynamically find the reference position (second $var, in print statement)
        let ref_pos = code.rfind("$var").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find $var despite qw in comment"
        );
        // Verify it points to the declaration (first $var)
        let decl_pos = code.find("$var").ok_or("declaration position not found")?;
        assert_eq!(links.as_ref().ok_or("Expected links")?[0].target_selection_range.0, decl_pos);
        Ok(())
    }
}

// =============================================================================
// constant-advanced feature: Advanced constant pragma parsing
// =============================================================================
// Run with: cargo test -p perl-parser --features constant-advanced
// =============================================================================
#[cfg(all(test, feature = "constant-advanced"))]
mod constant_advanced_tests {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    type ParentMap = FxHashMap<*const perl_parser::ast::Node, *const perl_parser::ast::Node>;

    fn parse_and_get_provider(
        code: &str,
    ) -> Result<
        (DeclarationProvider<'static>, ParentMap, Arc<perl_parser::ast::Node>),
        Box<dyn std::error::Error>,
    > {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let ast_arc = Arc::new(ast);

        let mut parent_map = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

        let leaked_map = Box::leak(Box::new(parent_map));
        let provider = unsafe {
            DeclarationProvider::new(ast_arc.clone(), code.to_string(), "test.pl".to_string())
                .with_parent_map(leaked_map)
                .with_doc_version(0)
        };

        Ok((provider, leaked_map.clone(), ast_arc))
    }

    #[test]
    fn test_constant_with_strict_option() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant -strict, FOO => 42; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // FOO at position 38-41
        let ref_pos = code.rfind("FOO").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(links.is_some(), "Should find declaration for FOO");
        let links = links.ok_or("Expected links for FOO")?;
        assert!(!links.is_empty(), "Links should not be empty");
        assert_eq!(links[0].target_selection_range.0, 22); // Start of FOO in declaration
        Ok(())
    }

    #[test]
    fn test_constant_with_comma_after_options() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant -nonstrict, -force, BAR => 'test'; print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // BAR at print position
        let ref_pos = code.rfind("BAR").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find declaration for BAR with options"
        );
        Ok(())
    }

    #[test]
    fn test_constant_with_unary_plus_hash() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant +{ FOO => 1, BAR => 2 }; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // FOO at print position
        let ref_pos = code.rfind("FOO").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find FOO in +{{...}}"
        );
        Ok(())
    }

    #[test]
    fn test_nested_braces_in_constant() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant { FOO => { nested => 1 }, BAR => 2 }; print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // BAR at print position
        let ref_pos = code.rfind("BAR").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find BAR despite nested braces"
        );
        Ok(())
    }

    #[test]
    fn test_multiple_hash_blocks() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant { A => 1 }, { B => 2 }; print B;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // B at print position - should find it in second hash
        let ref_pos = code.rfind("B").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find B in second hash block"
        );
        Ok(())
    }

    #[test]
    fn constant_options_qw_both_names_exact_spans() -> Result<(), Box<dyn std::error::Error>> {
        // Corrected syntax: print FOO; print BAR;
        let code = "use constant -strict, qw|FOO BAR|;\nprint FOO; print BAR;\n";
        let (provider, _pm, _ast) = parse_and_get_provider(code)?;

        // offset for FOO in `print FOO;`
        let foo_off = code.rfind("FOO").ok_or("FOO not found")?;
        let foo_links = provider.find_declaration(foo_off, 0);
        assert!(foo_links.is_some(), "Should find FOO");
        let foo_links = foo_links.ok_or("Expected links for FOO")?;
        assert!(!foo_links.is_empty(), "Should have at least one link for FOO");
        let foo_link = &foo_links[0].target_selection_range;
        assert_eq!(&code[foo_link.0..foo_link.1], "FOO", "FOO span should be exact");

        // offset for BAR
        let bar_off = code.rfind("BAR").ok_or("BAR not found")?;
        let bar_links = provider.find_declaration(bar_off, 0);
        assert!(bar_links.is_some(), "Should find BAR");
        let bar_links = bar_links.ok_or("Expected links for BAR")?;
        assert!(!bar_links.is_empty(), "Should have at least one link for BAR");
        let bar_link = &bar_links[0].target_selection_range;
        assert_eq!(&code[bar_link.0..bar_link.1], "BAR", "BAR span should be exact");
        Ok(())
    }
}

// =============================================================================
// qw-variants feature: Non-standard qw delimiter tests
// =============================================================================
// Run with: cargo test -p perl-parser --features qw-variants
// =============================================================================
#[cfg(all(test, feature = "qw-variants"))]
mod qw_variants_tests {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    type ParentMap = FxHashMap<*const perl_parser::ast::Node, *const perl_parser::ast::Node>;

    fn parse_and_get_provider(
        code: &str,
    ) -> Result<
        (DeclarationProvider<'static>, ParentMap, Arc<perl_parser::ast::Node>),
        Box<dyn std::error::Error>,
    > {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let ast_arc = Arc::new(ast);

        let mut parent_map = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

        let leaked_map = Box::leak(Box::new(parent_map));
        let provider = unsafe {
            DeclarationProvider::new(ast_arc.clone(), code.to_string(), "test.pl".to_string())
                .with_parent_map(leaked_map)
                .with_doc_version(0)
        };

        Ok((provider, leaked_map.clone(), ast_arc))
    }

    #[test]
    fn test_symmetric_qw_delimiters() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw|FOO BAR|; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // FOO at print position
        let ref_pos = code.rfind("FOO").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find FOO in qw|...|"
        );
        Ok(())
    }

    #[test]
    fn test_qw_exclamation_delimiters() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant qw!BAZ QUX!; print QUX;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // QUX at print position
        let ref_pos = code.rfind("QUX").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find QUX in qw!...!"
        );
        Ok(())
    }
}

// =============================================================================
// parser-extras feature: Unicode/emoji edge cases
// =============================================================================
// Run with: cargo test -p perl-parser --features parser-extras
// =============================================================================
#[cfg(all(test, feature = "parser-extras"))]
mod parser_extras_tests {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    type ParentMap = FxHashMap<*const perl_parser::ast::Node, *const perl_parser::ast::Node>;

    fn parse_and_get_provider(
        code: &str,
    ) -> Result<
        (DeclarationProvider<'static>, ParentMap, Arc<perl_parser::ast::Node>),
        Box<dyn std::error::Error>,
    > {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let ast_arc = Arc::new(ast);

        let mut parent_map = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

        let leaked_map = Box::leak(Box::new(parent_map));
        let provider = unsafe {
            DeclarationProvider::new(ast_arc.clone(), code.to_string(), "test.pl".to_string())
                .with_parent_map(leaked_map)
                .with_doc_version(0)
        };

        Ok((provider, leaked_map.clone(), ast_arc))
    }

    #[test]
    fn test_unicode_constant_name() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant π => 3.14159; print π;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // π at print position
        let ref_pos = code.rfind("π").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find Unicode constant π"
        );
        Ok(())
    }

    #[test]
    fn test_mixed_line_endings_with_emoji() -> Result<(), Box<dyn std::error::Error>> {
        // Test with CRLF and emoji
        let code = "my $🐍 = 'python';\r\nprint $🐍;";
        let (provider, _map, _ast) = parse_and_get_provider(code)?;

        // $🐍 at print position
        let ref_pos = code.rfind("$🐍").ok_or("reference position not found")?;
        let links = provider.find_declaration(ref_pos, 0);
        assert!(
            links.is_some() && !links.as_ref().ok_or("Expected links")?.is_empty(),
            "Should find emoji variable with CRLF"
        );
        Ok(())
    }
}
