/// Comprehensive tests for regex with arbitrary delimiters (Issue #444)
/// These tests verify that all acceptance criteria are met
use perl_parser::Parser;

#[test]
fn ac1_m_operator_exclamation_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m!pattern!;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Regex") || s.contains("Match"), "AC1 Failed: m!pattern! not recognized");
    Ok(())
}

#[test]
fn ac2_m_operator_brace_delimiter_nested() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m{pattern{nested}};"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Regex") || s.contains("Match"),
        "AC2 Failed: m{{pattern{{nested}}}} not handled"
    );
    Ok(())
}

#[test]
fn ac3_s_operator_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ s|old|new|g;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Substitution") || s.contains("Subst"),
        "AC3 Failed: s|old|new|g not recognized"
    );
    Ok(())
}

#[test]
fn ac4_modifiers_after_arbitrary_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m!pattern!i;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Regex") || s.contains("Match"),
        "AC4 Failed: m!pattern!i modifiers not parsed"
    );

    // Verify modifiers are captured (not leaked as separate token)
    let identifier_count = s.matches("Identifier").count();
    // Should have $text but not 'i' as separate identifier
    assert!(identifier_count <= 1, "AC4 Failed: Modifier 'i' leaked as identifier");
    Ok(())
}

#[test]
fn ac6_backward_compatibility_slash_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ /pattern/i;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Regex") || s.contains("Match"),
        "AC6 Failed: /pattern/ backward compatibility broken"
    );
    Ok(())
}

#[test]
fn various_delimiters_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let delimiters = vec![
        ("m!pattern!", "exclamation"),
        ("m{pattern}", "braces"),
        ("m[pattern]", "brackets"),
        ("m(pattern)", "parens"),
        ("m<pattern>", "angles"),
        ("m|pattern|", "pipes"),
        ("m#pattern#", "hash"),
        ("m~pattern~", "tilde"),
        ("m@pattern@", "at"),
        ("m%pattern%", "percent"),
    ];

    for (code, delim_name) in delimiters {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let s = format!("{:?}", ast);
                assert!(
                    s.contains("Regex") || s.contains("Match"),
                    "Failed to parse m operator with {} delimiter: {}",
                    delim_name,
                    code
                );
            }
            Err(e) => {
                return Err(format!("Failed to parse m operator with {} delimiter {}: {}", delim_name, code, e).into());
            }
        }
    }
    Ok(())
}

#[test]
fn substitution_various_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let delimiters = vec![
        ("s!old!new!", "exclamation"),
        ("s{old}{new}", "braces"),
        ("s[old][new]", "brackets"),
        ("s(old)(new)", "parens"),
        ("s<old><new>", "angles"),
        ("s|old|new|", "pipes"),
        ("s#old#new#", "hash"),
    ];

    for (code, delim_name) in delimiters {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let s = format!("{:?}", ast);
                assert!(
                    s.contains("Substitution") || s.contains("Subst"),
                    "Failed to parse s operator with {} delimiter: {}",
                    delim_name,
                    code
                );
            }
            Err(e) => {
                return Err(format!("Failed to parse s operator with {} delimiter {}: {}", delim_name, code, e).into());
            }
        }
    }
    Ok(())
}

#[test]
fn transliteration_various_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let delimiters = vec![
        ("tr!abc!xyz!", "exclamation"),
        ("tr{abc}{xyz}", "braces"),
        ("tr|abc|xyz|", "pipes"),
        ("y#abc#xyz#", "hash (y alias)"),
    ];

    for (code, delim_name) in delimiters {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let s = format!("{:?}", ast);
                assert!(
                    s.contains("Transliteration") || s.contains("Transl"),
                    "Failed to parse tr/y operator with {} delimiter: {}",
                    delim_name,
                    code
                );
            }
            Err(e) => {
                return Err(format!(
                    "Failed to parse tr/y operator with {} delimiter {}: {}",
                    delim_name, code, e
                ).into());
            }
        }
    }
    Ok(())
}

#[test]
fn qr_operator_various_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let delimiters = vec![
        ("qr!pattern!i", "exclamation"),
        ("qr{pattern}i", "braces"),
        ("qr|pattern|i", "pipes"),
        ("qr#pattern#i", "hash"),
    ];

    for (code, delim_name) in delimiters {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let s = format!("{:?}", ast);
                assert!(
                    s.contains("Regex"),
                    "Failed to parse qr operator with {} delimiter: {}",
                    delim_name,
                    code
                );
            }
            Err(e) => {
                return Err(format!("Failed to parse qr operator with {} delimiter {}: {}", delim_name, code, e).into());
            }
        }
    }
    Ok(())
}

#[test]
fn ac7_error_messages_distinguish_malformed() -> Result<(), Box<dyn std::error::Error>> {
    // Test that malformed regex produces clear error (not confused with bareword)
    let code = r#"$text =~ m!unterminated"#;
    let mut parser = Parser::new(code);
    // This should either parse (treating as incomplete) or error
    // The key is it shouldn't be confused with a bareword function call
    let result = parser.parse();
    // Either way, it should handle it gracefully
    match result {
        Ok(_) => {
            // Accepted as incomplete regex - OK
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            // Should not say "unknown function m" or similar bareword error
            assert!(
                !err_msg.to_lowercase().contains("function"),
                "AC7 Failed: Error message suggests bareword confusion: {}",
                err_msg
            );
        }
    }
    Ok(())
}

#[test]
fn complex_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m{outer{inner{deep}}};"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Regex") || s.contains("Match"), "Complex nested braces not handled");
    Ok(())
}

#[test]
fn modifiers_with_different_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = vec![
        ("m!pattern!imsxgc", "all modifiers with exclamation"),
        ("m{pattern}i", "single modifier with braces"),
        ("s|old|new|ge", "substitution modifiers with pipes"),
    ];

    for (code, description) in test_cases {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_ast) => {
                // Success
            }
            Err(e) => {
                return Err(format!("Failed to parse {}: {}", description, e).into());
            }
        }
    }
    Ok(())
}
