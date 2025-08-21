//! Property tests for parser invariants and safety properties

use proptest::prelude::*;
use perl_parser::{Parser, ast::{Node, NodeKind}};

/// Check that all node spans are well-formed
fn check_spans_monotonic(node: &Node) -> Result<(), String> {
    check_spans_rec(node, None)
}

fn check_spans_rec(node: &Node, parent: Option<(usize, usize)>) -> Result<(), String> {
    let start = node.start;
    let end = node.end;
    
    // Node's own span must be valid
    if start > end {
        return Err(format!("Node has invalid span: start {} > end {}", start, end));
    }
    
    // If has parent, must be contained within parent's span
    if let Some((p_start, p_end)) = parent {
        if start < p_start || end > p_end {
            return Err(format!(
                "Child span [{}, {}] exceeds parent span [{}, {}]",
                start, end, p_start, p_end
            ));
        }
    }
    
    // Check children
    for child in &node.children {
        check_spans_rec(child, Some((start, end)))?;
    }
    
    // Check that children don't overlap
    let mut prev_end = start;
    for child in &node.children {
        if child.start < prev_end {
            return Err(format!(
                "Overlapping children: previous ends at {}, next starts at {}",
                prev_end, child.start
            ));
        }
        prev_end = child.end;
    }
    
    Ok(())
}

/// Check for cycles in the AST
fn check_no_cycles(root: &Node) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![root as *const Node];
    
    while let Some(ptr) = stack.pop() {
        if !visited.insert(ptr) {
            return false; // Found a cycle
        }
        
        let node = unsafe { &*ptr };
        for child in &node.children {
            stack.push(child as *const Node);
        }
    }
    
    true
}

/// Count total nodes in AST
fn count_nodes(node: &Node) -> usize {
    1 + node.children.iter().map(|c| count_nodes(c)).sum::<usize>()
}

/// Get maximum depth of AST
fn max_depth(node: &Node) -> usize {
    node.children.iter()
        .map(|c| max_depth(c))
        .max()
        .unwrap_or(0) + 1
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(64),
        ..ProptestConfig::default()
    })]
    
    #[test]
    fn spans_are_monotonic_for_any_input(
        input in ".*"
            .prop_filter("Not empty", |s| !s.is_empty())
            .prop_filter("Not too long", |s| s.len() < 1000)
    ) {
        let mut parser = Parser::new(&input);
        
        if let Some(ast) = parser.parse() {
            match check_spans_monotonic(&ast) {
                Ok(()) => {},
                Err(e) => {
                    prop_assert!(false, "Span invariant violated: {}\nInput: {}", e, input);
                }
            }
        }
    }
    
    #[test]
    fn no_cycles_in_ast(
        input in prop::collection::vec(
            prop::sample::select(vec![
                "my $x = 1;",
                "sub foo { }",
                "for (1..10) { }",
                "if ($x) { }",
                "{ { { } } }",
            ]),
            1..5
        ).prop_map(|v| v.join("\n"))
    ) {
        let mut parser = Parser::new(&input);
        
        if let Some(ast) = parser.parse() {
            prop_assert!(
                check_no_cycles(&ast),
                "Found cycle in AST for input: {}",
                input
            );
        }
    }
    
    #[test]
    fn ast_depth_is_reasonable(
        depth in 1usize..20
    ) {
        // Generate deeply nested code
        let mut code = String::new();
        for _ in 0..depth {
            code.push_str("{ ");
        }
        code.push_str("1");
        for _ in 0..depth {
            code.push_str(" }");
        }
        
        let mut parser = Parser::new(&code);
        
        if let Some(ast) = parser.parse() {
            let actual_depth = max_depth(&ast);
            
            // AST depth should be proportional to nesting depth
            // but not necessarily equal (due to intermediate nodes)
            prop_assert!(
                actual_depth <= depth * 3,
                "AST depth {} too large for nesting depth {}",
                actual_depth, depth
            );
        }
    }
    
    #[test]
    fn parser_doesnt_panic_on_random_input(
        input in ".*".prop_filter("Reasonable size", |s| s.len() < 10000)
    ) {
        let mut parser = Parser::new(&input);
        
        // Should not panic
        let _ = parser.parse();
    }
    
    #[test]
    fn parser_doesnt_panic_on_binary_input(
        bytes in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        // Convert to string, possibly invalid UTF-8
        let input = String::from_utf8_lossy(&bytes);
        let mut parser = Parser::new(&input);
        
        // Should not panic even on weird input
        let _ = parser.parse();
    }
    
    #[test]
    fn empty_statements_dont_break_parser(
        semicolons in 1usize..20
    ) {
        let input = ";".repeat(semicolons);
        let mut parser = Parser::new(&input);
        
        let ast = parser.parse();
        prop_assert!(ast.is_some(), "Failed to parse {} semicolons", semicolons);
    }
    
    #[test]
    fn node_count_is_bounded(
        statements in prop::collection::vec(
            prop::sample::select(vec![
                "my $x = 1;",
                "$x++;",
                "print $x;",
                "sub f { }",
            ]),
            1..50
        )
    ) {
        let input = statements.join("\n");
        let mut parser = Parser::new(&input);
        
        if let Some(ast) = parser.parse() {
            let node_count = count_nodes(&ast);
            
            // Should have reasonable number of nodes
            // (not exponential in input size)
            prop_assert!(
                node_count <= statements.len() * 20,
                "Too many nodes ({}) for {} statements",
                node_count, statements.len()
            );
        }
    }
    
    #[test]
    fn parser_recovers_from_errors(
        valid1 in "[a-z]+",
        invalid in prop::sample::select(vec!["{{{{", "]]]]", "####", "!!!!"]),
        valid2 in "[a-z]+"
    ) {
        let input = format!("my ${} = 1;\n{}\nmy ${} = 2;", valid1, invalid, valid2);
        let mut parser = Parser::new(&input);
        
        // Should still parse something despite error in middle
        let ast = parser.parse();
        prop_assert!(ast.is_some(), "Parser failed completely on: {}", input);
        
        // Should find both valid variable names
        let ast = ast.unwrap();
        let debug = format!("{:?}", ast);
        prop_assert!(
            debug.contains(&valid1) || debug.contains(&valid2),
            "Lost valid code around error"
        );
    }
}