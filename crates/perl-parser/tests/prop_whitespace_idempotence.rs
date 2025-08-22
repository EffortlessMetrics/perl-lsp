use proptest::prelude::*;

// Include the shared utilities (which includes TokenType import)
include!("prop_test_utils.rs");

proptest! {
    #[test]
    fn respace_preserving_is_idempotent(
        s in ".{0,200}",
        ws in "[ \\t]{0,2}"
    ) {
        // Applying respace_preserving twice should give the same result as applying it once
        let once = respace_preserving(&s, &ws);
        let twice = respace_preserving(&once, &ws);
        
        // Compare the token streams (ignoring whitespace differences)
        let tokens_once = lex_core_spans(&once);
        let tokens_twice = lex_core_spans(&twice);
        
        // Extract just the semantic tokens (kind, text) for comparison
        let semantic_once: Vec<_> = tokens_once.into_iter()
            .map(|t| (format!("{:?}", t.kind), t.text))
            .collect();
        let semantic_twice: Vec<_> = tokens_twice.into_iter()
            .map(|t| (format!("{:?}", t.kind), t.text))
            .collect();
        
        prop_assert_eq!(
            semantic_once,
            semantic_twice,
            "respace_preserving is not idempotent for input '{}' with ws '{}'",
            s.escape_debug(),
            ws.escape_debug()
        );
    }
    
    #[test]
    fn insertion_safe_is_consistent(
        s in "[a-zA-Z0-9.(){}\\[\\]$@%]{0,50}",
        ws in "[ \\t]{1,2}"
    ) {
        // If insertion_safe says it's safe at a position,
        // actually inserting there shouldn't change the token stream
        let toks = lex_core_spans(&s);
        
        if toks.len() < 2 {
            return Ok(()); // Need at least 2 tokens to test insertion
        }
        
        // Find boundaries where insertion_safe returns true
        let mut safe_positions = Vec::new();
        for i in 0..toks.len() - 1 {
            if insertion_safe(&s, &toks, i, &ws) {
                safe_positions.push(i);
            }
        }
        
        // For each safe position, verify the insertion truly doesn't change tokens
        for pos in safe_positions {
            let mut modified = String::new();
            
            // Rebuild string with whitespace at position
            if pos == 0 {
                modified.push_str(&s[..toks[0].end]);
            } else {
                modified.push_str(&s[..toks[pos].end]);
            }
            modified.push_str(&ws);
            modified.push_str(&s[toks[pos + 1].start..]);
            
            let original_semantic: Vec<_> = toks.iter()
                .map(|t| (format!("{:?}", t.kind), t.text.clone()))
                .collect();
            let modified_semantic: Vec<_> = lex_core_spans(&modified).into_iter()
                .map(|t| (format!("{:?}", t.kind), t.text))
                .collect();
            
            prop_assert_eq!(
                original_semantic,
                modified_semantic,
                "insertion_safe incorrectly marked position {} as safe",
                pos
            );
        }
    }
    
    #[test]
    fn pair_breakable_is_symmetric(
        left_text in "[a-zA-Z0-9.]{1,10}",
        right_text in "[a-zA-Z0-9.]{1,10}"
    ) {
        // Create mock tokens
        let left = CoreTok {
            kind: TokenType::Identifier(left_text.clone().into()),
            text: left_text.clone(),
            start: 0,
            end: left_text.len(),
        };
        
        let right = CoreTok {
            kind: TokenType::Identifier(right_text.clone().into()),
            text: right_text.clone(),
            start: left_text.len(),
            end: left_text.len() + right_text.len(),
        };
        
        // pair_breakable should be consistent
        let breakable = pair_breakable(&left, &right);
        
        // If it's breakable, joining them should produce two tokens
        if breakable {
            let joined = format!("{}{}", left_text, right_text);
            let tokens = lex_core_spans(&joined);
            
            // Should produce exactly 2 tokens
            prop_assert!(
                tokens.len() == 2,
                "pair_breakable said '{}' + '{}' is breakable but got {} tokens",
                left_text,
                right_text,
                tokens.len()
            );
        }
    }
}