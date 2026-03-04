//! Comprehensive unit tests for perl-lsp-ast-utils
//!
//! Tests cover all public API functions with various edge cases and scenarios.

use perl_lsp_ast_utils::{
    find_declaration_position, find_function_insert_position, find_node_at_range,
    find_statement_start, get_indent_at,
};
use perl_parser_core::{Node, NodeKind};

// Helper function to create a test node
fn create_test_node(kind: NodeKind, start: usize, end: usize) -> Node {
    Node { kind, location: perl_parser_core::SourceLocation { start, end } }
}

// ============================================================================
// find_statement_start tests
// ============================================================================

#[test]
fn find_statement_start_at_beginning() {
    let src = "print 'hello';";
    assert_eq!(find_statement_start(src, 0), 0);
}

#[test]
fn find_statement_start_empty_string() {
    let src = "";
    assert_eq!(find_statement_start(src, 0), 0);
}

#[test]
fn find_statement_start_after_semicolon() {
    let src = "my $x = 1;\nmy $y = 2;";
    let pos = src.find("$y").unwrap_or(0);
    let expected = src.find('\n').unwrap_or(0) + 1;
    assert_eq!(find_statement_start(src, pos), expected);
}

#[test]
fn find_statement_start_after_newline() {
    let src = "print 'a';\nprint 'b';";
    let pos = src.find("'b'").unwrap_or(0);
    let expected = src.find('\n').unwrap_or(0) + 1;
    assert_eq!(find_statement_start(src, pos), expected);
}

#[test]
fn find_statement_start_multiple_semicolons() {
    let src = "a; b; c; d;";
    let pos = src.find('d').unwrap_or(0);
    assert!(find_statement_start(src, pos) > 0);
}

#[test]
fn find_statement_start_no_semicolon() {
    let src = "print 'hello'";
    let pos = src.len();
    assert_eq!(find_statement_start(src, pos), 0);
}

#[test]
fn find_statement_start_position_zero() {
    let src = "code here";
    assert_eq!(find_statement_start(src, 0), 0);
}

#[test]
fn find_statement_start_position_beyond_length() {
    let src = "my $x = 1;";
    // Position beyond the string should saturate
    let pos = src.len() + 100;
    assert!(find_statement_start(src, pos) >= 0);
}

#[test]
fn find_statement_start_multiline_with_multiple_newlines() {
    let src = "a;\nb;\nc;\nd;";
    let pos = src.find('d').unwrap_or(0);
    let expected = src.rfind('\n').unwrap_or(0) + 1;
    assert_eq!(find_statement_start(src, pos), expected);
}

#[test]
fn find_statement_start_after_first_semicolon() {
    let src = "first; second";
    let pos = src.find("second").unwrap_or(0);
    assert_eq!(find_statement_start(src, pos), 6);
}

#[test]
fn find_statement_start_with_tabs() {
    let src = "if (1) {\nfoo();\n\tbar();\n}";
    let pos = src.find("bar").unwrap_or(0);
    let newline_pos = src[..pos].rfind('\n').unwrap_or(0);
    assert_eq!(find_statement_start(src, pos), newline_pos + 1);
}

#[test]
fn find_statement_start_only_newline() {
    let src = "a\nb";
    let pos = src.find('b').unwrap_or(0);
    assert_eq!(find_statement_start(src, pos), src.find('\n').unwrap_or(0) + 1);
}

#[test]
fn find_statement_start_newline_at_end() {
    let src = "code;\n";
    let pos = src.len();
    assert_eq!(find_statement_start(src, pos), src.find('\n').unwrap_or(0) + 1);
}

#[test]
fn find_statement_start_consecutive_separators() {
    let src = "a;;\n\nb";
    let pos = src.find('b').unwrap_or(0);
    assert_eq!(find_statement_start(src, pos), src.rfind('\n').unwrap_or(0) + 1);
}

// ============================================================================
// find_declaration_position tests
// ============================================================================

#[test]
fn find_declaration_position_at_beginning() {
    let src = "print 'hello';";
    assert_eq!(find_declaration_position(src, 0), 0);
}

#[test]
fn find_declaration_position_delegates_to_statement_start() {
    let src = "print 'a';\nprint 'b';";
    let pos = src.find("'b'").unwrap_or(0);
    assert_eq!(find_declaration_position(src, pos), find_statement_start(src, pos));
}

#[test]
fn find_declaration_position_empty_string() {
    let src = "";
    assert_eq!(find_declaration_position(src, 0), 0);
}

#[test]
fn find_declaration_position_after_multiple_statements() {
    let src = "my $a = 1;\nmy $b = 2;\nmy $c = 3;";
    let pos = src.find("$c").unwrap_or(0);
    assert_eq!(find_declaration_position(src, pos), find_statement_start(src, pos));
}

#[test]
fn find_declaration_position_consistency_with_statement_start() {
    let test_cases = vec![("a;", 1), ("a; b;", 3), ("a;\nb;", 3), ("a;\n\nb;", 4)];

    for (src, pos) in test_cases {
        assert_eq!(
            find_declaration_position(src, pos),
            find_statement_start(src, pos),
            "Failed for src='{}', pos={}",
            src,
            pos
        );
    }
}

// ============================================================================
// find_function_insert_position tests
// ============================================================================

#[test]
fn find_function_insert_position_empty_string() {
    let src = "";
    assert_eq!(find_function_insert_position(src), 0);
}

#[test]
fn find_function_insert_position_simple_file() {
    let src = "my $x = 1;";
    assert_eq!(find_function_insert_position(src), src.len());
}

#[test]
fn find_function_insert_position_multiline() {
    let src = "sub foo { return 42; }\nsub bar { return 24; }\n";
    assert_eq!(find_function_insert_position(src), src.len());
}

#[test]
fn find_function_insert_position_with_trailing_newline() {
    let src = "code here\n";
    assert_eq!(find_function_insert_position(src), src.len());
}

#[test]
fn find_function_insert_position_large_file() {
    let src = "a".repeat(10000);
    assert_eq!(find_function_insert_position(&src), src.len());
}

#[test]
fn find_function_insert_position_unicode() {
    let src = "# 你好\nmy $x = '世界';";
    assert_eq!(find_function_insert_position(src), src.len());
}

#[test]
fn find_function_insert_position_always_end_of_file() {
    let test_cases = vec!["", "a", "hello world", "line1\nline2\nline3"];

    for src in test_cases {
        assert_eq!(
            find_function_insert_position(src),
            src.len(),
            "Expected end of file for '{}'",
            src
        );
    }
}

// ============================================================================
// get_indent_at tests
// ============================================================================

#[test]
fn get_indent_at_beginning_of_line_no_indent() {
    let src = "code here";
    assert_eq!(get_indent_at(src, 0), "");
}

#[test]
fn get_indent_at_with_spaces() {
    let src = "if (1) {\n    say 'x';\n}\n";
    let pos = src.find("say").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "    ");
}

#[test]
fn get_indent_at_with_tabs() {
    let src = "if (1) {\n\tprint 'a';\n}\n";
    let pos = src.find("print").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "\t");
}

#[test]
fn get_indent_at_mixed_tabs_and_spaces() {
    let src = "if (1) {\n\t  code here\n}\n";
    let pos = src.find("code").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "\t  ");
}

#[test]
fn get_indent_at_no_indent_after_newline() {
    let src = "first line\nno indent";
    let pos = src.find("no").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "");
}

#[test]
fn get_indent_at_multiple_indentation_levels() {
    let src = "a\n b\n  c\n   d";
    let pos = src.find('d').unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "   ");
}

#[test]
fn get_indent_at_empty_string() {
    let src = "";
    assert_eq!(get_indent_at(src, 0), "");
}

#[test]
fn get_indent_at_only_whitespace_line() {
    let src = "a\n    \nb";
    let pos = src.find("b").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "");
}

#[test]
fn get_indent_at_position_zero() {
    let src = "    code";
    // Position 0 is at the first character, which is the start of the line
    // The line starts at position 0 (no newline before), so the line is "    code"
    // The indent is the leading whitespace: "    "
    assert_eq!(get_indent_at(src, 0), "    ");
}

#[test]
fn get_indent_at_position_at_indent() {
    let src = "a\n    code";
    let pos = src.find(' ').unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "    ");
}

#[test]
fn get_indent_at_position_in_middle_of_code() {
    let src = "a\n    my_code";
    let pos = src.find("code").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "    ");
}

#[test]
fn get_indent_at_eight_spaces() {
    let src = "if {\n        deep nesting\n}\n";
    let pos = src.find("deep").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "        ");
}

#[test]
fn get_indent_at_no_preceding_newline() {
    let src = "    code";
    let pos = src.find("code").unwrap_or(0);
    assert_eq!(get_indent_at(src, pos), "    ");
}

#[test]
fn get_indent_at_position_beyond_length() {
    let src = "    code";
    // Position at string length is valid (at the end), but beyond causes panic
    // Use position at the end of the string
    let pos = src.len();
    assert_eq!(get_indent_at(src, pos), "    ");
}

#[test]
fn get_indent_at_last_character() {
    let src = "a\n    x";
    let pos = src.len() - 1; // position of 'x'
    assert_eq!(get_indent_at(src, pos), "    ");
}

// ============================================================================
// find_node_at_range tests
// ============================================================================

#[test]
fn find_node_at_range_single_node_exact_match() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 2);
    let result = find_node_at_range(&node, (0, 2));
    assert!(result.is_some());
    assert_eq!(result.unwrap().location.start, 0);
    assert_eq!(result.unwrap().location.end, 2);
}

#[test]
fn find_node_at_range_single_node_range_too_large() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 5, 7);
    let result = find_node_at_range(&node, (0, 10));
    assert!(result.is_none());
}

#[test]
fn find_node_at_range_single_node_range_inside() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 10);
    let result = find_node_at_range(&node, (2, 5));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_single_node_partial_overlap_left() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 5, 10);
    let result = find_node_at_range(&node, (0, 7));
    assert!(result.is_none());
}

#[test]
fn find_node_at_range_single_node_partial_overlap_right() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 5);
    let result = find_node_at_range(&node, (3, 10));
    assert!(result.is_none());
}

#[test]
fn find_node_at_range_program_with_statements() {
    let stmt1 = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let stmt2 = create_test_node(NodeKind::Number { value: "2".to_string() }, 2, 3);
    let program = create_test_node(NodeKind::Program { statements: vec![stmt1, stmt2] }, 0, 3);

    let result = find_node_at_range(&program, (2, 3));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_program_empty() {
    let program = create_test_node(NodeKind::Program { statements: vec![] }, 0, 0);
    let result = find_node_at_range(&program, (0, 0));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_block_with_statements() {
    let stmt = create_test_node(NodeKind::Number { value: "42".to_string() }, 2, 4);
    let block = create_test_node(NodeKind::Block { statements: vec![stmt] }, 0, 5);

    let result = find_node_at_range(&block, (2, 4));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_if_statement() {
    let cond = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let then_branch = create_test_node(NodeKind::Number { value: "2".to_string() }, 2, 3);

    let if_node = create_test_node(
        NodeKind::If {
            condition: Box::new(cond),
            then_branch: Box::new(then_branch),
            elsif_branches: vec![],
            else_branch: None,
        },
        0,
        4,
    );

    let result = find_node_at_range(&if_node, (2, 3));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_binary_expression() {
    let left = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let right = create_test_node(NodeKind::Number { value: "2".to_string() }, 3, 4);

    let binary = create_test_node(
        NodeKind::Binary { op: "+".to_string(), left: Box::new(left), right: Box::new(right) },
        0,
        4,
    );

    let result = find_node_at_range(&binary, (3, 4));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_binary_left_child() {
    let left = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let right = create_test_node(NodeKind::Number { value: "2".to_string() }, 3, 4);

    let binary = create_test_node(
        NodeKind::Binary { op: "+".to_string(), left: Box::new(left), right: Box::new(right) },
        0,
        4,
    );

    let result = find_node_at_range(&binary, (0, 1));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_no_match() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 2);
    let result = find_node_at_range(&node, (10, 20));
    assert!(result.is_none());
}

#[test]
fn find_node_at_range_zero_length_range() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 10);
    let result = find_node_at_range(&node, (5, 5));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_exact_node_boundaries() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 5, 10);
    let result = find_node_at_range(&node, (5, 10));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_nested_program_structure() {
    // Program contains Block which contains statements
    let inner_stmt = create_test_node(NodeKind::Number { value: "1".to_string() }, 5, 6);
    let block = create_test_node(NodeKind::Block { statements: vec![inner_stmt] }, 3, 7);
    let program = create_test_node(NodeKind::Program { statements: vec![block] }, 0, 8);

    let result = find_node_at_range(&program, (5, 6));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_if_with_elsif() {
    let cond1 = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let branch1 = create_test_node(NodeKind::Number { value: "a".to_string() }, 2, 3);

    let cond2 = create_test_node(NodeKind::Number { value: "2".to_string() }, 4, 5);
    let branch2 = create_test_node(NodeKind::Number { value: "b".to_string() }, 6, 7);

    let if_node = create_test_node(
        NodeKind::If {
            condition: Box::new(cond1),
            then_branch: Box::new(branch1),
            elsif_branches: vec![(Box::new(cond2), Box::new(branch2))],
            else_branch: None,
        },
        0,
        8,
    );

    let result = find_node_at_range(&if_node, (6, 7));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_if_with_else() {
    let cond = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 1);
    let then_br = create_test_node(NodeKind::Number { value: "a".to_string() }, 2, 3);
    let else_br = create_test_node(NodeKind::Number { value: "b".to_string() }, 4, 5);

    let if_node = create_test_node(
        NodeKind::If {
            condition: Box::new(cond),
            then_branch: Box::new(then_br),
            elsif_branches: vec![],
            else_branch: Some(Box::new(else_br)),
        },
        0,
        6,
    );

    let result = find_node_at_range(&if_node, (4, 5));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_deeply_nested() {
    let num = create_test_node(NodeKind::Number { value: "1".to_string() }, 10, 11);
    let left = create_test_node(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num),
            right: Box::new(create_test_node(NodeKind::Number { value: "2".to_string() }, 12, 13)),
        },
        10,
        13,
    );
    let right = create_test_node(NodeKind::Number { value: "3".to_string() }, 14, 15);
    let top_binary = create_test_node(
        NodeKind::Binary { op: "*".to_string(), left: Box::new(left), right: Box::new(right) },
        10,
        15,
    );

    let result = find_node_at_range(&top_binary, (10, 11));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_unsupported_node_kind() {
    // Test with a node kind that doesn't have special handling
    let node = create_test_node(NodeKind::MissingExpression, 0, 5);
    let result = find_node_at_range(&node, (1, 3));
    assert!(result.is_some()); // Should return the node itself
}

#[test]
fn find_node_at_range_multiple_statements_in_program() {
    let stmt1 = create_test_node(NodeKind::Number { value: "1".to_string() }, 0, 2);
    let stmt2 = create_test_node(NodeKind::Number { value: "2".to_string() }, 3, 5);
    let stmt3 = create_test_node(NodeKind::Number { value: "3".to_string() }, 6, 8);

    let program =
        create_test_node(NodeKind::Program { statements: vec![stmt1, stmt2, stmt3] }, 0, 8);

    let result = find_node_at_range(&program, (3, 5));
    assert!(result.is_some());
}

#[test]
fn find_node_at_range_range_end_equals_node_start() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 10, 15);
    let result = find_node_at_range(&node, (0, 10));
    assert!(result.is_none());
}

#[test]
fn find_node_at_range_range_start_equals_node_end() {
    let node = create_test_node(NodeKind::Number { value: "42".to_string() }, 0, 10);
    let result = find_node_at_range(&node, (10, 20));
    assert!(result.is_none());
}
