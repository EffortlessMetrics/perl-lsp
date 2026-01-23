//! Tests for name_span implementation in special blocks and phase blocks
//!
//! This test suite validates that AUTOLOAD, DESTROY, and phase blocks
//! (BEGIN, END, CHECK, INIT, UNITCHECK) have proper name_span/phase_span
//! for precise LSP navigation (go-to-definition, rename, etc.)

use perl_parser::Parser;
use perl_parser::ast::{Node, NodeKind};

/// Helper function to find the first Subroutine node in the AST
fn find_subroutine(node: &Node) -> Option<&Node> {
    match &node.kind {
        NodeKind::Subroutine { .. } => Some(node),
        NodeKind::Program { statements } => {
            for stmt in statements {
                if let Some(sub_node) = find_subroutine(stmt) {
                    return Some(sub_node);
                }
            }
            None
        }
        NodeKind::ExpressionStatement { expression } => find_subroutine(expression),
        _ => None,
    }
}

/// Helper function to find the first PhaseBlock node in the AST
fn find_phase_block(node: &Node) -> Option<&Node> {
    match &node.kind {
        NodeKind::PhaseBlock { .. } => Some(node),
        NodeKind::Program { statements } => {
            for stmt in statements {
                if let Some(phase_node) = find_phase_block(stmt) {
                    return Some(phase_node);
                }
            }
            None
        }
        NodeKind::ExpressionStatement { expression } => find_phase_block(expression),
        _ => None,
    }
}

#[test]
fn test_autoload_name_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub AUTOLOAD { print 'auto'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let sub_node = find_subroutine(&ast).ok_or("No Subroutine node found")?;

    if let NodeKind::Subroutine { name, name_span, .. } = &sub_node.kind {
        assert_eq!(name, &Some("AUTOLOAD".to_string()), "Subroutine name should be AUTOLOAD");

        let span = name_span.ok_or("name_span should be Some for AUTOLOAD")?;

        // Verify the span points to "AUTOLOAD" in the source
        // "sub AUTOLOAD" - AUTOLOAD starts at position 4 and ends at 12
        assert_eq!(span.start, 4, "AUTOLOAD name_span should start at position 4");
        assert_eq!(span.end, 12, "AUTOLOAD name_span should end at position 12");

        // Verify we can extract the name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "AUTOLOAD", "Extracted text should match AUTOLOAD");
    } else {
        return Err(format!("Expected Subroutine node, got {:?}", sub_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_destroy_name_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub DESTROY { print 'destroy'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let sub_node = find_subroutine(&ast).ok_or("No Subroutine node found")?;

    if let NodeKind::Subroutine { name, name_span, .. } = &sub_node.kind {
        assert_eq!(name, &Some("DESTROY".to_string()), "Subroutine name should be DESTROY");

        let span = name_span.ok_or("name_span should be Some for DESTROY")?;

        // Verify the span points to "DESTROY" in the source
        // "sub DESTROY" - DESTROY starts at position 4 and ends at 11
        assert_eq!(span.start, 4, "DESTROY name_span should start at position 4");
        assert_eq!(span.end, 11, "DESTROY name_span should end at position 11");

        // Verify we can extract the name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "DESTROY", "Extracted text should match DESTROY");
    } else {
        return Err(format!("Expected Subroutine node, got {:?}", sub_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_begin_phase_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { print 'starting'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
        assert_eq!(phase, "BEGIN", "Phase should be BEGIN");

        let span = phase_span.ok_or("phase_span should be Some for BEGIN")?;

        // Verify the span points to "BEGIN" in the source
        // "BEGIN {" - BEGIN starts at position 0 and ends at 5
        assert_eq!(span.start, 0, "BEGIN phase_span should start at position 0");
        assert_eq!(span.end, 5, "BEGIN phase_span should end at position 5");

        // Verify we can extract the phase name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "BEGIN", "Extracted text should match BEGIN");
    } else {
        return Err(format!("Expected PhaseBlock node, got {:?}", phase_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_end_phase_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "END { print 'cleanup'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
        assert_eq!(phase, "END", "Phase should be END");

        let span = phase_span.ok_or("phase_span should be Some for END")?;

        // Verify the span points to "END" in the source
        // "END {" - END starts at position 0 and ends at 3
        assert_eq!(span.start, 0, "END phase_span should start at position 0");
        assert_eq!(span.end, 3, "END phase_span should end at position 3");

        // Verify we can extract the phase name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "END", "Extracted text should match END");
    } else {
        return Err(format!("Expected PhaseBlock node, got {:?}", phase_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_check_phase_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "CHECK { print 'checking'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
        assert_eq!(phase, "CHECK", "Phase should be CHECK");

        let span = phase_span.ok_or("phase_span should be Some for CHECK")?;

        // Verify the span points to "CHECK" in the source
        assert_eq!(span.start, 0, "CHECK phase_span should start at position 0");
        assert_eq!(span.end, 5, "CHECK phase_span should end at position 5");

        // Verify we can extract the phase name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "CHECK", "Extracted text should match CHECK");
    } else {
        return Err(format!("Expected PhaseBlock node, got {:?}", phase_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_init_phase_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "INIT { print 'initializing'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
        assert_eq!(phase, "INIT", "Phase should be INIT");

        let span = phase_span.ok_or("phase_span should be Some for INIT")?;

        // Verify the span points to "INIT" in the source
        assert_eq!(span.start, 0, "INIT phase_span should start at position 0");
        assert_eq!(span.end, 4, "INIT phase_span should end at position 4");

        // Verify we can extract the phase name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "INIT", "Extracted text should match INIT");
    } else {
        return Err(format!("Expected PhaseBlock node, got {:?}", phase_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_unitcheck_phase_span() -> Result<(), Box<dyn std::error::Error>> {
    let code = "UNITCHECK { print 'unit checking'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
        assert_eq!(phase, "UNITCHECK", "Phase should be UNITCHECK");

        let span = phase_span.ok_or("phase_span should be Some for UNITCHECK")?;

        // Verify the span points to "UNITCHECK" in the source
        assert_eq!(span.start, 0, "UNITCHECK phase_span should start at position 0");
        assert_eq!(span.end, 9, "UNITCHECK phase_span should end at position 9");

        // Verify we can extract the phase name from source using the span
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "UNITCHECK", "Extracted text should match UNITCHECK");
    } else {
        return Err(format!("Expected PhaseBlock node, got {:?}", phase_node.kind).into());
    }
    Ok(())
}

#[test]
fn test_multiple_phase_blocks_with_spans() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
BEGIN { print 'start'; }
END { print 'end'; }
CHECK { print 'check'; }
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Collect all phase blocks
    let mut phase_blocks = Vec::new();
    if let NodeKind::Program { statements } = &ast.kind {
        for stmt in statements {
            if let NodeKind::PhaseBlock { .. } = &stmt.kind {
                phase_blocks.push(stmt);
            }
        }
    }

    assert_eq!(phase_blocks.len(), 3, "Should find 3 phase blocks");

    // Verify each phase block has proper span
    for phase_node in phase_blocks {
        if let NodeKind::PhaseBlock { phase, phase_span, .. } = &phase_node.kind {
            let span = phase_span.ok_or("phase_span should be Some")?;
            let extracted = &code[span.start..span.end];
            assert_eq!(extracted, phase, "Extracted text should match phase name");
        }
    }
    Ok(())
}

#[test]
fn test_autoload_and_destroy_both_have_spans() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub AUTOLOAD { print 'auto'; }
sub DESTROY { print 'destroy'; }
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Collect all subroutines
    let mut subroutines = Vec::new();
    if let NodeKind::Program { statements } = &ast.kind {
        for stmt in statements {
            if let Some(sub_node) = find_subroutine(stmt) {
                subroutines.push(sub_node);
            }
        }
    }

    assert_eq!(subroutines.len(), 2, "Should find 2 subroutines");

    // Verify both have proper name_span
    for sub_node in subroutines {
        if let NodeKind::Subroutine { name, name_span, .. } = &sub_node.kind {
            let span = name_span.ok_or("name_span should be Some")?;
            let extracted = &code[span.start..span.end];
            assert_eq!(
                extracted,
                name.as_ref().ok_or("name should be Some")?,
                "Extracted text should match subroutine name"
            );
        }
    }
    Ok(())
}

#[test]
fn test_name_span_not_entire_block() -> Result<(), Box<dyn std::error::Error>> {
    // This test ensures name_span is just the identifier, not the entire block
    let code = "sub AUTOLOAD { my $x = 1; print $x; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let sub_node = find_subroutine(&ast).ok_or("No Subroutine node found")?;

    if let NodeKind::Subroutine { name_span, .. } = &sub_node.kind {
        let span = name_span.ok_or("name_span should be Some")?;

        // The span should be much smaller than the entire code
        let span_length = span.end - span.start;
        assert_eq!(
            span_length, 8,
            "name_span should be 8 characters (AUTOLOAD), not the entire block"
        );

        // Verify the extracted text is just the identifier
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "AUTOLOAD");
        assert!(!extracted.contains('{'), "name_span should not include block delimiter");
        assert!(!extracted.contains("print"), "name_span should not include block body");
    }
    Ok(())
}

#[test]
fn test_phase_span_not_entire_block() -> Result<(), Box<dyn std::error::Error>> {
    // This test ensures phase_span is just the phase keyword, not the entire block
    let code = "BEGIN { my $x = 1; print $x; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let phase_node = find_phase_block(&ast).ok_or("No PhaseBlock node found")?;

    if let NodeKind::PhaseBlock { phase_span, .. } = &phase_node.kind {
        let span = phase_span.ok_or("phase_span should be Some")?;

        // The span should be much smaller than the entire code
        let span_length = span.end - span.start;
        assert_eq!(
            span_length, 5,
            "phase_span should be 5 characters (BEGIN), not the entire block"
        );

        // Verify the extracted text is just the keyword
        let extracted = &code[span.start..span.end];
        assert_eq!(extracted, "BEGIN");
        assert!(!extracted.contains('{'), "phase_span should not include block delimiter");
        assert!(!extracted.contains("print"), "phase_span should not include block body");
    }
    Ok(())
}
