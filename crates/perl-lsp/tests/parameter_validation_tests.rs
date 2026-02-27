use perl_lsp::features::diagnostics::DiagnosticsProvider;
use perl_parser::Parser;
use std::sync::Arc;

#[test]
fn test_duplicate_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub test($x, $y, $x) {
    print $x + $y;
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should have one error for duplicate parameter
    let duplicate_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("duplicate-parameter")).collect();

    assert_eq!(duplicate_errors.len(), 1);
    assert!(duplicate_errors[0].message.contains("Duplicate parameter"));
    assert!(duplicate_errors[0].message.contains("$x"));
    Ok(())
}

#[test]
fn test_parameter_shadows_global() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
my $count = 10;

sub increment($count) {
    return $count + 1;
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should have one warning for parameter shadowing
    let shadow_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some("parameter-shadows-global"))
        .collect();

    assert_eq!(shadow_warnings.len(), 1);
    assert!(shadow_warnings[0].message.contains("shadows"));
    assert!(shadow_warnings[0].message.contains("$count"));
    Ok(())
}

#[test]
fn test_unused_parameter() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub helper($x, $y, $z) {
    return $x + $y;  # $z is unused
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should have one warning for unused parameter
    let unused_warnings: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unused-parameter")).collect();

    assert_eq!(unused_warnings.len(), 1);
    assert!(unused_warnings[0].message.contains("never used"));
    assert!(unused_warnings[0].message.contains("$z"));
    Ok(())
}

#[cfg(feature = "lsp-extras")]
#[test]
fn test_bareword_under_strict() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;

print FOO;  # Bareword not allowed
my $hash = { key => value };  # These barewords should also be flagged
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should have errors for barewords
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    assert!(!bareword_errors.is_empty());
    assert!(bareword_errors[0].message.contains("Bareword"));
    assert!(bareword_errors[0].message.contains("not allowed"));
    Ok(())
}

#[test]
fn test_parameter_with_at_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub variadic($first, @rest) {
    print $first;
    print @rest;
}

sub legacy_style {
    my ($x, $y) = @_;  # @_ usage is fine
    return $x + $y;
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should not flag @_ usage
    let false_positives: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("@_")).collect();

    assert_eq!(false_positives.len(), 0, "Should not flag @_ usage");
    Ok(())
}

#[test]
fn test_parameter_intentionally_unused() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub callback($event, $_unused_data) {
    print "Event: $event\n";
    # $_unused_data is intentionally unused
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should not flag parameters starting with underscore as unused
    let unused_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some("unused-parameter"))
        .filter(|d| d.message.contains("$_unused_data"))
        .collect();

    assert_eq!(unused_warnings.len(), 0, "Should not flag underscore-prefixed parameters");
    Ok(())
}

#[test]
fn test_multiple_duplicate_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub complex($a, $b, $a, $c, $b) {
    return $a + $b + $c;
}
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let ast = Arc::new(ast);
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Should have errors for both duplicate parameters
    let duplicate_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("duplicate-parameter")).collect();

    assert_eq!(duplicate_errors.len(), 2, "Should detect both duplicate parameters");
    Ok(())
}
