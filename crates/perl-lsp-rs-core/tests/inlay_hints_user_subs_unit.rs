//! Unit tests for user-defined sub parameter inlay hints (#794).
//!
//! Tests the `parameter_hints` function in the inlay_hints provider directly,
//! using the parser to build real ASTs.

use perl_lsp_rs_core::providers::inlay_hints::parameter_hints;

/// Parse source into an AST node.
fn ast_for(source: &str) -> Result<perl_parser_core::ast::Node, Box<dyn std::error::Error>> {
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;
    Ok(ast)
}

/// Dummy position converter: returns (byte_offset / 100, byte_offset % 100).
/// Good enough for checking that hints appear (not exact positions).
fn dummy_pos(offset: usize) -> (u32, u32) {
    ((offset / 100) as u32, (offset % 100) as u32)
}

// ---------------------------------------------------------------------------
// Basic sub with two mandatory params
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_user_sub_two_params() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
sub greet($name, $greeting) { print "$greeting $name\n"; }
greet("Alice", "Hello");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(
        labels.contains(&"name:"),
        "Expected 'name:' hint for first arg of greet; labels: {labels:?}"
    );
    assert!(
        labels.contains(&"greeting:"),
        "Expected 'greeting:' hint for second arg of greet; labels: {labels:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Single-param sub should get no hints (noise-free policy)
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_user_sub_single_param_suppressed() -> Result<(), Box<dyn std::error::Error>>
{
    let src = r#"
sub say_it($msg) { print "$msg\n"; }
say_it("hello world");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(
        !labels.contains(&"msg:"),
        "Should suppress hint for single-param sub; labels: {labels:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Sub without a signature gets no hints
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_user_sub_no_signature_suppressed() -> Result<(), Box<dyn std::error::Error>>
{
    let src = r#"
sub old_style { my ($x, $y) = @_; $x + $y }
old_style(1, 2);
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(
        !labels.contains(&"x:"),
        "Should not hint for sub without formal signature; labels: {labels:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Builtins are not double-hinted
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_no_double_hint_for_builtins() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
open(FH, "<", "file.txt");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    // Count how many "filehandle:" labels there are -- should be exactly 1
    let count = hints.iter().filter(|h| h["label"].as_str() == Some("filehandle:")).count();
    assert_eq!(count, 1, "Should have exactly one filehandle: hint; hints: {hints:#?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Three-param sub: all positional args are hinted
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_user_sub_three_params() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
sub connect_db($host, $port, $dbname) { 1 }
connect_db("localhost", 5432, "mydb");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(labels.contains(&"host:"), "Expected 'host:'; labels: {labels:?}");
    assert!(labels.contains(&"port:"), "Expected 'port:'; labels: {labels:?}");
    assert!(labels.contains(&"dbname:"), "Expected 'dbname:'; labels: {labels:?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Slurpy (@rest) param: hints stop at slurpy boundary
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_user_sub_stops_at_slurpy() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
sub log_msg($level, @messages) { 1 }
log_msg("info", "hello", "world");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    // "level:" should be hinted
    assert!(labels.contains(&"level:"), "Expected 'level:' hint; labels: {labels:?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Unresolved call: no crash, no spurious hints
// ---------------------------------------------------------------------------
#[test]
fn test_parameter_hints_unresolved_call_no_hints() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
some_external_function("a", "b");
"#;
    let ast = ast_for(src)?;
    let hints = parameter_hints(&ast, &dummy_pos, None);

    let user_labels: Vec<&str> = hints
        .iter()
        .filter_map(|h| h["label"].as_str())
        .filter(|l| *l == "a:" || *l == "b:")
        .collect();

    assert!(
        user_labels.is_empty(),
        "Should not produce hints for unresolved calls; labels: {user_labels:?}"
    );
    Ok(())
}
