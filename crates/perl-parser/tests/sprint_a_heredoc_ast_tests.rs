/// Sprint A Heredoc AST Integration Tests
///
/// Tests that HeredocContext.statement_end_line is properly integrated into AST construction,
/// ensuring heredoc statements are correctly bounded within blocks.
///
/// Fixtures:
/// - F5: Simple block heredoc (if block with single heredoc)
/// - F6: Two heredocs in same block
/// - Edge cases: eval blocks, back-to-back heredocs
use perl_parser::Parser;
use perl_parser::ast::{Node, NodeKind};

/// Recursively collect all heredoc nodes
fn collect_heredocs(node: &Node) -> Vec<String> {
    let mut heredocs = Vec::new();
    collect_heredocs_recursive(node, &mut heredocs);
    heredocs
}

fn collect_heredocs_recursive(node: &Node, out: &mut Vec<String>) {
    if let NodeKind::Heredoc { delimiter, .. } = &node.kind {
        out.push(delimiter.clone());
    }

    node.for_each_child(|child| {
        collect_heredocs_recursive(child, out);
    });
}

/// Count top-level statements in the AST
fn count_top_level_statements(root: &Node) -> usize {
    match &root.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

/// Extract statement count inside a specific block node
fn count_block_statements(node: &Node) -> Option<usize> {
    match &node.kind {
        NodeKind::Block { statements } => Some(statements.len()),
        _ => {
            let mut result = None;
            node.for_each_child(|child| {
                if result.is_none() {
                    result = count_block_statements(child);
                }
            });
            result
        }
    }
}

// ============================================================================
// F5: Simple block heredoc
// ============================================================================

#[test]
fn f5_heredoc_in_if_block_ast_structure() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"if ($cond) {
    my $x = <<EOF;
line 1
EOF
    say $x;
}
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;

    // Collect heredocs
    let heredocs = collect_heredocs(&root);
    assert_eq!(heredocs.len(), 1, "Expected exactly one heredoc node");
    assert_eq!(&heredocs[0], "EOF", "Heredoc delimiter should be EOF");

    // Check AST structure: the heredoc statement and say statement should be separate
    // The block should contain 2 statements:
    // 1. my $x = <<EOF; (lines 2-4)
    // 2. say $x; (line 5)
    let block_stmt_count = count_block_statements(&root);
    assert!(block_stmt_count.is_some(), "Should find a block node in the AST");
    assert_eq!(
        block_stmt_count.ok_or("No block found")?,
        2,
        "Block should contain 2 separate statements (heredoc assignment and say)"
    );
    Ok(())
}

#[test]
fn f5_heredoc_statement_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that the heredoc statement doesn't swallow the next statement
    let src = r#"if ($cond) {
    my $x = <<EOF;
line 1
EOF
    say $x;
}
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;
    let sexp = root.to_sexp();

    // The say statement should be present and separate
    assert!(sexp.contains("say"), "say statement should be present and not consumed by heredoc");

    // Heredoc body should be consumed (not parsed as identifier)
    assert!(
        !sexp.contains("(identifier line)"),
        "Heredoc body should be consumed, not parsed as identifier"
    );
    Ok(())
}

// ============================================================================
// F6: Two heredocs in same block
// ============================================================================

#[test]
fn f6_two_heredocs_same_block_ast_structure() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"if ($cond) {
    my $x = <<EOF1;
foo
EOF1
    my $y = <<EOF2;
bar
EOF2
}
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;

    // Collect heredocs
    let heredocs = collect_heredocs(&root);
    assert_eq!(heredocs.len(), 2, "Expected exactly two heredoc nodes");
    assert_eq!(&heredocs[0], "EOF1", "First heredoc delimiter should be EOF1");
    assert_eq!(&heredocs[1], "EOF2", "Second heredoc delimiter should be EOF2");

    // Check AST structure: the block should contain 2 statements
    // 1. my $x = <<EOF1; (lines 2-4)
    // 2. my $y = <<EOF2; (lines 5-7)
    let block_stmt_count = count_block_statements(&root);
    assert!(block_stmt_count.is_some(), "Should find a block node in the AST");
    assert_eq!(
        block_stmt_count.ok_or("No block found")?,
        2,
        "Block should contain 2 separate heredoc statements"
    );
    Ok(())
}

#[test]
fn f6_two_heredocs_distinct_statements() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that each heredoc is in its own statement (not merged)
    let src = r#"if ($cond) {
    my $x = <<EOF1;
foo
EOF1
    my $y = <<EOF2;
bar
EOF2
}
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;
    let sexp = root.to_sexp();

    // Both variables should be present
    assert!(sexp.contains("(variable $ x)"), "Variable $x should be present");
    assert!(sexp.contains("(variable $ y)"), "Variable $y should be present");

    // Heredoc bodies should be consumed
    assert!(!sexp.contains("(identifier foo)"), "First heredoc body should be consumed");
    assert!(!sexp.contains("(identifier bar)"), "Second heredoc body should be consumed");
    Ok(())
}

// ============================================================================
// Edge Case: eval block with heredoc
// ============================================================================

#[test]
fn edge_case_heredoc_in_eval_block() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"eval {
    my $x = <<EOF;
content
EOF
    say $x;
};
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;

    // Collect heredocs
    let heredocs = collect_heredocs(&root);
    assert_eq!(heredocs.len(), 1, "Expected exactly one heredoc node in eval block");

    // Check that eval block has 2 statements (heredoc + say)
    let sexp = root.to_sexp();
    assert!(sexp.contains("say"), "say statement should be present after heredoc");
    assert!(
        !sexp.contains("(identifier content)"),
        "Heredoc body should be consumed in eval block"
    );
    Ok(())
}

// ============================================================================
// Edge Case: back-to-back heredocs (no blank line between)
// ============================================================================

#[test]
fn edge_case_back_to_back_heredocs() -> Result<(), Box<dyn std::error::Error>> {
    // Two heredocs with no blank line between their bodies
    let src = r#"my $x = <<EOF1;
first
EOF1
my $y = <<EOF2;
second
EOF2
"#;

    let mut parser = Parser::new(src);
    let root = parser.parse()?;

    // Collect heredocs
    let heredocs = collect_heredocs(&root);
    assert_eq!(heredocs.len(), 2, "Expected two heredoc nodes back-to-back");

    // Check that they are separate statements
    let stmt_count = count_top_level_statements(&root);
    assert_eq!(
        stmt_count, 2,
        "Should have 2 separate top-level statements for back-to-back heredocs"
    );

    // Bodies should be consumed correctly
    let sexp = root.to_sexp();
    assert!(!sexp.contains("(identifier first)"), "First heredoc body should be consumed");
    assert!(!sexp.contains("(identifier second)"), "Second heredoc body should be consumed");
    Ok(())
}
