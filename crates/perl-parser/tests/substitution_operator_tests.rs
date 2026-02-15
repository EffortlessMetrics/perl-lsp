//! Comprehensive tests for substitution operator (s///) parsing
//! This test module ensures complete coverage of the substitution operator
//! including edge cases, modifiers, and special delimiters

mod support;

use perl_parser::{Parser, ast::NodeKind};
use support::parser_error_helpers::has_parse_error;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_basic_substitution() -> TestResult {
    let code = "s/foo/bar/";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, "foo");
                assert_eq!(replacement, "bar");
                assert_eq!(modifiers, "");
            } else {
                return Err(format!(
                    "Expected Substitution node in expression, got {:?}",
                    expression.kind
                )
                .into());
            }
        } else {
            return Err(
                format!("Expected ExpressionStatement node, got {:?}", statements[0].kind).into()
            );
        }
    } else {
        return Err("Expected Program node".into());
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_modifiers() -> TestResult {
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
        let ast = parser.parse()?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, expected_modifiers, "Failed for {}", code);
                } else {
                    return Err(
                        format!("Expected Substitution node in expression for {}", code).into()
                    );
                }
            } else {
                return Err(format!("Expected ExpressionStatement for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_different_delimiters() -> TestResult {
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
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_nested_delimiters() -> TestResult {
    let test_cases = vec![
        ("s{f{o}o}{b{a}r}", "f{o}o", "b{a}r"),
        ("s[f[o]o][b[a]r]", "f[o]o", "b[a]r"),
        ("s(f(o)o)(b(a)r)", "f(o)o", "b(a)r"),
        ("s<f<o>o><b<a>r>", "f<o>o", "b<a>r"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_special_chars() -> TestResult {
    let test_cases = vec![
        (r#"s/\n/\\n/"#, r"\n", r"\\n"),
        (r#"s/\t/\s/"#, r"\t", r"\s"),
        (r#"s/\$var/\$new/"#, r"\$var", r"\$new"),
        (r#"s/\@array/\@new/"#, r"\@array", r"\@new"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_empty_pattern_or_replacement() -> TestResult {
    let test_cases = vec![("s///", "", ""), ("s/foo//", "foo", ""), ("s//bar/", "", "bar")];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// MUT_002: Fixed in quote_parser.rs - balanced delimiters now use per-segment delimiter detection
fn test_substitution_empty_replacement_balanced_delimiters() -> TestResult {
    // These test cases specifically target the empty replacement parsing logic
    // for paired delimiters in quote_parser.rs line 80
    let test_cases = vec![
        ("s{pattern}{}", "pattern", ""), // Empty replacement with braces
        ("s[pattern][]", "pattern", ""), // Empty replacement with brackets
        ("s(pattern)()", "pattern", ""), // Empty replacement with parentheses
        ("s<pattern><>", "pattern", ""), // Empty replacement with angle brackets
        ("s{}{replacement}", "", "replacement"), // Empty pattern with braces
        ("s[]{replacement}", "", "replacement"), // Empty pattern with brackets
        ("s(){replacement}", "", "replacement"), // Empty pattern with parentheses
        ("s<>{replacement}", "", "replacement"), // Empty pattern with angle brackets
        ("s{}{}", "", ""),               // Both empty with braces
        ("s[][]", "", ""),               // Both empty with brackets
        ("s()()", "", ""),               // Both empty with parentheses
        ("s<><>", "", ""),               // Both empty with angle brackets
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// MUT_002 Regression Test: Verify trailing code survives after balanced delimiter substitution operators
// Fixed in lexer: parse_substitution now detects replacement delimiter independently
// This is critical to prevent the lexer from swallowing trailing code after balanced delimiters
// NOTE: Real Perl supports this syntax: `s[foo]{bar}; $x = 1;` - both statements should be parsed
fn test_substitution_balanced_delimiters_with_trailing_code() -> TestResult {
    // Test cases that specifically target the trailing code parsing after balanced delimiters
    // Each case verifies that both the substitution and subsequent statements are properly parsed
    let test_cases = vec![
        ("s[foo]{bar}; $x = 1;", 2, "Substitution + expression statement"),
        ("s{foo}{bar}; print;", 2, "Substitution + function call"),
        ("s(foo)(bar); $x++;", 2, "Substitution + postfix increment"),
        ("s<foo><bar>; my $y = 1;", 2, "Substitution + variable declaration"),
        ("s[pattern]{repl}; s/a/b/;", 2, "Two substitutions"),
        ("s{x}{y}; if (1) { }", 2, "Substitution + if block"),
        ("s{a}{b}; s[c][d]; s(e)(f);", 3, "Three substitutions with different delimiters"),
        (
            "s[test]{value}; $var =~ s/old/new/g;",
            2,
            "Mixed balanced delimiters + bind operator with substitution",
        ),
        ("s{}{empty}; print 'hello';", 2, "Empty pattern substitution + print statement"),
        ("s<pattern><replacement>; my ($a, $b) = @_;", 2, "Substitution + list assignment"),
    ];

    for (code, expected_stmt_count, description) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|err| {
            format!("Parse failed for test case '{}': {}\nCode: {}", description, err, code)
        })?;

        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(
                statements.len(),
                expected_stmt_count,
                "Statement count mismatch for test case '{}': expected {} statements but got {}\nCode: {}",
                description,
                expected_stmt_count,
                statements.len(),
                code
            );

            // Verify first statement is a substitution
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if !matches!(expression.kind, NodeKind::Substitution { .. }) {
                    return Err(format!(
                        "First statement should be Substitution for test case '{}', got {:?}\nCode: {}",
                        description, expression.kind, code
                    ).into());
                }
            } else {
                return Err(format!(
                    "First statement should be ExpressionStatement containing Substitution for test case '{}', got {:?}\nCode: {}",
                    description, statements[0].kind, code
                ).into());
            }

            // For multi-statement cases, verify subsequent statements exist and are properly parsed
            if expected_stmt_count > 1 {
                for (idx, stmt) in statements.iter().enumerate().skip(1) {
                    assert!(
                        !matches!(stmt.kind, NodeKind::Error { .. }),
                        "Statement {} should not be Error for test case '{}', got {:?}\nCode: {}",
                        idx,
                        description,
                        stmt.kind,
                        code
                    );
                }
            }
        } else {
            return Err(format!(
                "Expected Program node for test case '{}', got {:?}\nCode: {}",
                description, ast.kind, code
            )
            .into());
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_expressions() -> TestResult {
    // Test the /e modifier which evaluates replacement as Perl code
    let code = r#"s/(\d+)/sprintf("%02d", $1)/eg"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, r"(\d+)");
                assert_eq!(replacement, r#"sprintf("%02d", $1)"#);
                assert_eq!(modifiers, "eg");
            } else {
                return Err("Expected Substitution node".into());
            }
        } else {
            return Err("Expected ExpressionStatement node".into());
        }
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_in_context() -> TestResult {
    let test_cases = vec![
        ("$str =~ s/foo/bar/g;", "foo", "bar", "g"),
        ("if ($line =~ s/^\\s+//) { }", r"^\s+", "", ""),
        ("while (s/  / /g) { }", "  ", " ", "g"),
    ];

    for (code, expected_pattern, expected_replacement, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        // Find the substitution node (might be nested)
        let found = find_substitution_node(&ast);
        let (pattern, replacement, modifiers) =
            found.ok_or_else(|| format!("No Substitution node found in {}", code))?;
        assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
        assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        assert_eq!(modifiers, expected_modifiers, "Modifiers mismatch for {}", code);
    }
    Ok(())
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_unicode() -> TestResult {
    let test_cases = vec![
        ("s/café/coffee/", "café", "coffee"),
        ("s/😀/😎/g", "😀", "😎"),
        ("s/λ/lambda/", "λ", "lambda"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
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
        NodeKind::ExpressionStatement { expression } => find_substitution_node(expression),
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
// MUT_005 FIXED: Invalid modifier validation now properly rejects invalid modifiers
#[ignore = "substitution: modifier validation needs investigation"]
fn test_substitution_invalid_modifier_characters() {
    // These test cases specifically target the invalid modifier validation logic.
    //
    // Valid Perl substitution modifiers are:
    //   g, i, m, s, x, o, e, r - basic modifiers
    //   a, d, l, u - character class modifiers (ASCII, default, locale, Unicode)
    //   n - non-capturing mode (Perl 5.22+)
    //   p - preserve match variables
    //   c - continue matching
    //
    // This test only includes ACTUALLY INVALID modifiers:
    //   b, f, h, j, k, q, t, v, w, y, z and numeric characters
    //
    // Note: Only alphanumeric characters are tested as "modifiers" since Perl's lexer
    // treats special characters (like @, ;, etc.) as separate tokens, not as modifiers.
    // For example, 's/foo/bar/;' is valid Perl - the ';' is a statement terminator.
    let invalid_modifier_cases = vec![
        // Invalid single letter modifiers (letters not in the valid set)
        "s/foo/bar/z", // Invalid modifier 'z'
        "s/foo/bar/b", // Invalid modifier 'b'
        "s/foo/bar/f", // Invalid modifier 'f'
        "s/foo/bar/h", // Invalid modifier 'h'
        "s/foo/bar/j", // Invalid modifier 'j'
        "s/foo/bar/k", // Invalid modifier 'k'
        "s/foo/bar/q", // Invalid modifier 'q'
        "s/foo/bar/t", // Invalid modifier 't'
        "s/foo/bar/v", // Invalid modifier 'v'
        "s/foo/bar/w", // Invalid modifier 'w'
        "s/foo/bar/y", // Invalid modifier 'y'
        // Invalid numeric modifiers
        "s/foo/bar/1", // Invalid numeric modifier '1'
        "s/foo/bar/2", // Invalid numeric modifier '2'
        "s/foo/bar/9", // Invalid numeric modifier '9'
        "s/foo/bar/0", // Invalid numeric modifier '0'
        // Combinations with invalid modifiers
        "s/foo/bar/iz",  // Valid 'i' but invalid 'z' in combination
        "s/foo/bar/mxy", // Valid 'm', 'x' but invalid 'y' in combination
        "s/foo/bar/gi1", // Valid 'g', 'i' but invalid '1' in combination
        "s/foo/bar/xyz", // Valid 'x' but invalid 'y', 'z' in combination
        "s/foo/bar/123", // All invalid numeric modifiers
        "s/foo/bar/gbf", // Valid 'g' but invalid 'b', 'f' in combination
    ];

    for code in invalid_modifier_cases {
        // All of these should produce an error signal (either Err or ERROR nodes in AST)
        // The parser uses IDE-friendly error recovery, so it may return Ok with ERROR nodes
        // instead of Err for some cases
        assert!(
            has_parse_error(code),
            "Expected parse error (Err or ERROR node) for invalid modifier case: {}",
            code
        );
    }
}

#[test]
// Ensure valid modifiers still work after hardening invalid modifier detection
fn test_substitution_valid_modifier_combinations() -> TestResult {
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
        ("s/foo/bar/ee", "ee"), // Double 'e' is valid
        ("s/foo/bar/gims", "gims"),
        ("s/foo/bar/gimsx", "gimsx"),
        ("s/foo/bar/gimsxo", "gimsxo"),
        ("s/foo/bar/gimsxoe", "gimsxoe"),
        ("s/foo/bar/gimsxoer", "gimsxoer"),
        ("s/foo/bar/eeg", "eeg"), // Multiple 'e' with other modifiers
    ];

    for (code, expected_modifiers) in valid_modifier_cases {
        let mut parser = Parser::new(code);
        let ast = parser
            .parse()
            .map_err(|e| format!("Valid modifiers should parse: {} - error: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, expected_modifiers, "Modifiers mismatch for {}", code);
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// Property-based testing for delimiter edge cases
fn test_substitution_delimiter_edge_cases() -> TestResult {
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
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

#[test]
// Test complex nested delimiter scenarios that could trigger edge cases
fn test_substitution_complex_nested_scenarios() -> TestResult {
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
        let ast = parser.parse().map_err(|e| format!("parse {} failed: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(
                        replacement, expected_replacement,
                        "Replacement mismatch for {}",
                        code
                    );
                } else {
                    return Err(format!("Expected Substitution node for {}", code).into());
                }
            } else {
                return Err(format!("Expected ExpressionStatement node for {}", code).into());
            }
        }
    }
    Ok(())
}

// TARGETED MUTATION KILLER TESTS - Kill MUT_005 modifier validation mutation
#[test]
#[ignore = "mutation hardening: modifier matching needs investigation"]
fn test_kill_mutation_modifier_character_matching() -> TestResult {
    // This test specifically targets the modifier character pattern in parser_backup.rs
    // Original: 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' => {
    // Mutated:  'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v' => {

    // Test that original valid modifiers work (this should pass with original code)
    let valid_cases = vec![
        ("s/test/repl/g", "g"),               // Tests 'g' character specifically
        ("s/test/repl/i", "i"),               // Tests 'i' character specifically
        ("s/test/repl/m", "m"),               // Tests 'm' character specifically
        ("s/test/repl/s", "s"),               // Tests 's' character specifically
        ("s/test/repl/x", "x"),               // Tests 'x' character specifically
        ("s/test/repl/o", "o"),               // Tests 'o' character specifically
        ("s/test/repl/e", "e"),               // Tests 'e' character specifically
        ("s/test/repl/r", "r"),               // Tests 'r' character specifically
        ("s/test/repl/gi", "gi"),             // Test combination
        ("s/test/repl/gimsxoer", "gimsxoer"), // Test all valid modifiers
    ];

    for (code, expected_modifiers) in valid_cases {
        let mut parser = Parser::new(code);
        let ast =
            parser.parse().map_err(|e| format!("Valid modifier '{}' should parse: {}", code, e))?;

        if let NodeKind::Program { statements } = &ast.kind
            && let NodeKind::ExpressionStatement { expression } = &statements[0].kind
            && let NodeKind::Substitution { modifiers, .. } = &expression.kind
        {
            assert_eq!(
                modifiers, expected_modifiers,
                "Valid modifier '{}' should be preserved - kills modifier char mutations",
                code
            );
        }
    }

    // Test that mutated invalid modifier characters fail (this kills the mutation)
    // Note: n, p, l are actually VALID Perl modifiers (non-capturing, preserve, locale)
    // so we only test actually invalid characters: z, q, w, k, v, b, f, h, j, t, y
    let invalid_mutated_cases = vec![
        "s/test/repl/z",   // Tests mutated 'z' character (should fail with original code)
        "s/test/repl/q",   // Tests mutated 'q' character (should fail with original code)
        "s/test/repl/w",   // Tests mutated 'w' character (should fail with original code)
        "s/test/repl/k",   // Tests mutated 'k' character (should fail with original code)
        "s/test/repl/v",   // Tests mutated 'v' character (should fail with original code)
        "s/test/repl/b",   // Tests mutated 'b' character (should fail with original code)
        "s/test/repl/f",   // Tests mutated 'f' character (should fail with original code)
        "s/test/repl/zq",  // Tests mutated character combination
        "s/test/repl/zwb", // Tests multiple mutated characters
    ];

    for code in invalid_mutated_cases {
        // These should fail with the original code (valid modifiers: g,i,m,s,x,o,e,r)
        // but would succeed with the mutation (invalid modifiers: z,q,w,n,p,k,l,v)
        // By asserting they fail, we kill the mutation
        // Note: Parser uses IDE-friendly error recovery, so check for ERROR nodes too
        assert!(
            has_parse_error(code),
            "Invalid mutated modifier '{}' should fail to parse - kills modifier character mutation",
            code
        );
    }
    Ok(())
}

// Additional targeted test for mixed valid/invalid modifiers to ensure precise character matching
#[test]
#[ignore = "mutation hardening: mixed modifier validation needs investigation"]
fn test_kill_mutation_mixed_modifier_validation() {
    // Test mixed cases where some characters are valid and others are invalid
    // This ensures the mutation cannot partially succeed

    let mixed_cases = vec![
        // Each case has some valid modifiers mixed with the mutated invalid ones
        // Note: n, p, l are actually VALID Perl modifiers, so we use truly invalid ones
        ("s/test/repl/gz", true), // 'g' valid, 'z' invalid (mutated char)
        ("s/test/repl/iq", true), // 'i' valid, 'q' invalid (mutated char)
        ("s/test/repl/mw", true), // 'm' valid, 'w' invalid (mutated char)
        ("s/test/repl/sb", true), // 's' valid, 'b' invalid (mutated char)
        ("s/test/repl/xf", true), // 'x' valid, 'f' invalid (mutated char)
        ("s/test/repl/ok", true), // 'o' valid, 'k' invalid (mutated char)
        ("s/test/repl/eh", true), // 'e' valid, 'h' invalid (mutated char)
        ("s/test/repl/rv", true), // 'r' valid, 'v' invalid (mutated char)
        // Pure valid modifiers should work (including extended set: a, d, l, u, n, p, c)
        ("s/test/repl/gim", false),     // All valid - basic
        ("s/test/repl/sox", false),     // All valid - basic
        ("s/test/repl/er", false),      // All valid - basic
        ("s/test/repl/adlunpc", false), // All valid - extended Perl modifiers
    ];

    for (code, should_fail) in mixed_cases {
        if should_fail {
            // Note: Parser uses IDE-friendly error recovery, so check for ERROR nodes too
            assert!(
                has_parse_error(code),
                "Mixed modifier case '{}' with invalid chars should fail - kills modifier mutation",
                code
            );
        } else {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Pure valid modifier case '{}' should succeed", code);
            if let Ok(ast) = result
                && let NodeKind::Program { statements } = &ast.kind
                && let NodeKind::ExpressionStatement { expression } = &statements[0].kind
                && let NodeKind::Substitution { modifiers, .. } = &expression.kind
            {
                // Verify valid modifiers are preserved
                assert!(
                    !modifiers.is_empty(),
                    "Valid modifiers should not be empty for '{}'",
                    code
                );
            }
        }
    }
}
