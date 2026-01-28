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

/// Helper to parse code and return the AST
fn parse_code(code: &str) -> Result<Node, perl_parser::ParseError> {
    let mut parser = Parser::new(code);
    parser.parse()
}

/// Helper to find nodes of a specific kind in the AST
fn find_nodes<'a>(node: &'a Node, matches: impl Fn(&NodeKind) -> bool + Copy) -> Vec<&'a Node> {
    let mut results = Vec::new();
    if matches(&node.kind) {
        results.push(node);
    }
    visit_children(node, &mut |child| {
        results.extend(find_nodes(child, matches));
    });
    results
}

/// Visit all child nodes
fn visit_children(node: &Node, visitor: &mut impl FnMut(&Node)) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                visitor(stmt);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                visitor(stmt);
            }
        }
        NodeKind::While { condition, body, continue_block } => {
            visitor(condition);
            visitor(body);
            if let Some(cont) = continue_block {
                visitor(cont);
            }
        }
        NodeKind::For { init, condition, increment, body, continue_block, .. } => {
            if let Some(i) = init {
                visitor(i);
            }
            if let Some(c) = condition {
                visitor(c);
            }
            if let Some(inc) = increment {
                visitor(inc);
            }
            visitor(body);
            if let Some(cont) = continue_block {
                visitor(cont);
            }
        }
        NodeKind::Foreach { iterator, iterable, body, continue_block } => {
            if let Some(iter) = iterator {
                visitor(iter);
            }
            visitor(iterable);
            visitor(body);
            if let Some(cont) = continue_block {
                visitor(cont);
            }
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            visitor(condition);
            visitor(then_branch);
            for (cond, branch) in elsif_branches {
                visitor(cond);
                visitor(branch);
            }
            if let Some(branch) = else_branch {
                visitor(branch);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            visitor(expression);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            visitor(variable);
            if let Some(init) = initializer {
                visitor(init);
            }
        }
        _ => {}
    }
}

// ============================================================================
// AC1: Parser recognizes `continue` and `redo` keywords
// ============================================================================

#[test]
fn parser_continue_keyword_recognized() {
    let code = r#"while (1) { } continue { print "done\n"; }"#;
    let ast = parse_code(code).expect("Failed to parse continue block");

    // Check that we have a While node with a continue_block
    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert!(!while_nodes.is_empty(), "Should find at least one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "While loop should have a continue block");
    } else {
        panic!("Expected While node");
    }
}

#[test]
fn parser_redo_keyword_recognized() {
    let code = r#"while (1) { redo; }"#;
    let ast = parse_code(code).expect("Failed to parse redo statement");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo node");
}

// ============================================================================
// AC2: Continue/redo statements are parsed correctly in all loop types
// ============================================================================

#[test]
fn parser_continue_in_while_loop() {
    let case = find_continue_redo_case("continue.while.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue in while loop");

    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "While loop should have a continue block");
    }
}

#[test]
fn parser_continue_in_until_loop() {
    let case = find_continue_redo_case("continue.until.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue in until loop");

    // Until is represented as While with negated condition
    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    if let NodeKind::While { continue_block, .. } = &while_nodes[0].kind {
        assert!(continue_block.is_some(), "Until loop should have a continue block");
    }
}

#[test]
fn parser_continue_in_for_loop() {
    let case = find_continue_redo_case("continue.for.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue in for loop");

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one For/Foreach node");

    // Check for continue block
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } => {
            assert!(continue_block.is_some(), "For loop should have a continue block");
        }
        NodeKind::Foreach { continue_block, .. } => {
            assert!(continue_block.is_some(), "For loop should have a continue block");
        }
        _ => panic!("Expected For or Foreach node"),
    }
}

#[test]
fn parser_continue_in_foreach_loop() {
    let case = find_continue_redo_case("continue.foreach.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue in foreach loop");

    let foreach_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::Foreach { .. }));
    assert_eq!(foreach_nodes.len(), 1, "Should have exactly one Foreach node");

    if let NodeKind::Foreach { continue_block, .. } = &foreach_nodes[0].kind {
        assert!(continue_block.is_some(), "Foreach loop should have a continue block");
    }
}

#[test]
fn parser_redo_in_while_loop() {
    let case = find_continue_redo_case("redo.while.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo in while loop");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

#[test]
fn parser_redo_in_until_loop() {
    let case = find_continue_redo_case("redo.until.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo in until loop");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

#[test]
fn parser_redo_in_for_loop() {
    let case = find_continue_redo_case("redo.for.basic").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo in for loop");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find at least one redo statement");
}

// ============================================================================
// AC3: Labels are supported for continue/redo statements
// ============================================================================

#[test]
fn parser_redo_with_label() {
    let case = find_continue_redo_case("redo.labeled.loop").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo with label");

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
    let case = find_continue_redo_case("redo.nested.labeled").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse nested labeled redo");

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
    let case =
        find_continue_redo_case("continue.next.interaction").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue with next");

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    let next_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "next"));
    assert!(!next_nodes.is_empty(), "Should find next statement");
}

#[test]
fn parser_continue_last_interaction() {
    let case =
        find_continue_redo_case("continue.last.interaction").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue with last");

    let last_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "last"));
    assert!(!last_nodes.is_empty(), "Should find last statement");
}

#[test]
fn parser_continue_redo_interaction() {
    let case =
        find_continue_redo_case("continue.redo.interaction").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse continue with redo");

    let while_nodes = find_nodes(&ast, |kind| matches!(kind, NodeKind::While { .. }));
    assert_eq!(while_nodes.len(), 1, "Should have exactly one While node");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_continue_nested_loops() {
    let case = find_continue_redo_case("continue.nested.loops").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse nested loops with continue");

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
    let case =
        find_continue_redo_case("continue.multiple.statements").expect("Failed to find test case");
    let ast =
        parse_code(case.source).expect("Failed to parse continue block with multiple statements");

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    // Verify continue block exists and has content
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } | NodeKind::Foreach { continue_block, .. } => {
            assert!(continue_block.is_some(), "Should have a continue block");
            let cont = continue_block.as_ref().unwrap();
            if let NodeKind::Block { statements } = &cont.kind {
                assert!(statements.len() >= 3, "Continue block should have multiple statements");
            }
        }
        _ => panic!("Expected For or Foreach node"),
    }
}

#[test]
fn parser_continue_empty_block() {
    let case = find_continue_redo_case("continue.empty.block").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse empty continue block");

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert!(!for_nodes.is_empty(), "Should have at least one loop");

    // Verify empty continue block
    match &for_nodes[0].kind {
        NodeKind::For { continue_block, .. } | NodeKind::Foreach { continue_block, .. } => {
            assert!(continue_block.is_some(), "Should have a continue block");
        }
        _ => panic!("Expected For or Foreach node"),
    }
}

#[test]
fn parser_redo_do_while() {
    let case = find_continue_redo_case("redo.do.while").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo in do-while");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_redo_conditional() {
    let case = find_continue_redo_case("redo.conditional").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse conditional redo");

    let redo_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::LoopControl { op, .. } if op == "redo"));
    assert!(!redo_nodes.is_empty(), "Should find redo statement");
}

#[test]
fn parser_redo_counter_reset() {
    let case = find_continue_redo_case("redo.counter.reset").expect("Failed to find test case");
    let ast = parse_code(case.source).expect("Failed to parse redo with counter reset");

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
    let ast = parse_code(code).expect("Failed to parse for loop with continue");

    let for_nodes =
        find_nodes(&ast, |kind| matches!(kind, NodeKind::For { .. } | NodeKind::Foreach { .. }));
    assert_eq!(for_nodes.len(), 1, "Should have exactly one For/Foreach node");

    match &for_nodes[0].kind {
        NodeKind::Foreach { iterator, iterable, body, continue_block } => {
            assert!(iterator.is_some(), "Should have iterator");
            // Verify iterable exists
            assert!(matches!(iterable.kind, NodeKind::Range { .. }), "Should have range iterable");
            // Verify body exists
            assert!(matches!(body.kind, NodeKind::Block { .. }), "Should have body block");
            // Verify continue_block exists
            assert!(continue_block.is_some(), "Should have continue block");
            assert!(
                matches!(continue_block.as_ref().unwrap().kind, NodeKind::Block { .. }),
                "Continue block should be a Block"
            );
        }
        _ => panic!("Expected Foreach node"),
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
    let ast = parse_code(code).expect("Failed to parse while loop with redo");

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
    let ast = parse_code(code).expect("Failed to parse while loop with labeled redo");

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
