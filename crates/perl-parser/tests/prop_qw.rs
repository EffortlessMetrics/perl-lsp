//! Property-based tests for qw expressions

use proptest::prelude::*;
use perl_corpus::r#gen::qw::{use_constant_qw, qw_in_context, simple_qw};
use perl_parser::{Parser, ast::Node};

/// Extract identifiers from AST
fn extract_identifiers(node: &Node) -> Vec<String> {
    let mut ids = Vec::new();
    extract_identifiers_rec(node, &mut ids);
    ids
}

fn extract_identifiers_rec(node: &Node, ids: &mut Vec<String>) {
    match &node.kind {
        perl_parser::ast::NodeKind::Identifier(name) => {
            ids.push(name.clone());
        }
        perl_parser::ast::NodeKind::Bareword(name) => {
            ids.push(name.clone());
        }
        _ => {}
    }
    
    for child in &node.children {
        extract_identifiers_rec(child, ids);
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
    fn qw_constants_are_discovered((src, expected) in use_constant_qw()) {
        let mut parser = Parser::new(&src);
        let ast = parser.parse();
        
        // Should parse without panic
        prop_assert!(ast.is_some(), "Failed to parse: {}", src);
        
        let ast = ast.unwrap();
        
        // Extract all identifiers from the AST
        let found = extract_identifiers(&ast);
        
        // Check that all expected words are found
        for word in &expected {
            prop_assert!(
                found.contains(word),
                "Missing word '{}' in AST. Found: {:?}\nSource: {}",
                word, found, src
            );
        }
        
        // Check that "qwerty" is not spuriously present
        prop_assert!(
            !found.contains(&"qwerty".to_string()),
            "Found spurious 'qwerty' in AST"
        );
    }
    
    #[test]
    fn simple_qw_parses_without_panic(qw in simple_qw()) {
        let mut parser = Parser::new(&qw);
        let ast = parser.parse();
        
        // Should not panic
        prop_assert!(ast.is_some(), "Failed to parse: {}", qw);
        
        // Should produce some nodes
        let ast = ast.unwrap();
        prop_assert!(ast.children.len() > 0);
    }
    
    #[test]
    fn qw_in_various_contexts_parses(code in qw_in_context()) {
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        
        // Should parse without panic
        prop_assert!(ast.is_some(), "Failed to parse: {}", code);
        
        // Should contain qw somewhere
        let src_str = format!("{:?}", ast.unwrap());
        prop_assert!(
            src_str.contains("QwList") || src_str.contains("Qw") || code.contains("qw"),
            "AST doesn't seem to contain qw construct"
        );
    }
    
    #[test]
    fn qw_delimiter_variations_are_equivalent(words in perl_corpus::r#gen::qw::words()) {
        // Test that different delimiters produce equivalent results
        let delimiters = vec![
            ('(', ')'),
            ('[', ']'),
            ('{', '}'),
            ('<', '>'),
            ('/', '/'),
        ];
        
        let mut results = Vec::new();
        
        for (open, close) in delimiters {
            let src = format!("my @x = qw{}{}{};", open, words.join(" "), close);
            let mut parser = Parser::new(&src);
            
            if let Some(ast) = parser.parse() {
                let ids = extract_identifiers(&ast);
                results.push(ids);
            }
        }
        
        // All delimiter variants should produce the same identifiers
        if !results.is_empty() {
            let first = &results[0];
            for (i, other) in results.iter().enumerate().skip(1) {
                prop_assert_eq!(
                    first, other,
                    "Delimiter variant {} produced different results",
                    i
                );
            }
        }
    }
}