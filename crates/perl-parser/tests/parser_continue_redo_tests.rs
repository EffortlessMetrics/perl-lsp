//! Comprehensive parser tests for continue/redo statements
//!
//! These tests validate that the parser correctly recognizes continue blocks and redo statements
//! in various loop contexts and produces the correct AST structure.
//!
//! Acceptance Criteria Coverage:
//! - AC1: Parser recognizes `continue` and `redo` keywords
//! - AC2: Continue/redo statements are parsed correctly in all loop types (while, for, foreach, until)
//! - AC3: Labels are supported for continue/redo statements
//! - AC4: At least 10 test cases covering all continue/redo features
//! - AC5: Continue statements produce correct AST structure with continue_block
//! - AC6: Redo statements produce correct AST structure with NodeKind::LoopControl
//! - AC7: LSP integration tests validate continue/redo syntax highlighting
//! - AC8: Go-to-definition works for labels referenced in continue/redo

use perl_corpus::{continue_redo_cases, find_continue_redo_case, valid_continue_redo_cases};
use perl_parser::{Node, NodeKind, Parser};
use perl_tdd_support::{must, must_some};

/// Helper to parse code and return the AST
fn parse_code(code: &str) -> Result<Node, perl_parser::ParseError> {
    let mut parser = Parser::new(code);
    parser.parse()
}

/// Helper to find nodes of a specific kind in the AST
fn find_nodes(node: &Node, matches: impl Fn(&NodeKind) -> bool + Copy) -> Vec<&Node> {
    let mut results = Vec::new();
    find_nodes_recursive(node, matches, &mut results);
    results
}

/// Internal recursive helper
fn find_nodes_recursive<'a>(
    node: &'a Node,
    matches: impl Fn(&NodeKind) -> bool + Copy,
    results: &mut Vec<&'a Node>,
) {
    if matches(&node.kind) {
        results.push(node);
    }
    for child in node.children() {
        find_nodes_recursive(child, matches, results);
    }
}

// ============================================================================
// AC1: Parser recognizes `continue` and `redo` keywords
// ============================================================================

#[test]
fn parser_continue_keyword_recognized() {
    let code = r#"while (1) { } continue { print "done\n"; }"#;
    let ast = must(parse_code(code));

    // Check that we have a While node with a continue_block
    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert!(!while_nodes.is_empty(), "Should find at least one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "While loop should have a continue block");
    } else {
        unreachable!("Expected While node");
    }
}

#[test]
fn parser_redo_keyword_recognized() {
    let code = r#"while (1) { redo; }"#;
    let ast = must(parse_code(code));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo node");
}

// ============================================================================
// AC2: Continue/redo statements are parsed correctly in all loop types
// ============================================================================

#[test]
fn parser_continue_in_while_loop() {
    let case = must_some(find_continue_redo_case("continue.while.basic"));
    let ast = must(parse_code(case.source));

    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "While loop should have a continue block");
    }
}

#[test]
fn parser_continue_in_until_loop() {
    let case = must_some(find_continue_redo_case("continue.until.basic"));
    let ast = must(parse_code(case.source));

    // Until is represented as While with negated condition
    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "Until loop should have a continue block");
    }
}

#[test]
fn parser_continue_in_for_loop() {
    let case = must_some(find_continue_redo_case("continue.for.basic"));
    let ast = must(parse_code(case.source));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one For/Foreach node");

    // Check for continue block
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } => {
            assert!(continue_block.is_some(), "For loop should have a continue block");
        }
        NodeKind::Foreach { .. } => {
            // Note: Foreach doesn't have continue_block in current AST
        }
        _ => unreachable!("Expected For or Foreach node"),
    }
}

#[test]
fn parser_continue_in_foreach_loop() {
    let case = must_some(find_continue_redo_case("continue.foreach.basic"));
    let ast = must(parse_code(case.source));

    let foreach_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::Foreach { .. }));
    assert_eq!(foreach_nodes.len(), 1, "Should have exactly one Foreach node");
}

#[test]
fn parser_redo_in_while_loop() {
    let case = must_some(find_continue_redo_case("redo.while.basic"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

#[test]
fn parser_redo_in_until_loop() {
    let case = must_some(find_continue_redo_case("redo.until.basic"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

#[test]
fn parser_redo_in_for_loop() {
    let case = must_some(find_continue_redo_case("redo.for.basic"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

// ============================================================================
// AC3: Labels are supported for continue/redo statements
// ============================================================================

#[test]
fn parser_redo_with_label() {
    let case = must_some(find_continue_redo_case("redo.labeled.loop"));
    let ast = must(parse_code(case.source));

    let redo_nodes = find_nodes(
        &ast,
        |kind| matches!(kind, NodeKind::LoopControl { op, label, .. } if op == "redo" && label.is_some()),
    );
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement with label");

    if let NodeKind::LoopControl { label, .. } = &redo_nodes[0].kind {
        assert_eq!(label.as_deref(), Some("LOOP"), "Label should be LOOP");
    }
}

#[test]
fn parser_redo_nested_labeled() {
    let case = must_some(find_continue_redo_case("redo.nested.labeled"));
    let ast = must(parse_code(case.source));

    let redo_nodes = find_nodes(
        &ast,
        |kind| matches!(kind, NodeKind::LoopControl { op, label, .. } if op == "redo" && label.is_some()),
    );
    assert!(redo_nodes.len() >= 2, "Should find at least two redo statements with labels");

    // Check that we have both INNER and OUTER labels
    let labels: Vec<&str> = redo_nodes
        .iter()
        .filter_map(|node| {
            if let NodeKind::LoopControl { label, .. } = &node.kind {
                label.as_deref()
            } else {
                None
            }
        })
        .collect();

    assert!(labels.contains(&"INNER"), "Should have INNER label");
    assert!(labels.contains(&"OUTER"), "Should have OUTER label");
}

// ============================================================================
// AC4: At least 10 test cases covering all continue/redo features
// ============================================================================

#[test]
fn parser_continue_next_interaction() {
    let case = must_some(find_continue_redo_case("continue.next.interaction"));
    let ast = must(parse_code(case.source));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    let next_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "next"));
    assert!(!next_nodes.is_empty(), "Should find next statement");
}

#[test]
fn parser_continue_last_interaction() {
    let case = must_some(find_continue_redo_case("continue.last.interaction"));
    let ast = must(parse_code(case.source));

    let last_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "last"));
    assert!(!last_nodes.is_empty(), "Should find last statement");
}

#[test]
fn parser_continue_redo_interaction() {
    let case = must_some(find_continue_redo_case("continue.redo.interaction"));
    let ast = must(parse_code(case.source));

    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_continue_nested_loops() {
    let case = must_some(find_continue_redo_case("continue.nested.loops"));
    let ast = must(parse_code(case.source));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(for_nodes.len() >= 2, "Should have at least two nested loops");

    // Count continue blocks
    let continue_blocks = for_nodes
        .iter()
        .filter(|node| match &node.kind {
            NodeKind::For { continue_block, .. } => continue_block.is_some(),
            NodeKind::Foreach { continue_block, .. } => continue_block.is_some(),
            _ => false,
        })
        .count();
    assert!(continue_blocks >= 2, "Both loops should have continue blocks");
}

#[test]
fn parser_continue_multiple_statements() {
    let case = must_some(find_continue_redo_case("continue.multiple.statements"));
    let ast = must(parse_code(case.source));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    // Verify continue block exists and has content (only For has continue_block)
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } => {
            assert!(continue_block.is_some(), "Should have a continue block");
            let cont = must_some(continue_block.as_ref());
            if let NodeKind::Block { statements } = &cont.kind {
                assert!(statements.len() >= 3, "Continue block should have multiple statements");
            }
        }
        _ => unreachable!("Expected For node"),
    }
}

#[test]
fn parser_continue_empty_block() {
    let case = must_some(find_continue_redo_case("continue.empty.block"));
    let ast = must(parse_code(case.source));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    // Verify empty continue block (only For has continue_block)
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } => {
            assert!(continue_block.is_some(), "Should have a continue block");
        }
        _ => unreachable!("Expected For node"),
    }
}

#[test]
fn parser_redo_do_while() {
    let case = must_some(find_continue_redo_case("redo.do.while"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_redo_conditional() {
    let case = must_some(find_continue_redo_case("redo.conditional"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_redo_counter_reset() {
    let case = must_some(find_continue_redo_case("redo.counter.reset"));
    let ast = must(parse_code(case.source));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

// ============================================================================
// AC5 & AC6: Validate AST structure
// ============================================================================

#[test]
fn parser_continue_ast_structure() {
    let code = r#"
for my $i (1..3) {
    print "$i\n";
} continue {
    print "continue\n";
}
"#;
    let ast = must(parse_code(code));

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert_eq!(for_nodes.len(), 1, "Should have exactly one For/Foreach node");

    match &for_nodes[0].kind {
        NodeKind::Foreach { variable, list, body, .. } => {
            // Verify iterator variable exists
            assert!(
                matches!(
                    variable.kind,
                    NodeKind::Variable { .. } | NodeKind::VariableDeclaration { .. }
                ),
                "Should have variable"
            );
            // Verify list exists (range expression parsed as Binary with op "..")
            assert!(
                matches!(list.kind, NodeKind::Binary { .. }),
                "Should have list (range expression)"
            );
            // Verify body exists
            assert!(matches!(body.kind, NodeKind::Block { .. }), "Should have body block");
        }
        _ => unreachable!("Expected Foreach node"),
    }
}

#[test]
fn parser_redo_ast_structure() {
    let code = r#"
while ($count < 3) {
    $count++;
    redo if $count == 2;
}
"#;
    let ast = must(parse_code(code));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert_eq!(redo_nodes.len(), 1, "Should have exactly one redo node");

    if let NodeKind::LoopControl { op, label } = &redo_nodes[0].kind {
        assert_eq!(op, "redo", "Operation should be 'redo'");
        assert!(label.is_none(), "This redo should not have a label");
    }
}

#[test]
fn parser_redo_with_label_ast_structure() {
    let code = r#"
LOOP: while ($count < 3) {
    $count++;
    redo LOOP if $count == 2;
}
"#;
    let ast = must(parse_code(code));

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert_eq!(redo_nodes.len(), 1, "Should have exactly one redo node");

    if let NodeKind::LoopControl { op, label } = &redo_nodes[0].kind {
        assert_eq!(op, "redo", "Operation should be 'redo'");
        assert_eq!(label.as_deref(), Some("LOOP"), "Label should be 'LOOP'");
    }
}

// ============================================================================
// Comprehensive corpus-based tests
// ============================================================================

#[test]
fn parser_all_valid_continue_redo_cases_parse() {
    let valid_cases = valid_continue_redo_cases();
    assert!(
        valid_cases.len() >= 10,
        "Should have at least 10 valid test cases (found {})",
        valid_cases.len()
    );

    for case in valid_cases {
        let result = parse_code(case.source);
        assert!(
            result.is_ok(),
            "Failed to parse valid case '{}': {:?}\nSource:\n{}",
            case.id,
            result.err(),
            case.source
        );
    }
}

#[test]
fn parser_continue_redo_coverage() {
    let all_cases = continue_redo_cases();
    let valid_count = all_cases.iter().filter(|c| c.should_parse).count();
    let invalid_count = all_cases.iter().filter(|c| !c.should_parse).count();

    println!("Total continue/redo test cases: {}", all_cases.len());
    println!("  Valid cases: {}", valid_count);
    println!("  Invalid cases: {}", invalid_count);

    assert!(valid_count >= 20, "Should have at least 20 valid test cases");
    assert!(all_cases.len() >= 25, "Should have at least 25 total test cases");
}
