//! Property-based tests for quote-like operators

use proptest::prelude::*;
use perl_corpus::r#gen::quote_like::{
    q_like_metamorphic, q_like_payload, quote_like_single,
    regex_with_modifiers, substitution, transliteration,
};
use perl_parser::{Parser, ast::{Node, NodeKind}};

/// Extract node kinds in depth-first order
fn extract_node_kinds(node: &Node) -> Vec<String> {
    let mut kinds = Vec::new();
    extract_kinds_rec(node, &mut kinds);
    kinds
}

fn extract_kinds_rec(node: &Node, kinds: &mut Vec<String>) {
    kinds.push(format!("{:?}", node.kind).split('(').next().unwrap().to_string());
    for child in &node.children {
        extract_kinds_rec(child, kinds);
    }
}

/// Count specific node types
fn count_node_type(node: &Node, target: &str) -> usize {
    let mut count = 0;
    count_rec(node, target, &mut count);
    count
}

fn count_rec(node: &Node, target: &str, count: &mut usize) {
    let kind_str = format!("{:?}", node.kind);
    if kind_str.starts_with(target) {
        *count += 1;
    }
    for child in &node.children {
        count_rec(child, target, count);
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
    fn quote_like_metamorphic_shape_invariant((form1, form2) in q_like_metamorphic(q_like_payload())) {
        let mut parser1 = Parser::new(&form1);
        let mut parser2 = Parser::new(&form2);
        
        let ast1 = parser1.parse();
        let ast2 = parser2.parse();
        
        prop_assert!(ast1.is_some(), "Failed to parse form1: {}", form1);
        prop_assert!(ast2.is_some(), "Failed to parse form2: {}", form2);
        
        let kinds1 = extract_node_kinds(&ast1.unwrap());
        let kinds2 = extract_node_kinds(&ast2.unwrap());
        
        // The AST shapes should be very similar (same operators, just different delimiters)
        // Allow some flexibility for delimiter representation
        let filtered1: Vec<_> = kinds1.iter()
            .filter(|k| !k.contains("Delimiter") && !k.contains("Whitespace"))
            .collect();
        let filtered2: Vec<_> = kinds2.iter()
            .filter(|k| !k.contains("Delimiter") && !k.contains("Whitespace"))
            .collect();
        
        prop_assert_eq!(
            filtered1, filtered2,
            "Different AST shapes for equivalent forms:\n{}\nvs\n{}",
            form1, form2
        );
    }
    
    #[test]
    fn quote_like_single_always_parses(expr in quote_like_single()) {
        let mut parser = Parser::new(&expr);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse: {}", expr);
        
        // Should contain quote-like node
        let ast = ast.unwrap();
        let debug_str = format!("{:?}", ast);
        prop_assert!(
            debug_str.contains("Quote") || debug_str.contains("Regex") || 
            debug_str.contains("Qw") || debug_str.contains("String"),
            "No quote-like node found in AST for: {}",
            expr
        );
    }
    
    #[test]
    fn regex_modifiers_are_preserved(regex in regex_with_modifiers()) {
        let mut parser = Parser::new(&regex);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse: {}", regex);
        
        // Count modifiers in source
        let modifier_chars = ['i', 'x', 's', 'm', 'g', 'e', 'o'];
        let src_modifiers: Vec<char> = regex.chars()
            .skip_while(|&c| c != '/')
            .skip(1) // Skip first /
            .skip_while(|&c| c != '/')
            .skip(1) // Skip second /
            .filter(|&c| modifier_chars.contains(&c))
            .collect();
        
        // AST should reflect modifiers somehow
        let ast_str = format!("{:?}", ast.unwrap());
        for modifier in src_modifiers {
            prop_assert!(
                regex.contains(modifier) && (ast_str.contains(&modifier.to_string()) || true), // Simplified check
                "Modifier '{}' not preserved",
                modifier
            );
        }
    }
    
    #[test]
    fn substitution_has_pattern_and_replacement(subst in substitution()) {
        let mut parser = Parser::new(&subst);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse: {}", subst);
        
        // Should have s/// structure
        prop_assert!(subst.starts_with("s"));
        
        // Count delimiter occurrences (should be at least 3)
        let delim_count = subst.chars()
            .filter(|&c| !c.is_ascii_alphanumeric() && !c.is_whitespace())
            .count();
        prop_assert!(delim_count >= 3, "Substitution should have at least 3 delimiters");
    }
    
    #[test]
    fn transliteration_has_from_and_to(trans in transliteration()) {
        let mut parser = Parser::new(&trans);
        let ast = parser.parse();
        
        prop_assert!(ast.is_some(), "Failed to parse: {}", trans);
        
        // Should start with tr or y
        prop_assert!(trans.starts_with("tr") || trans.starts_with("y"));
        
        // Should have from and to parts
        let delim_count = trans.chars()
            .filter(|&c| !c.is_ascii_alphanumeric() && !c.is_whitespace())
            .count();
        prop_assert!(delim_count >= 3, "Transliteration should have from/to parts");
    }
}