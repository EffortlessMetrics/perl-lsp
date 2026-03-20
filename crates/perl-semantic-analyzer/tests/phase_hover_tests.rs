//! Tests for BEGIN/END/CHECK/INIT/UNITCHECK phase block hover documentation.
//!
//! Phase blocks have distinct execution semantics in Perl:
//! - BEGIN: compile-time execution
//! - END: interpreter shutdown
//! - INIT: post-compile, pre-runtime
//! - CHECK: end of compilation, before INIT
//! - UNITCHECK: after each compilation unit compiles
//!
//! Each phase block should produce hover info explaining when it runs.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_semantic_analyzer::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the first PhaseBlock node in the AST, returning its location.
fn find_phase_block_location(node: &Node) -> Option<SourceLocation> {
    match &node.kind {
        NodeKind::PhaseBlock { .. } => Some(node.location),
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                if let Some(loc) = find_phase_block_location(stmt) {
                    return Some(loc);
                }
            }
            None
        }
        NodeKind::ExpressionStatement { expression } => find_phase_block_location(expression),
        _ => None,
    }
}

fn parse_and_analyze(code: &str) -> Result<(Node, SemanticAnalyzer), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    Ok((ast, analyzer))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_begin_phase_block_hover_contains_compile_time() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { my $x = 1; }";
    let (ast, analyzer) = parse_and_analyze(code)?;

    let loc = find_phase_block_location(&ast).ok_or("no PhaseBlock node found")?;
    let hover = analyzer.hover_at(loc).ok_or("no hover info for BEGIN block")?;

    assert!(
        hover.signature.to_lowercase().contains("begin")
            || hover.signature.to_lowercase().contains("compile"),
        "BEGIN hover signature should mention 'begin' or 'compile', got: {}",
        hover.signature
    );
    Ok(())
}

#[test]
fn test_end_phase_block_hover_contains_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let code = "END { print 'done'; }";
    let (ast, analyzer) = parse_and_analyze(code)?;

    let loc = find_phase_block_location(&ast).ok_or("no PhaseBlock node found")?;
    let hover = analyzer.hover_at(loc).ok_or("no hover info for END block")?;

    assert!(
        hover.signature.to_lowercase().contains("end")
            || hover.signature.to_lowercase().contains("shutdown")
            || hover.signature.to_lowercase().contains("cleanup"),
        "END hover signature should mention 'end', 'shutdown', or 'cleanup', got: {}",
        hover.signature
    );
    Ok(())
}

#[test]
fn test_init_phase_block_hover_contains_post_compile() -> Result<(), Box<dyn std::error::Error>> {
    let code = "INIT { my $y = 2; }";
    let (ast, analyzer) = parse_and_analyze(code)?;

    let loc = find_phase_block_location(&ast).ok_or("no PhaseBlock node found")?;
    let hover = analyzer.hover_at(loc).ok_or("no hover info for INIT block")?;

    assert!(
        hover.signature.to_lowercase().contains("init")
            || hover.signature.to_lowercase().contains("post-compile")
            || hover.signature.to_lowercase().contains("runtime"),
        "INIT hover signature should mention 'init', 'post-compile', or 'runtime', got: {}",
        hover.signature
    );
    Ok(())
}

#[test]
fn test_check_phase_block_hover_present() -> Result<(), Box<dyn std::error::Error>> {
    let code = "CHECK { my $z = 3; }";
    let (ast, analyzer) = parse_and_analyze(code)?;

    let loc = find_phase_block_location(&ast).ok_or("no PhaseBlock node found")?;
    let hover = analyzer.hover_at(loc).ok_or("no hover info for CHECK block")?;

    assert!(
        hover.signature.to_lowercase().contains("check"),
        "CHECK hover signature should mention 'check', got: {}",
        hover.signature
    );
    Ok(())
}

#[test]
fn test_unitcheck_phase_block_hover_present() -> Result<(), Box<dyn std::error::Error>> {
    let code = "UNITCHECK { my $u = 4; }";
    let (ast, analyzer) = parse_and_analyze(code)?;

    let loc = find_phase_block_location(&ast).ok_or("no PhaseBlock node found")?;
    let hover = analyzer.hover_at(loc).ok_or("no hover info for UNITCHECK block")?;

    assert!(
        hover.signature.to_lowercase().contains("unitcheck")
            || hover.signature.to_lowercase().contains("unit"),
        "UNITCHECK hover signature should mention 'unitcheck' or 'unit', got: {}",
        hover.signature
    );
    Ok(())
}

#[test]
fn test_phase_block_hover_has_documentation() -> Result<(), Box<dyn std::error::Error>> {
    // All phase blocks should provide documentation explaining execution semantics
    let cases = [
        ("BEGIN { 1 }", "BEGIN"),
        ("END { 1 }", "END"),
        ("INIT { 1 }", "INIT"),
        ("CHECK { 1 }", "CHECK"),
        ("UNITCHECK { 1 }", "UNITCHECK"),
    ];

    for (code, phase) in cases {
        let (ast, analyzer) = parse_and_analyze(code)?;
        let loc = find_phase_block_location(&ast)
            .ok_or_else(|| format!("no PhaseBlock node found for {phase}"))?;
        let hover =
            analyzer.hover_at(loc).ok_or_else(|| format!("no hover info for {phase} block"))?;

        assert!(
            hover.documentation.is_some(),
            "{phase} hover should include documentation describing execution semantics"
        );
    }
    Ok(())
}
