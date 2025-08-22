//! Metamorphic property tests for whitespace and comment insertion

use perl_parser::Parser;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};

// Pull in the shared helpers (includes CoreTok, TokenType, and whitespace functions)
include!("prop_test_utils.rs");

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_whitespace");

// The whitespace manipulation functions are now in prop_test_utils.rs:
// - CoreTok struct
// - lex_core_spans()
// - pair_breakable()
// - insertion_safe()
// - respace_preserving()

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64),
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(REGRESS_DIR))),
        .. ProptestConfig::default()
    })]

    #[test]
    fn whitespace_insertion_preserves_tokens(
        src in "[a-zA-Z0-9_$@%&*(){}\\[\\];:,.<>!?+\\-=/ \t\n]{0,200}",
        ws in "[ \t\n]{0,3}" // Can now safely allow 0 whitespace
    ) {
        // Original non-space/comment tokens
        let base = lex_core_spans(&src);

        // Skip heredoc/format cases to avoid complications
        // Heredocs are inherently stateful and don't fit our token-based model
        prop_assume!(!base.iter().any(|t| matches!(
            t.kind, TokenType::HeredocStart | TokenType::HeredocBody(_) | TokenType::FormatBody(_)
        )));

        // Insert whitespace only at safe boundaries, preserving required boundaries
        let sprinkled = respace_preserving(&src, &ws);
        let again = lex_core_spans(&sprinkled);

        // Compare normalized pairs (kind, text) only
        let base_pairs: Vec<_> = base.iter().map(|t| (t.kind.clone(), t.text.clone())).collect();
        let again_pairs: Vec<_> = again.iter().map(|t| (t.kind.clone(), t.text.clone())).collect();

        prop_assert_eq!(&base_pairs, &again_pairs,
            "tokenization changed:\nSRC: {}\nSPRINKLED: {}\nbase: {:?}\nagain: {:?}",
            src, sprinkled, base_pairs, again_pairs);
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

            // Insert whitespace only at safe boundaries, preserving required boundaries
            let transformed = respace_preserving(original, &ws);

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
            let core_before = lex_core_spans(original);
            let transformed = respace_preserving(original, &ws);
            let core_after = lex_core_spans(&transformed);

            // Compare normalized pairs (kind, text) only
            let before_pairs: Vec<_> = core_before.iter().map(|t| (t.kind.clone(), t.text.clone())).collect();
            let after_pairs: Vec<_> = core_after.iter().map(|t| (t.kind.clone(), t.text.clone())).collect();

            prop_assert_eq!(&before_pairs, &after_pairs,
                "Glue tokens changed:\nOriginal: {}\nTransformed: {}",
                original, transformed);
        }
    }
}
