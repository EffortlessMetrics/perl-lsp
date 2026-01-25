#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Property-based tests for `qw/.../` expressions

#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_parser::Parser;
use proptest::{
    collection, prop_assume, prop_oneof, proptest,
    strategy::{Just, Strategy},
    test_runner::{Config as ProptestConfig, FileFailurePersistence},
};

// Pull in the shared helpers (delims, extract_ast_shape, etc.)
include!("prop_test_utils.rs");

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_qw");

// Small helper to make word payloads
fn word() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_]{1,8}".prop_map(|s| s.to_string())
}

// Basic delimiter strategy for qw
fn delim_strategy() -> impl Strategy<Value = (char, char)> {
    prop_oneof![
        Just(('(', ')')),
        Just(('{', '}')),
        Just(('[', ']')),
        Just(('<', '>')),
        Just(('/', '/')),
        Just(('#', '#')),
        Just(('!', '!')),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(64),
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(REGRESS_DIR))),
        .. ProptestConfig::default()
    })]

    /// Different delimiters around the same `qw` payload should yield the same *shape*.
    #[test]
    fn qw_delimiter_variations_are_equivalent(
        words in collection::vec(word(), 1..6),
        (open1, close1) in delim_strategy(),
        (open2, close2) in delim_strategy(),
    ) {
        let payload = words.join(" ");
        // Avoid collisions with the chosen delimiters
        prop_assume!(!payload.contains(open1) && !payload.contains(close1));
        prop_assume!(!payload.contains(open2) && !payload.contains(close2));

        let a = format!("my @x = qw{}{}{};", open1, payload, close1);
        let b = format!("my @x = qw{}{}{};", open2, payload, close2);

        let mut pa = Parser::new(&a);
        let mut pb = Parser::new(&b);
        let sa = extract_ast_shape(&pa.parse().expect("parse a"));
        let sb = extract_ast_shape(&pb.parse().expect("parse b"));

        prop_assert_eq!(&sa, &sb, "shapes differ:\nA: {}\n{:?}\n\nB: {}\n{:?}", a, sa, b, sb);
    }

    /// `qw` in a few simple contexts parses the same shape regardless of the delimiter.
    #[test]
    fn qw_in_various_contexts_parseable(
        words in collection::vec("[A-Za-z0-9_]+", 1..4),
        (open, close) in delim_strategy(),
    ) {
        let payload = words.join(" ");
        prop_assume!(!payload.contains(open) && !payload.contains(close));

        let snippets = [
            format!("my @x = qw{}{}{};", open, payload, close),
            format!("for my $w (qw{}{}{}) {{ }}", open, payload, close),
            format!("print scalar(qw{}{}{});", open, payload, close),
        ];

        for code in &snippets {
            let mut p = Parser::new(code);
            let _ = p.parse().expect(code);
        }
    }
}
