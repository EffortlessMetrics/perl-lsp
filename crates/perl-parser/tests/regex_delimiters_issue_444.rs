use perl_parser::Parser;

#[test]
fn test_m_operator_with_exclamation_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m!pattern!;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Regex") || s.contains("Match"), "Expected regex match: {}", s);
    Ok(())
}

#[test]
fn test_m_operator_with_brace_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m{pattern};"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Regex") || s.contains("Match"), "Expected regex match: {}", s);
    Ok(())
}

#[test]
fn test_m_operator_with_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m|pattern|;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Regex") || s.contains("Match"), "Expected regex match: {}", s);
    Ok(())
}

#[test]
fn test_s_operator_with_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ s|old|new|g;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(s.contains("Substitution") || s.contains("Subst"), "Expected substitution: {}", s);
    Ok(())
}

#[test]
fn test_m_operator_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m{pattern{nested}};"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Regex") || s.contains("Match"),
        "Expected regex match with nested braces: {}",
        s
    );
    Ok(())
}

#[test]
fn test_m_operator_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"$text =~ m!pattern!i;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let s = format!("{:?}", ast);
    assert!(
        s.contains("Regex") || s.contains("Match"),
        "Expected regex match with modifiers: {}",
        s
    );
    Ok(())
}
