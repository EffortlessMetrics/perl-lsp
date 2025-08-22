//! Metamorphic property tests for whitespace and comment insertion

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};
use perl_parser::Parser;
use perl_lexer::{PerlLexer, TokenType};

// Pull in the shared helpers
include!("prop_test_utils.rs");

const REGRESS_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/_proptest-regressions/prop_whitespace"
);

// Re-lex and keep only "semantic" tokens
fn lex_core_tokens(src: &str) -> Vec<(TokenType, String)> {
    let mut lx = PerlLexer::new(src);
    let mut out = Vec::new();
    let mut count = 0;
    while let Some(t) = lx.next_token() {
        // Prevent infinite loops
        count += 1;
        if count > 10000 {
            break;
        }
        match t.token_type {
            TokenType::Whitespace | TokenType::Newline | TokenType::Comment(_) | TokenType::EOF => {}
            // Skip big bodies that carry their own newlines:
            TokenType::HeredocBody(_) | TokenType::FormatBody(_) => {}
            _ => out.push((t.token_type.clone(), t.text.to_string())),
        }
    }
    out
}

// Check if the boundary between two tokens is breakable (safe to insert whitespace)
fn is_breakable_boundary(left: &(TokenType, String), right: &(TokenType, String)) -> bool {
    // Join the two tokens and re-lex
    let joined = format!("{}{}", left.1, right.1);
    let re = lex_core_tokens(&joined);
    
    // The boundary is breakable if we get exactly the same two tokens back
    re.len() == 2
        && re[0].0 == left.0 && re[0].1 == left.1
        && re[1].0 == right.0 && re[1].1 == right.1
}

// Join original tokens with injected whitespace only at breakable boundaries
fn respace_by_tokens_boundary_aware(original: &str, ws: &str) -> String {
    let toks = lex_core_tokens(original);
    if toks.is_empty() { 
        return original.to_string(); 
    }
    
    let mut out = String::new();
    for i in 0..toks.len() {
        if i > 0 && is_breakable_boundary(&toks[i-1], &toks[i]) {
            out.push_str(ws);
        }
        out.push_str(&toks[i].1);
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64),
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(REGRESS_DIR.into()))),
        .. ProptestConfig::default()
    })]

    #[test]
    fn whitespace_insertion_preserves_tokens(
        src in "[a-zA-Z0-9_$@%&*(){}\\[\\];:,.<>!?+\\-=/ \t\n]{0,200}",
        ws in "[ \t\n]{0,3}" // Can now safely allow 0 whitespace
    ) {
        // Original non-space/comment tokens
        let base = lex_core_tokens(&src);

        // Skip heredoc/format cases to avoid complications
        prop_assume!(!base.iter().any(|(k,_)| matches!(
            k, TokenType::HeredocStart | TokenType::FormatBody(_)
        )));
        
        // Skip if there are adjacent tokens that would merge when whitespace is removed
        // This happens when tokens are only separated by whitespace but would form a single
        // token if joined (like "a" + "_" becoming "a_")
        if base.len() >= 2 {
            for i in 0..base.len()-1 {
                if !is_breakable_boundary(&base[i], &base[i+1]) {
                    // This pair would merge if whitespace is removed, skip this test case
                    return Ok(());
                }
            }
        }

        // Insert whitespace only at breakable boundaries
        let sprinkled = respace_by_tokens_boundary_aware(&src, &ws);
        let again = lex_core_tokens(&sprinkled);

        prop_assert_eq!(&base, &again,
            "tokenization changed:\nSRC:      {}\nSPRINKLED: {}\nbase:  {:?}\nagain: {:?}",
            src, sprinkled, base, again);
    }

    #[test]
    fn simple_code_whitespace_insertion_preserves_shape(
        ws in "[ \t\n]{0,3}"  // Can now safely allow 0 whitespace
    ) {
        let originals = vec![
            "my $x = 1;",
            "sub foo { return 42; }",
            "for (1..10) { print; }",
            "if ($x) { $y++; } else { $z--; }",
            "$x + $y * $z",
            "print 'hello', 'world';",
        ];
        
        for original in originals {
            // Parse original
            let mut parser1 = Parser::new(original);
            let ast1 = parser1.parse();
            prop_assume!(ast1.is_ok());
            
            // Insert whitespace only at breakable boundaries
            let transformed = respace_by_tokens_boundary_aware(original, &ws);
            
            // Parse transformed
            let mut parser2 = Parser::new(&transformed);
            let ast2 = parser2.parse();
            
            prop_assert!(ast2.is_ok(), 
                "Failed to parse after whitespace insertion:\nOriginal: {}\nTransformed: {}",
                original, transformed);
            
            // Compare shapes
            let shape1 = extract_ast_shape(&ast1.unwrap());
            let shape2 = extract_ast_shape(&ast2.unwrap());
            
            prop_assert_eq!(shape1, shape2,
                "Different AST shape after whitespace insertion.\nOriginal: {}\nTransformed: {}",
                original, transformed);
        }
    }

    #[test]
    fn glue_tokens_preserved(
        ws in "[ \t\n]{0,3}"  // Can now safely allow 0 whitespace
    ) {
        // Test that glue tokens like ->, ::, .., ... don't get split
        let glue_samples = vec![
            "$obj->method",
            "Package::Module",
            "1..10",
            "1...10",
            "$x => $y",
            "$x // $y",
            "$x << 2",
            "$x >> 2",
            "$x && $y",
            "$x || $y",
        ];
        
        for original in glue_samples {
            let core_before = lex_core_tokens(original);
            let transformed = respace_by_tokens_boundary_aware(original, &ws);
            let core_after = lex_core_tokens(&transformed);
            
            prop_assert_eq!(core_before, core_after,
                "Glue tokens changed:\nOriginal: {}\nTransformed: {}",
                original, transformed);
        }
    }
}