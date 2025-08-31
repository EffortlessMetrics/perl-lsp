// Test to verify the Unicode fix works correctly

use perl_lexer::PerlLexer;

#[test]
fn test_unicode_heredoc_fixes() {
    let test_cases = vec![
        "¡<<'",        // The specific failing case - should not panic
        "<<'END'",     // Valid heredoc for comparison
        "¡test",       // Unicode identifier
        "¡ << 'test'", // Unicode with spacing
    ];

    for input in test_cases {
        println!("Testing input: {:?}", input);
        let mut lexer = PerlLexer::new(input);

        // This should not panic - the main test
        let mut token_count = 0;
        while let Some(token) = lexer.next_token() {
            println!("  Token: {:?} '{}'", token.token_type, token.text);
            token_count += 1;

            // Safety valve to prevent infinite loops in testing
            if token_count > 10 {
                break;
            }
        }
        println!("  Success! Processed {} tokens without panic", token_count);
    }
}

#[test]
fn test_unicode_regression_case() {
    // The specific failing case from the proptest regression
    let input = "¡<<'";

    let mut lexer = PerlLexer::new(input);

    // This should not panic anymore
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lexer.next_token()));

    assert!(result.is_ok(), "Lexer should not panic on Unicode input: {:?}", input);
}
