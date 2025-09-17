/// Comprehensive tests for substitution operator (s///) parsing
/// This test module ensures complete coverage of the substitution operator
/// including edge cases, modifiers, and special delimiters
use perl_parser::{Parser, ast::NodeKind};


#[test]
// #[ignore = "substitution operator not implemented"]
fn test_basic_substitution() {
    let code = "s/foo/bar/";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, "foo");
                assert_eq!(replacement, "bar");
                assert_eq!(modifiers, "");
            } else {
                panic!("Expected Substitution node in expression, got {:?}", expression.kind);
            }
        } else {
            panic!("Expected ExpressionStatement node, got {:?}", statements[0].kind);
        }
    } else {
        panic!("Expected Program node");
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_modifiers() {
    let test_cases = vec![
        ("s/foo/bar/g", "g"),
        ("s/foo/bar/i", "i"),
        ("s/foo/bar/gi", "gi"),
        ("s/foo/bar/gix", "gix"),
        ("s/foo/bar/msxi", "msxi"),
        ("s/foo/bar/e", "e"),
        ("s/foo/bar/ee", "ee"),
        ("s/foo/bar/eeg", "eeg"),
        ("s/foo/bar/r", "r"),
    ];

    for (code, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, expected_modifiers, "Failed for {}", code);
                } else {
                    panic!("Expected Substitution node in expression for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_different_delimiters() {
    let test_cases = vec![
        ("s(foo)(bar)", "foo", "bar"),
        ("s{foo}{bar}", "foo", "bar"),
        ("s[foo][bar]", "foo", "bar"),
        ("s<foo><bar>", "foo", "bar"),
        ("s#foo#bar#", "foo", "bar"),
        ("s!foo!bar!", "foo", "bar"),
        ("s|foo|bar|", "foo", "bar"),
        ("s,foo,bar,", "foo", "bar"),
        ("s'foo'bar'", "foo", "bar"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_nested_delimiters() {
    let test_cases = vec![
        ("s{f{o}o}{b{a}r}", "f{o}o", "b{a}r"),
        ("s[f[o]o][b[a]r]", "f[o]o", "b[a]r"),
        ("s(f(o)o)(b(a)r)", "f(o)o", "b(a)r"),
        ("s<f<o>o><b<a>r>", "f<o>o", "b<a>r"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_special_chars() {
    let test_cases = vec![
        (r#"s/\n/\\n/"#, r"\n", r"\\n"),
        (r#"s/\t/\s/"#, r"\t", r"\s"),
        (r#"s/\$var/\$new/"#, r"\$var", r"\$new"),
        (r#"s/\@array/\@new/"#, r"\@array", r"\@new"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_empty_pattern_or_replacement() {
    let test_cases = vec![("s///", "", ""), ("s/foo//", "foo", ""), ("s//bar/", "", "bar")];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
#[ignore = "MUT_002: Exposes empty replacement parsing bug in quote_parser.rs:80 - will kill mutant when fixed"]
// Target MUT_002: Empty replacement with balanced delimiters - quote_parser.rs:80
fn test_substitution_empty_replacement_balanced_delimiters() {
    // These test cases specifically target the empty replacement parsing logic
    // for paired delimiters in quote_parser.rs line 80
    let test_cases = vec![
        ("s{pattern}{}", "pattern", ""),  // Empty replacement with braces
        ("s[pattern][]", "pattern", ""),  // Empty replacement with brackets
        ("s(pattern)()", "pattern", ""),  // Empty replacement with parentheses
        ("s<pattern><>", "pattern", ""),  // Empty replacement with angle brackets
        ("s{}{replacement}", "", "replacement"),  // Empty pattern with braces
        ("s[]{replacement}", "", "replacement"),  // Empty pattern with brackets
        ("s(){replacement}", "", "replacement"),  // Empty pattern with parentheses
        ("s<>{replacement}", "", "replacement"),  // Empty pattern with angle brackets
        ("s{}{}", "", ""),              // Both empty with braces
        ("s[][]", "", ""),              // Both empty with brackets
        ("s()()", "", ""),              // Both empty with parentheses
        ("s<><>", "", ""),              // Both empty with angle brackets
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_expressions() {
    // Test the /e modifier which evaluates replacement as Perl code
    let code = r#"s/(\d+)/sprintf("%02d", $1)/eg"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");

    if let NodeKind::Program { statements } = &ast.kind {
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, r"(\d+)");
                assert_eq!(replacement, r#"sprintf("%02d", $1)"#);
                assert_eq!(modifiers, "eg");
            } else {
                panic!("Expected Substitution node");
            }
        } else {
            panic!("Expected ExpressionStatement node");
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_in_context() {
    let test_cases = vec![
        ("$str =~ s/foo/bar/g;", "foo", "bar", "g"),
        ("if ($line =~ s/^\\s+//) { }", r"^\s+", "", ""),
        ("while (s/  / /g) { }", "  ", " ", "g"),
    ];

    for (code, expected_pattern, expected_replacement, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        // Find the substitution node (might be nested)
        let found = find_substitution_node(&ast);
        assert!(found.is_some(), "No Substitution node found in {}", code);

        let (pattern, replacement, modifiers) = found.unwrap();
        assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
        assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        assert_eq!(modifiers, expected_modifiers, "Modifiers mismatch for {}", code);
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_unicode() {
    let test_cases = vec![
        ("s/café/coffee/", "café", "coffee"),
        ("s/😀/😎/g", "😀", "😎"),
        ("s/λ/lambda/", "λ", "lambda"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

// Helper function to find substitution node in AST
fn find_substitution_node(node: &perl_parser::ast::Node) -> Option<(String, String, String)> {
    match &node.kind {
        NodeKind::Substitution { pattern, replacement, modifiers, .. } => {
            Some((pattern.clone(), replacement.clone(), modifiers.clone()))
        }
        NodeKind::Program { statements } => {
            for stmt in statements {
                if let Some(result) = find_substitution_node(stmt) {
                    return Some(result);
                }
            }
            None
        }
        NodeKind::ExpressionStatement { expression } => {
            find_substitution_node(expression)
        }
        NodeKind::Binary { left, right, .. } => {
            find_substitution_node(left).or_else(|| find_substitution_node(right))
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                if let Some(result) = find_substitution_node(stmt) {
                    return Some(result);
                }
            }
            None
        }
        NodeKind::If { condition, then_branch, else_branch, .. } => {
            find_substitution_node(condition)
                .or_else(|| find_substitution_node(then_branch))
                .or_else(|| else_branch.as_ref().and_then(|b| find_substitution_node(b)))
        }
        NodeKind::While { condition, body, .. } => {
            find_substitution_node(condition).or_else(|| find_substitution_node(body))
        }
        _ => None,
    }
}

#[test]
#[ignore = "MUT_005: Exposes invalid modifier validation bug in parser_backup.rs:4231 - will kill mutant when fixed"]
// Target MUT_005: Invalid modifier character validation - parser_backup.rs:4231
fn test_substitution_invalid_modifier_characters() {
    // These test cases specifically target the invalid modifier validation logic
    // in parser_backup.rs line 4231 where only 'g', 'i', 'm', 's', 'x', 'o', 'e', 'r' are allowed
    let invalid_modifier_cases = vec![
        "s/foo/bar/z",    // Invalid modifier 'z'
        "s/foo/bar/a",    // Invalid modifier 'a'
        "s/foo/bar/b",    // Invalid modifier 'b'
        "s/foo/bar/c",    // Invalid modifier 'c'
        "s/foo/bar/d",    // Invalid modifier 'd'
        "s/foo/bar/f",    // Invalid modifier 'f'
        "s/foo/bar/h",    // Invalid modifier 'h'
        "s/foo/bar/j",    // Invalid modifier 'j'
        "s/foo/bar/k",    // Invalid modifier 'k'
        "s/foo/bar/l",    // Invalid modifier 'l'
        "s/foo/bar/n",    // Invalid modifier 'n'
        "s/foo/bar/p",    // Invalid modifier 'p'
        "s/foo/bar/q",    // Invalid modifier 'q'
        "s/foo/bar/t",    // Invalid modifier 't'
        "s/foo/bar/u",    // Invalid modifier 'u'
        "s/foo/bar/v",    // Invalid modifier 'v'
        "s/foo/bar/w",    // Invalid modifier 'w'
        "s/foo/bar/y",    // Invalid modifier 'y'
        "s/foo/bar/1",    // Invalid numeric modifier '1'
        "s/foo/bar/2",    // Invalid numeric modifier '2'
        "s/foo/bar/9",    // Invalid numeric modifier '9'
        "s/foo/bar/0",    // Invalid numeric modifier '0'
        "s/foo/bar/@",    // Invalid symbol modifier '@'
        "s/foo/bar/#",    // Invalid symbol modifier '#'
        "s/foo/bar/$",    // Invalid symbol modifier '$'
        "s/foo/bar/%",    // Invalid symbol modifier '%'
        "s/foo/bar/^",    // Invalid symbol modifier '^'
        "s/foo/bar/&",    // Invalid symbol modifier '&'
        "s/foo/bar/*",    // Invalid symbol modifier '*'
        "s/foo/bar/(",    // Invalid symbol modifier '('
        "s/foo/bar/)",    // Invalid symbol modifier ')'
        "s/foo/bar/-",    // Invalid symbol modifier '-'
        "s/foo/bar/+",    // Invalid symbol modifier '+'
        "s/foo/bar/=",    // Invalid symbol modifier '='
        "s/foo/bar/[",    // Invalid symbol modifier '['
        "s/foo/bar/]",    // Invalid symbol modifier ']'
        "s/foo/bar/{",    // Invalid symbol modifier '{'
        "s/foo/bar/}",    // Invalid symbol modifier '}'
        "s/foo/bar/|",    // Invalid symbol modifier '|'
        "s/foo/bar/\\",   // Invalid symbol modifier '\\'
        "s/foo/bar/:",    // Invalid symbol modifier ':'
        "s/foo/bar/;",    // Invalid symbol modifier ';'
        "s/foo/bar/\"",   // Invalid symbol modifier '"'
        "s/foo/bar/'",    // Invalid symbol modifier "'"
        "s/foo/bar/<",    // Invalid symbol modifier '<'
        "s/foo/bar/>",    // Invalid symbol modifier '>'
        "s/foo/bar/,",    // Invalid symbol modifier ','
        "s/foo/bar/.",    // Invalid symbol modifier '.'
        "s/foo/bar/?",    // Invalid symbol modifier '?'
        "s/foo/bar/ ",    // Invalid space modifier
        "s/foo/bar/\t",   // Invalid tab modifier
        "s/foo/bar/\n",   // Invalid newline modifier
        "s/foo/bar/\r",   // Invalid carriage return modifier
        "s/foo/bar/ga",   // Valid 'g' but invalid 'a' in combination
        "s/foo/bar/iz",   // Valid 'i' but invalid 'z' in combination
        "s/foo/bar/mxy",  // Valid 'm', 'x' but invalid 'y' in combination
        "s/foo/bar/gi1",  // Valid 'g', 'i' but invalid '1' in combination
        "s/foo/bar/xyz",  // Valid 'x' but invalid 'y', 'z' in combination
        "s/foo/bar/123",  // All invalid numeric modifiers
        "s/foo/bar/abc",  // Mix of invalid letters
        "s/foo/bar/!@#",  // Mix of invalid symbols
    ];

    for code in invalid_modifier_cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();

        // All of these should fail to parse due to invalid modifiers
        // The parser should detect the invalid modifier and return an error
        assert!(result.is_err(), "Expected parse error for invalid modifier case: {}", code);
    }
}

#[test]
// Ensure valid modifiers still work after hardening invalid modifier detection
fn test_substitution_valid_modifier_combinations() {
    // Test all valid single and combination modifiers to ensure they still work
    let valid_modifier_cases = vec![
        ("s/foo/bar/g", "g"),
        ("s/foo/bar/i", "i"),
        ("s/foo/bar/m", "m"),
        ("s/foo/bar/s", "s"),
        ("s/foo/bar/x", "x"),
        ("s/foo/bar/o", "o"),
        ("s/foo/bar/e", "e"),
        ("s/foo/bar/r", "r"),
        ("s/foo/bar/gi", "gi"),
        ("s/foo/bar/gm", "gm"),
        ("s/foo/bar/gs", "gs"),
        ("s/foo/bar/gx", "gx"),
        ("s/foo/bar/go", "go"),
        ("s/foo/bar/ge", "ge"),
        ("s/foo/bar/gr", "gr"),
        ("s/foo/bar/im", "im"),
        ("s/foo/bar/is", "is"),
        ("s/foo/bar/ix", "ix"),
        ("s/foo/bar/io", "io"),
        ("s/foo/bar/ie", "ie"),
        ("s/foo/bar/ir", "ir"),
        ("s/foo/bar/ms", "ms"),
        ("s/foo/bar/mx", "mx"),
        ("s/foo/bar/mo", "mo"),
        ("s/foo/bar/me", "me"),
        ("s/foo/bar/mr", "mr"),
        ("s/foo/bar/sx", "sx"),
        ("s/foo/bar/so", "so"),
        ("s/foo/bar/se", "se"),
        ("s/foo/bar/sr", "sr"),
        ("s/foo/bar/xo", "xo"),
        ("s/foo/bar/xe", "xe"),
        ("s/foo/bar/xr", "xr"),
        ("s/foo/bar/oe", "oe"),
        ("s/foo/bar/or", "or"),
        ("s/foo/bar/er", "er"),
        ("s/foo/bar/ee", "ee"),  // Double 'e' is valid
        ("s/foo/bar/gims", "gims"),
        ("s/foo/bar/gimsx", "gimsx"),
        ("s/foo/bar/gimsxo", "gimsxo"),
        ("s/foo/bar/gimsxoe", "gimsxoe"),
        ("s/foo/bar/gimsxoer", "gimsxoer"),
        ("s/foo/bar/eeg", "eeg"),  // Multiple 'e' with other modifiers
    ];

    for (code, expected_modifiers) in valid_modifier_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("Valid modifiers should parse: {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, expected_modifiers, "Modifiers mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement for {}", code);
            }
        }
    }
}

#[test]
// Property-based testing for delimiter edge cases
fn test_substitution_delimiter_edge_cases() {
    // Test edge cases with different delimiter combinations
    let edge_cases = vec![
        // Single delimiter character edge cases
        ("s#pattern#replacement#", "pattern", "replacement"),
        ("s|pattern|replacement|", "pattern", "replacement"),
        ("s!pattern!replacement!", "pattern", "replacement"),
        ("s@pattern@replacement@", "pattern", "replacement"),
        ("s%pattern%replacement%", "pattern", "replacement"),
        ("s^pattern^replacement^", "pattern", "replacement"),
        ("s&pattern&replacement&", "pattern", "replacement"),
        ("s*pattern*replacement*", "pattern", "replacement"),
        ("s+pattern+replacement+", "pattern", "replacement"),
        ("s=pattern=replacement=", "pattern", "replacement"),
        ("s~pattern~replacement~", "pattern", "replacement"),
        ("s:pattern:replacement:", "pattern", "replacement"),
        ("s;pattern;replacement;", "pattern", "replacement"),
        ("s,pattern,replacement,", "pattern", "replacement"),
        ("s.pattern.replacement.", "pattern", "replacement"),
        ("s?pattern?replacement?", "pattern", "replacement"),
        // Single quote delimiter (special case)
        ("s'pattern'replacement'", "pattern", "replacement"),
        // Test with modifiers on different delimiters
        ("s#pattern#replacement#g", "pattern", "replacement"),
        ("s|pattern|replacement|gi", "pattern", "replacement"),
        ("s!pattern!replacement!gim", "pattern", "replacement"),
        ("s@pattern@replacement@gims", "pattern", "replacement"),
        ("s%pattern%replacement%gimsx", "pattern", "replacement"),
        ("s'pattern'replacement'gimsxoer", "pattern", "replacement"),
    ];

    for (code, expected_pattern, expected_replacement) in edge_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// Test complex nested delimiter scenarios that could trigger edge cases
fn test_substitution_complex_nested_scenarios() {
    let complex_cases = vec![
        // Deep nesting
        ("s{a{b{c}d}e}{x{y{z}w}v}", "a{b{c}d}e", "x{y{z}w}v"),
        ("s[a[b[c]d]e][x[y[z]w]v]", "a[b[c]d]e", "x[y[z]w]v"),
        ("s(a(b(c)d)e)(x(y(z)w)v)", "a(b(c)d)e", "x(y(z)w)v"),
        ("s<a<b<c>d>e><x<y<z>w>v>", "a<b<c>d>e", "x<y<z>w>v"),
        // Mixed nesting types within same delimiter
        ("s{a[b]c}{x(y)z}", "a[b]c", "x(y)z"),
        ("s[a{b}c][x(y)z]", "a{b}c", "x(y)z"),
        ("s(a{b}[c])(x[y]{z})", "a{b}[c]", "x[y]{z}"),
        ("s<a{b}[c](d)><x[y]{z}(w)>", "a{b}[c](d)", "x[y]{z}(w)"),
        // Empty nested structures
        ("s{a{}b}{x{}y}", "a{}b", "x{}y"),
        ("s[a[]b][x[]y]", "a[]b", "x[]y"),
        ("s(a()b)(x()y)", "a()b", "x()y"),
        ("s<a<>b><x<>y>", "a<>b", "x<>y"),
    ];

    for (code, expected_pattern, expected_replacement) in complex_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}
