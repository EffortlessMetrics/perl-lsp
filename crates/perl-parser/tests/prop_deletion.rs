#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

// Include the shared utilities
include!("prop_test_utils.rs");

/// Delete whitespace between tokens that can be safely joined
pub fn delete_on_breakable(src: &str) -> String {
    let toks = lex_core_spans(src);
    if toks.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut prev_end = 0;

    for (i, tok) in toks.iter().enumerate() {
        // Include any text before this token's start
        if tok.start > prev_end {
            let gap = &src[prev_end..tok.start];

            // Check if we should delete this whitespace
            if i > 0 {
                let prev_tok = &toks[i - 1];
                if pair_breakable(prev_tok, tok) {
                    // These tokens can be safely joined, delete the whitespace
                    // (don't add it)
                } else {
                    // These tokens would merge if joined, keep the whitespace
                    result.push_str(gap);
                }
            } else {
                // Keep leading whitespace
                result.push_str(gap);
            }
        }

        // Add the token itself
        result.push_str(&tok.text);
        prev_end = tok.end;
    }

    // Include any trailing text
    if prev_end < src.len() {
        result.push_str(&src[prev_end..]);
    }

    result
}

proptest! {
    #[test]
    fn delete_preserves_core_tokens(
        s in "[a-zA-Z0-9 \t\n()\\[\\];:,.+=\\-*]+",
    ) {
        // Use simple alphanumeric and basic operators to avoid edge cases

        let before = lex_core_spans(&s);
        let deleted = delete_on_breakable(&s);
        let after = lex_core_spans(&deleted);

        // The core tokens should be the same
        prop_assert_eq!(
            before.len(),
            after.len(),
            "Token count changed: {} -> {} for input {:?} -> {:?}",
            before.len(),
            after.len(),
            s,
            deleted
        );

        for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
            prop_assert_eq!(
                &b.text,
                &a.text,
                "Token {} text changed: {:?} -> {:?}",
                i,
                b.text,
                a.text
            );
            prop_assert_eq!(
                std::mem::discriminant(&b.kind),
                std::mem::discriminant(&a.kind),
                "Token {} type changed: {:?} -> {:?}",
                i,
                b.kind,
                a.kind
            );
        }
    }

    #[test]
    fn delete_then_respace_preserves_tokens(
        s in "[a-zA-Z0-9 \t\n()\\[\\];:,.+=\\-*]+",
        ws in r"[ \t\n]*",
    ) {
        // Use simple alphanumeric and basic operators to avoid edge cases

        // Test that deletion followed by respacing preserves token structure
        // This verifies both operations are token-preserving

        let deleted = delete_on_breakable(&s);
        let respaced = respace_preserving(&deleted, &ws);

        let deleted_toks = lex_core_spans(&deleted);
        let respaced_toks = lex_core_spans(&respaced);

        // Both should have the same tokens
        prop_assert_eq!(
            deleted_toks.len(),
            respaced_toks.len(),
            "Token count changed between deleted and respaced for input {:?} -> deleted {:?} -> respaced {:?}",
            s,
            deleted,
            respaced
        );

        for (i, (d, r)) in deleted_toks.iter().zip(respaced_toks.iter()).enumerate() {
            prop_assert_eq!(
                &d.text,
                &r.text,
                "Token {} text changed between deleted and respaced",
                i
            );
        }
    }
}
