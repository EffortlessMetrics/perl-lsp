#[cfg(test)]
mod declaration_micro_tests {
    use perl_parser::{Parser, declaration::DeclarationProvider};
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    type ParentMap = FxHashMap<*const perl_parser::ast::Node, *const perl_parser::ast::Node>;

    fn parse_and_get_provider(
        code: &str,
    ) -> (DeclarationProvider<'static>, ParentMap, Arc<perl_parser::ast::Node>) {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("Failed to parse");
        let ast_arc = Arc::new(ast);

        // Build parent map
        let mut parent_map = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

        // Create provider - we need to leak to satisfy lifetime
        let leaked_map = Box::leak(Box::new(parent_map));
        let provider =
            DeclarationProvider::new(ast_arc.clone(), code.to_string(), "test.pl".to_string())
                .with_parent_map(leaked_map)
                .with_doc_version(0);

        (provider, leaked_map.clone(), ast_arc)
    }

    #[test]
    #[ignore = "Parser doesn't support constant pragma with -strict"]
    fn test_constant_with_strict_option() {
        let code = "use constant -strict, FOO => 42; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // FOO at position 38-41
        let links = provider.find_declaration(38, 0);
        assert!(links.is_some(), "Should find declaration for FOO");
        let links = links.unwrap();
        assert!(!links.is_empty(), "Links should not be empty");
        assert_eq!(links[0].target_selection_range.0, 22); // Start of FOO in declaration
    }

    #[test]
    #[ignore = "Parser doesn't support constant pragma with multiple options"]
    fn test_constant_with_comma_after_options() {
        let code = "use constant -nonstrict, -force, BAR => 'test'; print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // BAR at print position
        let links = provider.find_declaration(55, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find declaration for BAR with options"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support all qw delimiter forms"]
    fn test_symmetric_qw_delimiters() {
        let code = "use constant qw|FOO BAR|; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // FOO at print position
        let links = provider.find_declaration(32, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find FOO in qw|...|"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support qw with exclamation delimiters"]
    fn test_qw_exclamation_delimiters() {
        let code = "use constant qw!BAZ QUX!; print QUX;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // QUX at print position
        let links = provider.find_declaration(32, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find QUX in qw!...!"
        );
    }

    #[test]
    #[ignore = "Parser qw matching issue"]
    fn test_word_boundary_qwerty_not_matched() {
        let code = "my $qwerty = 'test'; print $qwerty;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // qwerty at print position - should find the variable, not think it's qw
        let links = provider.find_declaration(27, 0);
        assert!(links.is_some(), "Should find qwerty variable");
        let links = links.unwrap();
        assert!(!links.is_empty(), "Links should not be empty");
        // The declaration should be at position 4 (after "my $")
        assert_eq!(links[0].target_selection_range.0, 4);
    }

    #[test]
    fn test_multiple_qw_on_same_line() {
        let code = "use constant qw(FOO); use constant qw(BAR); print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // BAR at print position
        let links = provider.find_declaration(51, 0);
        assert!(links.is_some(), "Should find BAR from second qw");
        let links = links.unwrap();
        assert!(!links.is_empty(), "Links should not be empty");
        // Should point to the second qw, not the first
        assert!(links[0].target_selection_range.0 > 21, "Should point to second use constant");
    }

    #[test]
    #[ignore = "BUG: Parser span calculation differs - test expects hardcoded byte offset"]
    fn test_comment_with_qw_in_it() {
        let code = "# qw is used here\nmy $var = 1; print $var;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // $var at print position
        let links = provider.find_declaration(36, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find $var despite qw in comment"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support unary plus in constant hash"]
    fn test_constant_with_unary_plus_hash() {
        let code = "use constant +{ FOO => 1, BAR => 2 }; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // FOO at print position
        let links = provider.find_declaration(45, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find FOO in +{{...}}"
        );
    }

    #[test]
    fn test_empty_qw() {
        let code = "use constant qw(); my $x = 1; print $x;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // $x at print position - should still work with empty qw
        let links = provider.find_declaration(37, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find $x despite empty qw()"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support nested braces in constant"]
    fn test_nested_braces_in_constant() {
        let code = "use constant { FOO => { nested => 1 }, BAR => 2 }; print BAR;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // BAR at print position
        let links = provider.find_declaration(58, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find BAR despite nested braces"
        );
    }

    #[test]
    fn test_multiline_qw() {
        let code = "use constant qw(\n    FOO\n    BAR\n); print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // FOO at print position
        let links = provider.find_declaration(42, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find FOO in multi-line qw"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support unicode constant names"]
    fn test_unicode_constant_name() {
        let code = "use constant π => 3.14159; print π;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // π at print position
        let links = provider.find_declaration(33, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find Unicode constant π"
        );
    }

    #[test]
    #[ignore = "Parser doesn't handle mixed line endings with emoji"]
    fn test_mixed_line_endings_with_emoji() {
        // Test with CRLF and emoji
        let code = "my $🐍 = 'python';\r\nprint $🐍;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // $🐍 at print position
        let links = provider.find_declaration(27, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find emoji variable with CRLF"
        );
    }

    #[test]
    fn test_constant_single_arrow_form() {
        let code = "use constant FOO => 'value'; print FOO;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // FOO at print position
        let links = provider.find_declaration(35, 0);
        assert!(links.is_some(), "Should find simple arrow constant");
        let links = links.unwrap();
        assert!(!links.is_empty(), "Links should not be empty");
        assert_eq!(links[0].target_selection_range.0, 13); // Position of FOO in declaration
    }

    #[test]
    #[ignore = "Parser doesn't support multiple hash blocks"]
    fn test_multiple_hash_blocks() {
        let code = "use constant { A => 1 }, { B => 2 }; print B;";
        let (provider, _map, _ast) = parse_and_get_provider(code);

        // B at print position - should find it in second hash
        let links = provider.find_declaration(43, 0);
        assert!(
            links.is_some() && !links.as_ref().unwrap().is_empty(),
            "Should find B in second hash block"
        );
    }

    #[test]
    #[ignore = "Parser doesn't support constant pragma with options and qw"]
    fn constant_options_qw_both_names_exact_spans() {
        let code = "use constant -strict, qw|FOO BAR|;\nprint FOO, BAR;\n";
        let (provider, _pm, _ast) = parse_and_get_provider(code);

        // offset for FOO in `print FOO, BAR;`
        let foo_off = code.find("FOO,").unwrap();
        let foo_links = provider.find_declaration(foo_off, 0);
        assert!(foo_links.is_some(), "Should find FOO");
        let foo_links = foo_links.unwrap();
        assert!(!foo_links.is_empty(), "Should have at least one link for FOO");
        let foo_link = &foo_links[0].target_selection_range;
        assert_eq!(&code[foo_link.0..foo_link.1], "FOO", "FOO span should be exact");

        // offset for BAR
        let bar_off = code.find(" BAR;").unwrap() + 1;
        let bar_links = provider.find_declaration(bar_off, 0);
        assert!(bar_links.is_some(), "Should find BAR");
        let bar_links = bar_links.unwrap();
        assert!(!bar_links.is_empty(), "Should have at least one link for BAR");
        let bar_link = &bar_links[0].target_selection_range;
        assert_eq!(&code[bar_link.0..bar_link.1], "BAR", "BAR span should be exact");
    }

    #[test]
    fn crlf_with_trailing_emoji_clamp_and_roundtrip() {
        use perl_parser::positions::LineStartsCache;

        // "A🐍\r\n" — UTF-16 columns: 'A'->1, '🐍'->+2, total line len = 3 before CR
        let text = "A🐍\r\n";
        let cache = LineStartsCache::new(text);

        // place caret far past EOL on line 0, ensure clamp then round-trip
        let off = cache.position_to_offset(text, 0, 999);
        let (line, col) = cache.offset_to_position(text, off);
        assert_eq!((line, col), (0, 3), "Should clamp to end of line which is column 3 in UTF-16");
    }
}
