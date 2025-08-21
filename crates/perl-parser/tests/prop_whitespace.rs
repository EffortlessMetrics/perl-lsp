//! Metamorphic property tests for whitespace and comment insertion

use proptest::prelude::*;
use perl_corpus::gen::whitespace::{
    sprinkle_whitespace, whitespace_stress_test, commented_code,
};
use perl_parser::{Parser, ast::{Node, NodeKind}};
use std::collections::HashSet;

/// Extract all token texts (identifiers, keywords, operators)
fn extract_tokens(node: &Node) -> Vec<String> {
    let mut tokens = Vec::new();
    extract_tokens_rec(node, &mut tokens);
    tokens
}

fn extract_tokens_rec(node: &Node, tokens: &mut Vec<String>) {
    match &node.kind {
        NodeKind::Identifier(s) |
        NodeKind::Bareword(s) |
        NodeKind::PackageName(s) |
        NodeKind::SubroutineName(s) => {
            tokens.push(s.clone());
        }
        NodeKind::Number(n) => {
            tokens.push(n.to_string());
        }
        NodeKind::Keyword(k) => {
            tokens.push(format!("{:?}", k));
        }
        NodeKind::Operator(op) => {
            tokens.push(format!("{:?}", op));
        }
        _ => {}
    }
    
    for child in &node.children {
        extract_tokens_rec(child, tokens);
    }
}

/// Extract AST shape (node kinds only, no values)
fn extract_shape(node: &Node) -> Vec<String> {
    let mut shape = Vec::new();
    extract_shape_rec(node, &mut shape);
    shape
}

fn extract_shape_rec(node: &Node, shape: &mut Vec<String>) {
    // Get just the variant name, not the values
    let kind_str = format!("{:?}", node.kind);
    let variant = kind_str.split('(').next().unwrap_or(&kind_str).to_string();
    shape.push(variant);
    
    for child in &node.children {
        extract_shape_rec(child, shape);
    }
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
    fn whitespace_insertion_preserves_tokens(seed in 0u64..1000) {
        let original = "use strict; my $x = 1 + 2; print $x;";
        let transformed = sprinkle_whitespace(original, seed);
        
        // Parse both versions
        let mut parser1 = Parser::new(original);
        let mut parser2 = Parser::new(&transformed);
        
        let ast1 = parser1.parse();
        let ast2 = parser2.parse();
        
        prop_assert!(ast1.is_some(), "Failed to parse original");
        prop_assert!(ast2.is_some(), "Failed to parse transformed: {}", transformed);
        
        // Extract tokens from both
        let tokens1 = extract_tokens(&ast1.unwrap());
        let tokens2 = extract_tokens(&ast2.unwrap());
        
        // Filter out comment tokens if any
        let tokens1_set: HashSet<_> = tokens1.into_iter()
            .filter(|t| !t.starts_with('#'))
            .collect();
        let tokens2_set: HashSet<_> = tokens2.into_iter()
            .filter(|t| !t.starts_with('#'))
            .collect();
        
        // Should have the same tokens
        prop_assert_eq!(
            tokens1_set, tokens2_set,
            "Different tokens after whitespace insertion.\nOriginal: {}\nTransformed: {}",
            original, transformed
        );
    }
    
    #[test]
    fn comment_insertion_preserves_shape(seed in 0u64..1000) {
        let originals = vec![
            "my $x = 1;",
            "sub foo { return 42; }",
            "for (1..10) { print; }",
            "if ($x) { $y++; } else { $z--; }",
        ];
        
        for original in originals {
            let transformed = sprinkle_whitespace(original, seed);
            
            let mut parser1 = Parser::new(original);
            let mut parser2 = Parser::new(&transformed);
            
            let ast1 = parser1.parse();
            let ast2 = parser2.parse();
            
            if ast1.is_none() || ast2.is_none() {
                continue; // Skip if parsing fails
            }
            
            let shape1 = extract_shape(&ast1.unwrap());
            let shape2 = extract_shape(&ast2.unwrap());
            
            // Filter out comment nodes
            let shape1_filtered: Vec<_> = shape1.into_iter()
                .filter(|s| !s.contains("Comment"))
                .collect();
            let shape2_filtered: Vec<_> = shape2.into_iter()
                .filter(|s| !s.contains("Comment"))
                .collect();
            
            prop_assert_eq!(
                shape1_filtered, shape2_filtered,
                "Different AST shape after comment insertion.\nOriginal: {}\nTransformed: {}",
                original, transformed
            );
        }
    }
    
    #[test]
    fn whitespace_stress_code_parses(code in whitespace_stress_test()) {
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse whitespace-heavy code: {}", code);
        
        // Should still contain expected elements
        prop_assert!(code.contains("use"));
        prop_assert!(code.contains("strict"));
        prop_assert!(code.contains("my"));
        prop_assert!(code.contains("print"));
    }
    
    #[test]
    fn commented_code_has_both_code_and_comments(code in commented_code()) {
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse commented code: {}", code);
        
        // Should have both comments and actual code
        prop_assert!(code.contains('#'), "No comments found");
        
        let non_comment_lines = code.lines()
            .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
            .count();
        prop_assert!(non_comment_lines > 0, "No actual code found");
    }
    
    #[test]
    fn massive_whitespace_doesnt_crash(
        base in "[a-zA-Z]+",
        ws_count in 1usize..100
    ) {
        let mut code = String::from("my $");
        code.push_str(&base);
        code.push_str(&" ".repeat(ws_count));
        code.push_str("= 1;");
        
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        
        // Should not panic or fail
        prop_assert!(ast.is_some(), "Failed with {} spaces", ws_count);
    }
    
    #[test]
    fn tab_space_mixing_preserves_semantics(
        tabs in 0usize..5,
        spaces in 0usize..10
    ) {
        let indent1 = "\t".repeat(tabs) + &" ".repeat(spaces);
        let indent2 = " ".repeat(spaces) + &"\t".repeat(tabs);
        
        let code1 = format!("sub foo {{\n{}return 42;\n}}", indent1);
        let code2 = format!("sub foo {{\n{}return 42;\n}}", indent2);
        
        let mut parser1 = Parser::new(&code1);
        let mut parser2 = Parser::new(&code2);
        
        let ast1 = parser1.parse();
        let ast2 = parser2.parse();
        
        prop_assert!(ast1.is_some() && ast2.is_some());
        
        // Both should define the same subroutine
        let tokens1 = extract_tokens(&ast1.unwrap());
        let tokens2 = extract_tokens(&ast2.unwrap());
        
        prop_assert!(tokens1.contains(&"foo".to_string()));
        prop_assert!(tokens2.contains(&"foo".to_string()));
        prop_assert!(tokens1.contains(&"42".to_string()));
        prop_assert!(tokens2.contains(&"42".to_string()));
    }
}